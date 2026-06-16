use std::{cell::Cell, io::Write, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    error::{IntoExn, Result},
    extractor::Extractor,
    git_repo::GitRepo,
    memory::MemoryChange,
};

/// A single message in a conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

/// Handle to an open session. Tracks the commit offset via internal mutability
/// so that operations like commit can take `&self`.
#[derive(Debug)]
pub struct Session {
    pub owner: String,
    pub agent_id: String,
    pub session_id: String,
    pub base_path: PathBuf,
    messages_file: PathBuf,
    offset_file: PathBuf,
    offset: Cell<usize>,
}

impl Session {
    /// Returns the current commit offset (number of lines already committed).
    pub fn offset(&self) -> usize {
        self.offset.get()
    }
}

/// Manages session lifecycle: creation, message appending, offset-based
/// commits. Cheap to clone — the lock is shared.
#[derive(Clone)]
pub(crate) struct SessionManager {
    data_dir: PathBuf,
    lock: Arc<Mutex<()>>,
    extractor: Option<Arc<dyn Extractor>>,
}

/// Result of a commit operation.
#[derive(Debug)]
pub enum CommitResult {
    Committed { messages_processed: usize, changes: Vec<MemoryChange> },
    NoNewMessages,
}

impl CommitResult {
    /// Number of messages processed, or 0 if nothing to commit.
    pub fn messages_processed(&self) -> usize {
        match self {
            CommitResult::Committed { messages_processed, .. } => *messages_processed,
            CommitResult::NoNewMessages => 0,
        }
    }

    /// Memory changes extracted by the LLM (empty in Phase 1).
    pub fn changes(&self) -> &[MemoryChange] {
        match self {
            CommitResult::Committed { changes, .. } => changes,
            CommitResult::NoNewMessages => &[],
        }
    }
}

impl SessionManager {
    pub(crate) fn new(data_dir: PathBuf) -> Self {
        Self { data_dir, lock: Arc::new(Mutex::new(())), extractor: None }
    }

    pub(crate) fn set_extractor(&mut self, ext: Arc<dyn Extractor>) {
        self.extractor = Some(ext);
    }

    /// Open the git repo for a given owner.
    fn repo_for_owner(&self, owner: &str) -> Result<GitRepo> {
        GitRepo::open_or_init(&self.data_dir.join(owner))
    }

    /// Get or create a session directory and return a Session handle.
    pub(crate) fn get_or_create(
        &self,
        owner: &str,
        agent_id: &str,
        session_id: &str,
    ) -> Result<Session> {
        let base_path = self
            .data_dir
            .join(owner)
            .join("agents")
            .join(agent_id)
            .join("sessions")
            .join(session_id);

        std::fs::create_dir_all(&base_path).into_exn()?;

        let messages_file = base_path.join("messages.jsonl");
        let offset_file = base_path.join(".commit_offset");

        if !messages_file.exists() {
            std::fs::File::create(&messages_file).into_exn()?;
        }

        let offset = if offset_file.exists() {
            let content = std::fs::read_to_string(&offset_file).into_exn()?;
            content.trim().parse().unwrap_or(0)
        } else {
            0
        };

        Ok(Session {
            owner: owner.to_string(),
            agent_id: agent_id.to_string(),
            session_id: session_id.to_string(),
            base_path,
            offset: Cell::new(offset),
            messages_file,
            offset_file,
        })
    }

    /// Append a message to the session's messages.jsonl.
    pub(crate) async fn add_message(
        &self,
        session: &Session,
        role: String,
        content: String,
    ) -> Result<()> {
        let msg = Message { role, content, timestamp: chrono::Utc::now().to_rfc3339() };
        let line = serde_json::to_string(&msg).into_exn()? + "\n";

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&session.messages_file)
            .into_exn()?;
        file.write_all(line.as_bytes()).into_exn()?;
        Ok(())
    }

    /// Commit unprocessed messages. Uses `&Session` (not `&mut`) thanks to
    /// `Cell<usize>` for the offset.
    pub(crate) async fn commit_session(&self, session: &Session) -> Result<CommitResult> {
        let _guard = self.lock.lock().await;

        let total = if session.messages_file.exists() {
            let content = std::fs::read_to_string(&session.messages_file).into_exn()?;
            content.lines().count()
        } else {
            0
        };

        let cur_offset = session.offset.get();
        if cur_offset >= total {
            return Ok(CommitResult::NoNewMessages);
        }

        let new_message_count = total - cur_offset;

        // Phase 2: extract memories from new messages
        let changes = if let Some(ext) = &self.extractor {
            let new_msgs = read_messages_range(&session.messages_file, cur_offset)?;
            ext.extract(&new_msgs).await.unwrap_or_default()
        } else {
            vec![]
        };

        session.offset.set(total);
        std::fs::write(&session.offset_file, total.to_string()).into_exn()?;

        let repo = self.repo_for_owner(&session.owner)?;
        repo.add_path(&session.messages_file)?;
        repo.add_path(&session.offset_file)?;
        repo.commit(&format!(
            "commit session {}/{}/{} offset {}",
            session.owner, session.agent_id, session.session_id, total
        ))?;

        Ok(CommitResult::Committed { messages_processed: new_message_count, changes })
    }

    /// Read the full message history for a session.
    pub(crate) fn read_full_history(&self, session: &Session) -> Result<Vec<Message>> {
        if !session.messages_file.exists() {
            return Ok(vec![]);
        }
        let content = std::fs::read_to_string(&session.messages_file).into_exn()?;
        let messages: Vec<Message> =
            content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();
        Ok(messages)
    }
}

/// Read messages from a specific line offset to end of file.
fn read_messages_range(file: &PathBuf, offset: usize) -> Result<Vec<Message>> {
    if !file.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(file).into_exn()?;
    let messages: Vec<Message> =
        content.lines().skip(offset).filter_map(|line| serde_json::from_str(line).ok()).collect();
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("runestone_test_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn unwrap<T>(r: Result<T>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => panic!("{e:?}"),
        }
    }

    #[tokio::test]
    async fn test_create_and_add_message() {
        let dir = test_dir("create_add");
        let mgr = SessionManager::new(dir.clone());

        let session = unwrap(mgr.get_or_create("alice", "mybot", "s1"));
        assert_eq!(session.offset(), 0);

        unwrap(mgr.add_message(&session, "user".into(), "Hello".into()).await);

        let history = unwrap(mgr.read_full_history(&session));
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].role, "user");
        assert_eq!(history[0].content, "Hello");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_commit_and_offset() {
        let dir = test_dir("commit_offset");
        let mgr = SessionManager::new(dir.clone());

        let session = unwrap(mgr.get_or_create("alice", "mybot", "s1"));

        unwrap(mgr.add_message(&session, "user".into(), "msg1".into()).await);
        unwrap(mgr.add_message(&session, "user".into(), "msg2".into()).await);

        let result = unwrap(mgr.commit_session(&session).await);
        match result {
            CommitResult::Committed { messages_processed, .. } => {
                assert_eq!(messages_processed, 2)
            }
            _ => panic!("expected Committed"),
        }
        assert_eq!(session.offset(), 2);

        // No new messages
        let result = unwrap(mgr.commit_session(&session).await);
        match result {
            CommitResult::NoNewMessages => {}
            _ => panic!("expected NoNewMessages"),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}

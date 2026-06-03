use crate::error::{IntoExn, Result};
use crate::git_repo::GitRepo;
use crate::memory::MemoryChange;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A single message in a conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

/// Represents an open session.
#[derive(Debug, Clone)]
pub struct Session {
    pub owner: String,
    pub agent_id: String,
    pub session_id: String,
    pub base_path: PathBuf,
    pub offset: usize,
    messages_file: PathBuf,
    offset_file: PathBuf,
}

/// Manages session lifecycle: creation, message appending, offset-based commits.
pub struct SessionManager {
    data_dir: PathBuf,
    lock: Arc<Mutex<()>>,
}

/// Result of a commit operation.
pub enum CommitResult {
    Committed {
        messages_processed: usize,
        changes: Vec<MemoryChange>,
    },
    NoNewMessages,
}

impl SessionManager {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// Open the git repo for a given owner.
    pub fn repo_for_owner(&self, owner: &str) -> Result<GitRepo> {
        GitRepo::open_or_init(&self.data_dir.join(owner))
    }

    /// Get or create a session directory and return a Session handle.
    pub fn get_or_create(
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
            offset,
            messages_file,
            offset_file,
        })
    }

    /// Append a message to the session's messages.jsonl.
    pub async fn add_message(
        &self,
        session: &Session,
        role: String,
        content: String,
    ) -> Result<()> {
        let msg = Message {
            role,
            content,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let line = serde_json::to_string(&msg).into_exn()? + "\n";

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&session.messages_file)
            .into_exn()?;
        file.write_all(line.as_bytes()).into_exn()?;
        Ok(())
    }

    /// Commit unprocessed messages for a session.
    pub async fn commit_session(&self, session: &mut Session) -> Result<CommitResult> {
        let _guard = self.lock.lock().await;

        let total = if session.messages_file.exists() {
            let content = std::fs::read_to_string(&session.messages_file).into_exn()?;
            content.lines().count()
        } else {
            0
        };

        if session.offset >= total {
            return Ok(CommitResult::NoNewMessages);
        }

        let new_message_count = total - session.offset;
        session.offset = total;
        std::fs::write(&session.offset_file, total.to_string()).into_exn()?;

        let repo = self.repo_for_owner(&session.owner)?;
        repo.add_path(&session.messages_file)?;
        repo.add_path(&session.offset_file)?;
        repo.commit(&format!(
            "commit session {}/{}/{} offset {}",
            session.owner, session.agent_id, session.session_id, total
        ))?;

        Ok(CommitResult::Committed {
            messages_processed: new_message_count,
            changes: vec![],
        })
    }

    /// Read the full message history for a session.
    pub fn read_full_history(&self, session: &Session) -> Result<Vec<Message>> {
        if !session.messages_file.exists() {
            return Ok(vec![]);
        }
        let content = std::fs::read_to_string(&session.messages_file).into_exn()?;
        let messages: Vec<Message> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        Ok(messages)
    }
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
        assert_eq!(session.offset, 0);

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

        let mut session = unwrap(mgr.get_or_create("alice", "mybot", "s1"));

        unwrap(mgr.add_message(&session, "user".into(), "msg1".into()).await);
        unwrap(mgr.add_message(&session, "user".into(), "msg2".into()).await);

        let result = unwrap(mgr.commit_session(&mut session).await);
        match result {
            CommitResult::Committed {
                messages_processed, ..
            } => assert_eq!(messages_processed, 2),
            _ => panic!("expected Committed"),
        }
        assert_eq!(session.offset, 2);

        let result = unwrap(mgr.commit_session(&mut session).await);
        match result {
            CommitResult::NoNewMessages => {}
            _ => panic!("expected NoNewMessages"),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}

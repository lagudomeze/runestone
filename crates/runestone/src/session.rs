use std::{
    cell::Cell,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

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
    pub fn offset(&self) -> usize {
        self.offset.get()
    }
}

/// Manages session lifecycle. Generic over the extractor type `E`.
/// Cheap to clone — the lock is shared.
#[derive(Clone)]
pub(crate) struct SessionManager<E: Extractor> {
    data_dir: PathBuf,
    lock: Arc<Mutex<()>>,
    extractor: E,
}

/// Result of a commit operation.
#[derive(Debug)]
pub enum CommitResult {
    Committed { messages_processed: usize, changes: Vec<MemoryChange> },
    NoNewMessages,
}

impl CommitResult {
    pub fn messages_processed(&self) -> usize {
        match self {
            CommitResult::Committed { messages_processed, .. } => *messages_processed,
            CommitResult::NoNewMessages => 0,
        }
    }

    pub fn changes(&self) -> &[MemoryChange] {
        match self {
            CommitResult::Committed { changes, .. } => changes,
            CommitResult::NoNewMessages => &[],
        }
    }
}

impl<E: Extractor> SessionManager<E> {
    pub(crate) fn new(data_dir: PathBuf, extractor: E) -> Self {
        Self { data_dir, lock: Arc::new(Mutex::new(())), extractor }
    }

    /// Change the extractor type (consumes self, reuses shared lock).
    pub(crate) fn with_extractor<E2: Extractor>(self, ext: E2) -> SessionManager<E2> {
        SessionManager { data_dir: self.data_dir, lock: self.lock, extractor: ext }
    }

    fn repo_for_owner(&self, owner: &str) -> Result<GitRepo> {
        GitRepo::open_or_init(&self.data_dir.join(owner))
    }

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
        let new_msgs = read_messages_range(&session.messages_file, cur_offset)?;
        let changes = self.extractor.extract(&new_msgs).await.unwrap_or_default();

        // Apply extracted changes to disk
        let mut changed_files =
            apply_changes(&self.data_dir, &session.owner, &changes, &session.base_path)?;

        // Regenerate L0 (.abstract.md) and L1 (.overview.md) for dirty directories
        let summary_files =
            regenerate_summaries(&self.data_dir, &session.owner, &changed_files, &self.extractor)
                .await
                .unwrap_or_default();
        changed_files.extend(summary_files);

        session.offset.set(total);
        std::fs::write(&session.offset_file, total.to_string()).into_exn()?;

        let repo = self.repo_for_owner(&session.owner)?;
        repo.add_path(&session.messages_file)?;
        repo.add_path(&session.offset_file)?;
        for f in &changed_files {
            repo.add_path(f)?;
        }
        repo.commit(&format!(
            "commit session {}/{}/{} offset {}",
            session.owner, session.agent_id, session.session_id, total
        ))?;

        Ok(CommitResult::Committed { messages_processed: new_message_count, changes })
    }

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

fn read_messages_range(file: &PathBuf, offset: usize) -> Result<Vec<Message>> {
    if !file.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(file).into_exn()?;
    let messages: Vec<Message> =
        content.lines().skip(offset).filter_map(|line| serde_json::from_str(line).ok()).collect();
    Ok(messages)
}

/// Write extracted MemoryChanges to disk. Returns the list of files modified
/// (for git staging).
fn apply_changes(
    data_dir: &Path,
    owner: &str,
    changes: &[MemoryChange],
    session_path: &Path,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let base = data_dir.join(owner);

    for c in changes {
        let (path, content) = match c {
            MemoryChange::GlobalProfile { content } => {
                (base.join("memory/profile.md"), content.as_str())
            }
            MemoryChange::GlobalPreference { key, value } => {
                (base.join("memory/preferences").join(format!("{key}.md")), value.as_str())
            }
            MemoryChange::GlobalEntity { name, description } => {
                (base.join("memory/entities").join(format!("{name}.md")), description.as_str())
            }
            MemoryChange::GlobalEvent { title, detail } => {
                (base.join("memory/events").join(format!("{title}.md")), detail.as_str())
            }
            MemoryChange::AgentCase { agent_id, title, content } => {
                let p = base
                    .join("agents")
                    .join(agent_id)
                    .join("memory/cases")
                    .join(format!("{title}.md"));
                (p, content.as_str())
            }
            MemoryChange::AgentPattern { agent_id, name, workflow } => {
                let p = base
                    .join("agents")
                    .join(agent_id)
                    .join("memory/patterns")
                    .join(format!("{name}.md"));
                (p, workflow.as_str())
            }
            MemoryChange::AgentTool { agent_id, name, usage } => {
                let p = base
                    .join("agents")
                    .join(agent_id)
                    .join("memory/tools")
                    .join(format!("{name}.md"));
                (p, usage.as_str())
            }
            MemoryChange::AgentSkill { agent_id, name, steps } => {
                let p = base
                    .join("agents")
                    .join(agent_id)
                    .join("memory/skills")
                    .join(format!("{name}.md"));
                (p, steps.as_str())
            }
            MemoryChange::UpdateAbstract { session_path: _, content } => {
                (session_path.join(".abstract.md"), content.as_str())
            }
            MemoryChange::UpdateOverview { session_path: _, content } => {
                (session_path.join(".overview.md"), content.as_str())
            }
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).into_exn()?;
        }
        std::fs::write(&path, content).into_exn()?;
        files.push(path);
    }

    Ok(files)
}

/// After files change, regenerate L0 (.abstract.md) and L1 (.overview.md)
/// for affected directories, bottom-up.
async fn regenerate_summaries<E: Extractor>(
    data_dir: &Path,
    owner: &str,
    changed_files: &[PathBuf],
    extractor: &E,
) -> Result<Vec<PathBuf>> {
    use std::collections::BTreeSet;

    let base = data_dir.join(owner);

    // Collect unique parent directories of changed files
    let mut dirs: BTreeSet<PathBuf> = BTreeSet::new();
    for f in changed_files {
        if let Some(parent) = f.parent() {
            let d = parent.to_path_buf();
            // Walk up to find all ancestors within the owner root
            let mut current = d.clone();
            while current.starts_with(&base) && current != base {
                dirs.insert(current.clone());
                if let Some(p) = current.parent() {
                    current = p.to_path_buf();
                } else {
                    break;
                }
            }
        }
    }

    // Sort by depth (deepest first) so children update before parents
    let mut sorted: Vec<PathBuf> = dirs.into_iter().collect();
    sorted.sort_by_key(|d| -(d.components().count() as isize));

    use crate::extractor::FileEntry;

    let mut generated = Vec::new();
    for dir in &sorted {
        // Collect non-meta .md files in this directory
        let mut files: Vec<FileEntry> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let name =
                    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if p.extension().is_some_and(|e| e == "md")
                    && name != ".abstract.md"
                    && name != ".overview.md"
                    && p.is_file()
                    && let Ok(content) = std::fs::read_to_string(&p)
                {
                    files.push(FileEntry { name, content });
                }
            }
        }

        if files.is_empty() {
            continue;
        }

        // Generate L0 abstract (incremental: read existing abstract first)
        let dir_name = dir.strip_prefix(&base).unwrap_or(dir).to_string_lossy().to_string();
        let abstract_path = dir.join(".abstract.md");
        let existing = std::fs::read_to_string(&abstract_path).ok();
        let abstract_md =
            extractor.summarize_directory(&dir_name, existing.as_deref(), &files).await?;
        std::fs::write(&abstract_path, &abstract_md).into_exn()?;
        generated.push(abstract_path);

        // Generate L1 overview for top-level memory dirs
        if is_top_memory_dir(dir, &base) {
            let mut children: Vec<FileEntry> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let sub = entry.path();
                    if sub.is_dir() {
                        let abs_file = sub.join(".abstract.md");
                        if let Ok(content) = std::fs::read_to_string(&abs_file) {
                            let name = sub
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            children.push(FileEntry {
                                name,
                                content: content.lines().next().unwrap_or("").to_string(),
                            });
                        }
                    }
                }
            }
            if !children.is_empty() {
                let overview_path = dir.join(".overview.md");
                let overview = extractor.generate_overview(&dir_name, &children).await?;
                std::fs::write(&overview_path, &overview).into_exn()?;
                generated.push(overview_path);
            }
        }
    }

    Ok(generated)
}

/// Check if a directory is a "top-level" memory directory deserving L1
/// overview.
fn is_top_memory_dir(dir: &Path, base: &Path) -> bool {
    // memory/ or agents/{agent}/memory/
    let rel = dir.strip_prefix(base).unwrap_or(dir);
    // memory dir immediately under owner root
    if rel == Path::new("memory") {
        return true;
    }
    // agents/{agent}/memory/ — 4 components deep from owner root
    let components: Vec<_> = rel.components().collect();
    components.len() == 4
        && components[0].as_os_str() == "agents"
        && components[2].as_os_str() == "memory"
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
        let mgr = SessionManager::new(dir.clone(), crate::extractor::NoopExtractor);
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
        let mgr = SessionManager::new(dir.clone(), crate::extractor::NoopExtractor);
        let session = unwrap(mgr.get_or_create("alice", "mybot", "s1"));
        unwrap(mgr.add_message(&session, "user".into(), "msg1".into()).await);
        unwrap(mgr.add_message(&session, "user".into(), "msg2".into()).await);
        let result = unwrap(mgr.commit_session(&session).await);
        match result {
            CommitResult::Committed { messages_processed, .. } => assert_eq!(messages_processed, 2),
            _ => panic!("expected Committed"),
        }
        assert_eq!(session.offset(), 2);
        let result = unwrap(mgr.commit_session(&session).await);
        match result {
            CommitResult::NoNewMessages => {}
            _ => panic!("expected NoNewMessages"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}

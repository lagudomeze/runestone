//! # Runestone
//!
//! A personal AI memory system based on Rust + Git.
//!
//! ## Quick start
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() {
//! use runestone::Runestone;
//!
//! let rs = Runestone::new("./data", "alice");
//! let agent = rs.agent("mybot");
//!
//! // Sessions are scoped to the agent
//! let s = agent.session_open("s1").unwrap();
//! agent.session_add(&s, "user", "Hello").await.unwrap();
//! let result = agent.session_commit(&s).await.unwrap();
//! for msg in agent.session_history(&s).unwrap() {
//!     println!("[{}] {}: {}", msg.timestamp, msg.role, msg.content);
//! }
//! # }
//! ```
//!
//! ## Memory operations
//!
//! ```no_run
//! use runestone::{MemoryKind, Preference, Profile, Runestone};
//!
//! let rs = Runestone::new("./data", "alice");
//!
//! // Global memory — on Runestone
//! rs.memory_store(&Profile, &"Alice, engineer".to_string()).unwrap();
//! assert_eq!(rs.memory_load(&Profile).unwrap(), Some("Alice, engineer".to_string()));
//!
//! // Agent memory — on Agent
//! let agent = rs.agent("mybot");
//! let pref = Preference { key: "language".into() };
//! agent.memory_store(&pref, &"Rust".to_string()).unwrap();
//! assert_eq!(agent.memory_load(&pref).unwrap(), Some("Rust".to_string()));
//! ```

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use session::SessionManager;

use crate::error::IntoExn;

mod error;
pub mod extractor;
mod git_repo;
mod memory;
mod session;

// ── Re-exports ──────────────────────────────────────────────────────────────

pub use error::{Result, RunestoneError};
pub use memory::{Case, Entity, Event, MemoryChange, MemoryHit, MemoryKind, Preference, Profile};
pub use session::{CommitResult, Message, Session};

// ── Runestone ────────────────────────────────────────────────────────────────

/// Entry point for the Runestone memory system.
///
/// Owns user-level (global) memories. Use [`agent`](Self::agent) to get an
/// agent-scoped handle for session and agent-level memory operations.
pub struct Runestone {
    owner: String,
    data_dir: PathBuf,
    sessions: SessionManager,
}

impl Runestone {
    pub fn new(data_dir: impl Into<PathBuf>, owner: impl Into<String>) -> Self {
        let dir: PathBuf = data_dir.into();
        Self { owner: owner.into(), sessions: SessionManager::new(dir.clone()), data_dir: dir }
    }

    /// Attach an LLM extractor. When set, every [`Agent::session_commit`] will
    /// run extraction and populate [`CommitResult::changes`].
    pub fn with_extractor(mut self, ext: impl crate::extractor::Extractor + 'static) -> Self {
        self.sessions.set_extractor(Arc::new(ext));
        self
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// Get a handle to an agent. Agents own sessions and agent-level memories.
    /// The handle is cheap to create and can outlive this `Runestone`.
    pub fn agent(&self, agent_id: &str) -> Agent {
        Agent {
            owner: self.owner.clone(),
            id: agent_id.to_string(),
            data_dir: self.data_dir.clone(),
            sessions: self.sessions.clone(),
        }
    }

    // ── Global memory ────────────────────────────────────────────────────

    pub fn memory_store<K: MemoryKind + ?Sized>(&self, kind: &K, value: &K::Value) -> Result<()> {
        write_memory(&self.owner, &self.data_dir, kind, value)
    }

    pub fn memory_load<K: MemoryKind + ?Sized>(&self, kind: &K) -> Result<Option<K::Value>> {
        read_memory(&self.owner, &self.data_dir, kind)
    }

    /// List all memory files across global and all agent directories.
    pub fn memory_list(&self) -> Result<Vec<String>> {
        let base = self.data_dir.join(&self.owner);
        let mut files = Vec::new();
        walk_md_files(&base, &self.data_dir, &mut files)?;
        files.sort();
        Ok(files)
    }

    #[allow(unused_variables)]
    pub fn memory_search(&self, query: &str, limit: usize) -> Result<Vec<MemoryHit>> {
        Err(RunestoneError::Other("memory_search is not yet implemented (Phase 3)".into()).into())
    }

    #[allow(unused_variables)]
    pub fn resource_add(&self, uri: &str) -> Result<()> {
        Err(RunestoneError::Other("resource_add is not yet implemented (Phase 2)".into()).into())
    }
}

// ── Agent ────────────────────────────────────────────────────────────────────

/// A handle to a specific agent.
///
/// Agents own sessions and agent-level memories. An agent is created via
/// [`Runestone::agent`] and is independent of the `Runestone` instance —
/// it can outlive it thanks to shared ownership of the underlying session
/// manager (via `Clone`).
pub struct Agent {
    owner: String,
    id: String,
    data_dir: PathBuf,
    sessions: SessionManager,
}

impl Agent {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn owner(&self) -> &str {
        &self.owner
    }

    // ── Session ──────────────────────────────────────────────────────────

    pub fn session_open(&self, session_id: &str) -> Result<Session> {
        self.sessions.get_or_create(&self.owner, &self.id, session_id)
    }

    pub async fn session_add(&self, session: &Session, role: &str, content: &str) -> Result<()> {
        self.sessions.add_message(session, role.to_string(), content.to_string()).await
    }

    /// Commit unprocessed messages. In Phase 2 the agent will asynchronously
    /// extract memories from committed messages and write them to its memory
    /// directory.
    pub async fn session_commit(&self, session: &Session) -> Result<CommitResult> {
        self.sessions.commit_session(session).await
    }

    pub fn session_history(&self, session: &Session) -> Result<Vec<Message>> {
        self.sessions.read_full_history(session)
    }

    // ── Agent memory ─────────────────────────────────────────────────────

    pub fn memory_store<K: MemoryKind + ?Sized>(&self, kind: &K, value: &K::Value) -> Result<()> {
        write_memory(&self.owner, &self.data_dir, kind, value)
    }

    pub fn memory_load<K: MemoryKind + ?Sized>(&self, kind: &K) -> Result<Option<K::Value>> {
        read_memory(&self.owner, &self.data_dir, kind)
    }

    /// List memory files under this agent's directory.
    pub fn memory_list(&self) -> Result<Vec<String>> {
        let base = self.data_dir.join(&self.owner).join("agents").join(&self.id).join("memory");
        let mut files = Vec::new();
        walk_md_files(&base, &self.data_dir, &mut files)?;
        files.sort();
        Ok(files)
    }
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn abs_path(owner: &str, data_dir: &Path, rel: &Path) -> PathBuf {
    data_dir.join(owner).join(rel)
}

fn write_memory<K: MemoryKind + ?Sized>(
    owner: &str,
    data_dir: &Path,
    kind: &K,
    value: &K::Value,
) -> Result<()> {
    let full = abs_path(owner, data_dir, &kind.path());
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).into_exn()?;
    }
    let content = kind.encode(value);
    std::fs::write(&full, content).into_exn()?;
    Ok(())
}

fn read_memory<K: MemoryKind + ?Sized>(
    owner: &str,
    data_dir: &Path,
    kind: &K,
) -> Result<Option<K::Value>> {
    let full = abs_path(owner, data_dir, &kind.path());
    if !full.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&full).into_exn()?;
    kind.decode(&raw).map(Some)
}

fn walk_md_files(dir: &Path, data_dir: &Path, files: &mut Vec<String>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).into_exn()? {
        let entry = entry.into_exn()?;
        let path = entry.path();
        if path.is_dir() {
            walk_md_files(&path, data_dir, files)?;
        } else if path.extension().is_some_and(|e| e == "md")
            && let Ok(rel) = path.strip_prefix(data_dir)
        {
            files.push(rel.to_string_lossy().into_owned());
        }
    }
    Ok(())
}

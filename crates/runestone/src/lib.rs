#![deny(unused_crate_dependencies)]

//! # Runestone
//!
//! A personal AI memory system based on Rust + Git.
//!
//! ## Quick start
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() {
//! use runestone::{NoopExtractor, Runestone};
//!
//! let rs = Runestone::new("./data", "alice", NoopExtractor);
//! let agent = rs.agent("mybot");
//!
//! let s = agent.session_open("s1").unwrap();
//! agent.session_add(&s, "user", "Hello").await.unwrap();
//! let result = agent.session_commit(&s).await.unwrap();
//! for msg in agent.session_history(&s).unwrap() {
//!     println!("[{}] {}: {}", msg.timestamp, msg.role, msg.content);
//! }
//! # }
//! ```
//!
//! ## With LLM extraction
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() {
//! use runestone::{Runestone, extractor::RigExtractor};
//!
//! // let model = rig::providers::openai::Client::from_env().completion_model("gpt-4o-mini");
//! // let rs = Runestone::new("./data", "alice", RigExtractor::new(model));
//! # }
//! ```

use std::path::{Path, PathBuf};

use extractor::Extractor;
use session::SessionManager;
use tokio::sync::Mutex;

use crate::error::IntoExn;

mod error;
pub mod extractor;
mod git_repo;
mod index;
mod memory;
mod retriever;
mod session;

// ── Re-exports ──────────────────────────────────────────────────────────────

pub use error::{Result, RunestoneError};
pub use extractor::{FileEntry, NoopExtractor};
pub use memory::{Case, Entity, Event, MemoryChange, MemoryHit, MemoryKind, Preference, Profile};
pub use session::{CommitResult, Message, Session};

// ── Runestone ────────────────────────────────────────────────────────────────

/// Entry point. Generic over the extractor type `E`.
pub struct Runestone<E: Extractor> {
    owner: String,
    data_dir: PathBuf,
    sessions: SessionManager<E>,
    index_cache: Mutex<Option<index::Index>>,
}

impl<E: Extractor> Runestone<E> {
    pub fn new(data_dir: impl Into<PathBuf>, owner: impl Into<String>, extractor: E) -> Self {
        let dir: PathBuf = data_dir.into();
        Self {
            owner: owner.into(),
            sessions: SessionManager::new(dir.clone(), extractor),
            index_cache: Mutex::new(None),
            data_dir: dir,
        }
    }

    /// Change the extractor type (consumes self, preserves index).
    pub fn with_extractor<E2: Extractor>(self, ext: E2) -> Runestone<E2> {
        Runestone {
            owner: self.owner,
            data_dir: self.data_dir.clone(),
            sessions: self.sessions.with_extractor(ext),
            index_cache: self.index_cache,
        }
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn agent(&self, agent_id: &str) -> Agent<E>
    where
        E: Clone,
    {
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

    pub fn memory_list(&self) -> Result<Vec<String>> {
        let base = self.data_dir.join(&self.owner);
        let mut files = Vec::new();
        walk_md_files(&base, &self.data_dir, &mut files)?;
        files.sort();
        Ok(files)
    }

    /// Keyword search over memory files (sync, no embedding model needed).
    pub fn memory_search(&self, query: &str, limit: usize) -> Result<Vec<MemoryHit>> {
        let base = self.data_dir.join(&self.owner);
        retriever::search(&base, &self.data_dir, query, limit)
    }

    /// Recursive search: vector on L0 → LLM routing → L1 overview → L2 files.
    pub async fn memory_search_deep(&self, query: &str, limit: usize) -> Result<Vec<MemoryHit>>
    where
        E: Clone,
    {
        let base = self.data_dir.join(&self.owner);
        let mut cache = self.index_cache.lock().await;
        if cache.is_none() {
            *cache = Some(index::Index::build(&base, &self.data_dir).await);
        }
        let idx = cache.as_ref().unwrap();
        idx.recursive_search(query, limit, &self.data_dir, self.sessions.get_extractor().clone())
            .await
    }

    /// Semantic search using vector similarity on L0 abstracts.
    /// Builds the index on first call (downloads model ~80MB).
    pub async fn memory_search_semantic(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemoryHit>> {
        if self.index_cache.lock().await.is_none() {
            let base = self.data_dir.join(&self.owner);
            let idx = index::Index::build(&base, &self.data_dir).await;
            *self.index_cache.lock().await = Some(idx);
        }
        let cache = self.index_cache.lock().await;
        let idx = cache.as_ref().unwrap();
        idx.search_async(query, limit).await
    }

    /// Rebuild the semantic search index (re-downloads model if needed).
    pub async fn index_rebuild(&self) -> Result<()> {
        let base = self.data_dir.join(&self.owner);
        let idx = index::Index::build(&base, &self.data_dir).await;
        *self.index_cache.lock().await = Some(idx);
        Ok(())
    }

    #[allow(unused_variables)]
    pub fn resource_add(&self, uri: &str) -> Result<()> {
        Err(RunestoneError::Other("resource_add is not yet implemented (Phase 2)".into()).into())
    }

    /// Initialise the owner's data directory by cloning a remote repo.
    pub fn git_init(&self, remote_url: &str) -> Result<()> {
        let path = self.data_dir.join(&self.owner);
        tracing::debug!("[init] cloning {remote_url} into {path:?}");
        crate::git_repo::GitRepo::clone(&path, remote_url)?;
        tracing::debug!("[init] done");
        Ok(())
    }

    /// Sync the owner's git repo with a remote: pull rebase, then push.
    pub fn sync(&self, remote_url: &str) -> Result<()> {
        tracing::debug!("[sync] opening repo at {:?}", self.data_dir.join(&self.owner));
        let repo = crate::git_repo::GitRepo::open_or_init(&self.data_dir.join(&self.owner))?;
        tracing::debug!("[sync] committing pending changes...");
        repo.commit_all("runestone sync auto-commit")?;
        tracing::debug!("[sync] pull_rebase...");
        repo.pull_rebase(remote_url)?;
        tracing::debug!("[sync] push...");
        repo.push(remote_url)?;
        tracing::debug!("[sync] done");
        Ok(())
    }
}

// ── Agent ────────────────────────────────────────────────────────────────────

pub struct Agent<E: Extractor> {
    owner: String,
    id: String,
    data_dir: PathBuf,
    sessions: SessionManager<E>,
}

impl<E: Extractor> Agent<E> {
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
    std::fs::write(&full, kind.encode(value)).into_exn()?;

    // Regenerate parent directory's L0 abstract
    regenerate_abstract_for_dir(owner, data_dir, full.parent());
    Ok(())
}

/// Generate a simple `.abstract.md` for a directory by listing its files.
fn regenerate_abstract_for_dir(owner: &str, data_dir: &Path, dir: Option<&Path>) {
    let Some(dir) = dir else { return };
    let base = data_dir.join(owner);
    let Ok(rel) = dir.strip_prefix(&base) else { return };

    let mut files: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if p.extension().is_some_and(|e| e == "md")
                && name != ".abstract.md"
                && name != ".overview.md"
                && p.is_file()
            {
                files.push(name);
            }
        }
    }

    if files.is_empty() {
        return;
    }
    let summary = format!("{}: {}", rel.to_string_lossy(), files.join(", "));
    let _ = std::fs::write(dir.join(".abstract.md"), summary);
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

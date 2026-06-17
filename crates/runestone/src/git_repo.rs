use std::path::{Path, PathBuf};

use git2::{RemoteCallbacks, Repository, Signature};
use tracing::{debug, warn};

use crate::error::{IntoExn, Result, RunestoneError};

/// Git repository wrapper, one per owner at `./data/{owner}/`.
pub(crate) struct GitRepo {
    repo: Repository,
    workdir: PathBuf,
}

/// Per-operation credential provider. Create a new instance for each
/// push/fetch. Rotates through sources: host-specific keys → scanned ~/.ssh
/// keys → SSH agent → default.
struct CredentialProvider {
    user: String,
    sources: Vec<CredSource>,
    attempt: usize,
}

enum CredSource {
    KeyFile(PathBuf),
    SshAgent,
    Default,
}

impl CredentialProvider {
    fn new(url: &str, username: Option<&str>) -> Self {
        let user = username.unwrap_or("git").to_string();
        let mut sources = Vec::new();

        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let ssh_dir = home.join(".ssh");

            // 1. Host-specific IdentityFile from ~/.ssh/config
            let host = extract_host(url);
            if let Some(identity_files) = ssh_config_identity_files(&home, &host) {
                for path in identity_files {
                    sources.push(CredSource::KeyFile(path));
                }
            }

            // 2. Scan ~/.ssh/ for standard private key files
            if let Ok(entries) = std::fs::read_dir(&ssh_dir) {
                let mut key_paths: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| is_private_key(p))
                    .collect();
                key_paths.sort();
                for path in key_paths {
                    if !sources.iter().any(|s| matches!(s, CredSource::KeyFile(p) if p == &path)) {
                        sources.push(CredSource::KeyFile(path));
                    }
                }
            }
        }

        // 3. SSH agent
        sources.push(CredSource::SshAgent);

        // 4. Default
        sources.push(CredSource::Default);

        debug!("[cred] provider init: user={user}, {} sources", sources.len());
        for (i, s) in sources.iter().enumerate() {
            debug!("[cred]   [{i}] {s:?}");
        }

        Self { user, sources, attempt: 0 }
    }

    fn next(&mut self) -> std::result::Result<git2::Cred, git2::Error> {
        while self.attempt < self.sources.len() {
            let i = self.attempt;
            self.attempt += 1;
            let result = match &self.sources[i] {
                CredSource::KeyFile(path) => git2::Cred::ssh_key(&self.user, None, path, None),
                CredSource::SshAgent => git2::Cred::ssh_key_from_agent(&self.user),
                CredSource::Default => git2::Cred::default(),
            };
            match result {
                Ok(cred) => {
                    debug!("[cred] attempt {i} → ok ({:?})", self.sources[i]);
                    return Ok(cred);
                }
                Err(e) => {
                    debug!("[cred] attempt {i} → err ({:?}): {e}", self.sources[i]);
                }
            }
        }
        Err(git2::Error::from_str("no more auth methods"))
    }
}

impl std::fmt::Debug for CredSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CredSource::KeyFile(p) => write!(f, "key {}", p.display()),
            CredSource::SshAgent => write!(f, "ssh-agent"),
            CredSource::Default => write!(f, "default"),
        }
    }
}

/// Extract hostname from a git URL like `git@github.com:user/repo.git` or
/// `ssh://git@host/path`.
fn extract_host(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("git@")
        && let Some(host) = rest.split(':').next()
    {
        return host.to_string();
    }
    if let Some(rest) = url.strip_prefix("ssh://") {
        let rest = rest.trim_start_matches("git@");
        if let Some(host) = rest.split('/').next()
            && let Some(h) = host.split(':').next()
        {
            return h.to_string();
        }
    }
    if let Some(rest) = url.strip_prefix("https://")
        && let Some(host) = rest.split('/').next()
    {
        return host.to_string();
    }
    "".to_string()
}

/// Parse `~/.ssh/config` for IdentityFile directives matching `host`.
fn ssh_config_identity_files(home: &Path, host: &str) -> Option<Vec<PathBuf>> {
    let config_path = home.join(".ssh").join("config");
    let content = std::fs::read_to_string(&config_path).ok()?;
    let mut paths = Vec::new();
    let mut current_host_matches = false;
    let mut wildcard_matches = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        let keyword = parts.next()?.to_lowercase();
        let value = parts.next()?;

        if keyword == "host" {
            current_host_matches = value == host || (value == "*" && !wildcard_matches);
            if value == "*" {
                wildcard_matches = true;
            }
            continue;
        }

        if (current_host_matches || wildcard_matches) && keyword == "identityfile" {
            let expanded = if let Some(stripped) = value.strip_prefix("~/") {
                home.join(stripped)
            } else if value == "~" {
                home.to_path_buf()
            } else if value.starts_with('/') {
                PathBuf::from(value)
            } else {
                home.join(".ssh").join(value)
            };
            if expanded.exists() {
                paths.push(expanded);
            }
        }
    }

    if paths.is_empty() { None } else { Some(paths) }
}

/// Heuristic: is this file likely an SSH private key?
fn is_private_key(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if name.starts_with('.')
        || name.ends_with(".pub")
        || name == "config"
        || name == "authorized_keys"
        || name.starts_with("known_hosts")
    {
        return false;
    }
    path.is_file()
}

impl GitRepo {
    /// Clone a remote repo (e.g. `git clone --depth 1` equivalent).
    pub(crate) fn clone(path: &Path, remote_url: &str) -> Result<Self> {
        if path.exists() {
            std::fs::remove_dir_all(path).into_exn()?;
        }
        std::fs::create_dir_all(path).into_exn()?;

        let mut provider = CredentialProvider::new(remote_url, None);
        let mut cb = RemoteCallbacks::new();
        cb.credentials(move |_url, _username, _allowed| provider.next());

        let mut fo = git2::FetchOptions::new();
        fo.remote_callbacks(cb).depth(1);

        let repo =
            git2::build::RepoBuilder::new().fetch_options(fo).clone(remote_url, path).into_exn()?;

        let workdir = repo
            .workdir()
            .ok_or_else(|| RunestoneError::Other("bare repository has no workdir".into()))?
            .canonicalize()
            .into_exn()?;
        Ok(Self { repo, workdir })
    }

    pub(crate) fn open_or_init(path: &Path) -> Result<Self> {
        let repo = match Repository::open(path) {
            Ok(repo) => repo,
            Err(_) => {
                std::fs::create_dir_all(path).into_exn()?;
                Repository::init(path).into_exn()?
            }
        };
        let workdir = repo
            .workdir()
            .ok_or_else(|| RunestoneError::Other("bare repository has no workdir".into()))?
            .canonicalize()
            .into_exn()?;
        Ok(Self { repo, workdir })
    }

    fn resolve(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(path)
        }
    }

    pub(crate) fn add_path(&self, path: &Path) -> Result<()> {
        let mut index = self.repo.index().into_exn()?;
        let abs = self.resolve(path);
        let relative = abs.strip_prefix(&self.workdir).unwrap_or(&abs);
        index.add_path(relative).into_exn()?;
        index.write().into_exn()?;
        Ok(())
    }

    pub(crate) fn commit(&self, message: &str) -> Result<git2::Oid> {
        let sig = Signature::now("runestone", "runestone@local").into_exn()?;
        let mut index = self.repo.index().into_exn()?;
        let tree_id = index.write_tree().into_exn()?;
        let tree = self.repo.find_tree(tree_id).into_exn()?;

        let parents = match self.repo.head() {
            Ok(head) => {
                let obj = head.peel_to_commit().into_exn()?;
                vec![obj]
            }
            Err(_) => vec![],
        };
        let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();

        self.repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs).into_exn()
    }

    // ── Remote sync ────────────────────────────────────────────────────────

    /// Stage all files and commit (no-op if index is clean).
    pub(crate) fn commit_all(&self, message: &str) -> Result<Option<git2::Oid>> {
        let mut index = self.repo.index().into_exn()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).into_exn()?;
        index.write().into_exn()?;
        let tree_id = index.write_tree().into_exn()?;
        let tree = self.repo.find_tree(tree_id).into_exn()?;

        let parents = match self.repo.head() {
            Ok(head) => {
                let obj = head.peel_to_commit().into_exn()?;
                vec![obj]
            }
            Err(_) => vec![],
        };

        if let Some(parent) = parents.first()
            && parent.tree_id() == tree_id
        {
            return Ok(None);
        }

        let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
        let sig = Signature::now("runestone", "runestone@local").into_exn()?;
        let oid =
            self.repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs).into_exn()?;
        debug!("[commit] {oid}");
        Ok(Some(oid))
    }

    /// Push the current branch to the given remote URL.
    pub(crate) fn push(&self, remote_url: &str) -> Result<()> {
        let (branch, oid) = {
            let head = self.repo.head().into_exn()?;
            let branch = head.shorthand().unwrap_or("main").to_string();
            if branch == "HEAD" {
                return Err(RunestoneError::Other(
                    "HEAD is detached; cannot determine branch to push".into(),
                )
                .into());
            }
            let oid = head.peel_to_commit().into_exn()?.id();
            (branch, oid)
        };
        let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
        debug!("[push] branch={branch}, refspec={refspec}");

        let mut provider = CredentialProvider::new(remote_url, None);
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(move |_url, _username, _allowed| provider.next());
        callbacks.sideband_progress(|data| {
            debug!("[push] sideband: {}", String::from_utf8_lossy(data));
            true
        });
        callbacks.push_update_reference(|refname, status| {
            if let Some(s) = status {
                warn!("[push] ref {refname} rejected: {s}");
            } else {
                debug!("[push] ref {refname} accepted");
            }
            Ok(())
        });
        callbacks.transfer_progress(|p| {
            debug!(
                "[push] transfer: {}/{} objects, {} KiB",
                p.received_objects(),
                p.total_objects(),
                p.received_bytes() / 1024,
            );
            true
        });

        debug!("[push] creating remote_anonymous...");
        let mut remote = self
            .repo
            .remote_anonymous(remote_url)
            .into_exn()
            .map_err(|e| RunestoneError::Other(format!("invalid remote URL: {e}")))?;

        debug!("[push] pushing...");
        remote
            .push(&[&refspec], Some(git2::PushOptions::new().remote_callbacks(callbacks)))
            .into_exn()?;
        drop(remote);

        let tracked = format!("refs/remotes/origin/{branch}");
        self.repo.reference(&tracked, oid, true, "sync").into_exn()?;
        debug!("[push] updated {tracked}");
        debug!("[push] done");
        Ok(())
    }

    /// Pull with rebase from the given remote URL.
    pub(crate) fn pull_rebase(&self, remote_url: &str) -> Result<()> {
        debug!("[pull] creating remote_anonymous...");
        let mut remote = self
            .repo
            .remote_anonymous(remote_url)
            .into_exn()
            .map_err(|e| RunestoneError::Other(format!("invalid remote URL: {e}")))?;

        let mut provider = CredentialProvider::new(remote_url, None);
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(move |_url, _username, _allowed| provider.next());

        if self.repo.find_reference("refs/remotes/origin/HEAD").is_ok()
            || self.repo.references_glob("refs/remotes/origin/*").into_exn()?.count() > 0
        {
            debug!("[pull] fetching...");
            remote
                .fetch(
                    &["refs/heads/*:refs/remotes/origin/*"],
                    Some(git2::FetchOptions::new().remote_callbacks(callbacks)),
                    None,
                )
                .into_exn()?;
            debug!("[pull] fetch done");
        } else {
            debug!("[pull] first sync, skipping fetch");
        }

        let fetch_head = match self.repo.find_reference("FETCH_HEAD") {
            Ok(r) => r,
            Err(_) => {
                debug!("[pull] remote has no refs, skipping rebase");
                return Ok(());
            }
        };
        let fetch_commit = fetch_head.peel_to_commit().into_exn()?;

        let head = self.repo.head().into_exn()?;
        let head_commit = head.peel_to_commit().into_exn()?;

        // Nothing to rebase if fetch head is already our ancestor
        if fetch_commit.id() == head_commit.id()
            || self.repo.graph_descendant_of(head_commit.id(), fetch_commit.id()).unwrap_or(false)
        {
            debug!("[pull] already up to date, skipping rebase");
            return Ok(());
        }

        let annotated = self.repo.reference_to_annotated_commit(&fetch_head).into_exn()?;
        let head_annotated = self.repo.reference_to_annotated_commit(&head).into_exn()?;

        debug!("[pull] rebasing...");
        let mut rebase = self
            .repo
            .rebase(
                Some(&head_annotated),
                Some(&annotated),
                None,
                Some(git2::RebaseOptions::new().inmemory(false).quiet(true)),
            )
            .into_exn()?;

        let sig = Signature::now("runestone", "runestone@local").into_exn()?;
        while let Some(op) = rebase.next() {
            let _op = op.into_exn()?;
            rebase.commit(None, &sig, None).into_exn()?;
        }
        rebase.finish(None).into_exn()?;

        debug!("[pull] done");
        Ok(())
    }
}

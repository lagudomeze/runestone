use std::path::{Path, PathBuf};

use git2::{RemoteCallbacks, Repository, Signature};

use crate::error::{IntoExn, Result, RunestoneError};

/// Git repository wrapper, one per owner at `./data/{owner}/`.
pub(crate) struct GitRepo {
    repo: Repository,
    workdir: PathBuf,
}

impl GitRepo {
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

    /// Push the current branch to the given remote URL.
    pub(crate) fn push(&self, remote_url: &str) -> Result<()> {
        let mut remote = self
            .repo
            .remote_anonymous(remote_url)
            .into_exn()
            .map_err(|e| RunestoneError::Other(format!("invalid remote URL: {e}")))?;

        let head = self.repo.head().into_exn()?;
        let branch = head.shorthand().unwrap_or("main");

        let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, _username, _allowed| {
            git2::Cred::ssh_key_from_agent("git").or_else(|_| git2::Cred::default())
        });

        remote
            .push(&[&refspec], Some(git2::PushOptions::new().remote_callbacks(callbacks)))
            .into_exn()?;

        Ok(())
    }

    /// Pull with rebase from the given remote URL.
    pub(crate) fn pull_rebase(&self, remote_url: &str) -> Result<()> {
        // Fetch
        let mut remote = self
            .repo
            .remote_anonymous(remote_url)
            .into_exn()
            .map_err(|e| RunestoneError::Other(format!("invalid remote URL: {e}")))?;

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, _username, _allowed| {
            git2::Cred::ssh_key_from_agent("git").or_else(|_| git2::Cred::default())
        });

        remote
            .fetch(
                &["refs/heads/*:refs/remotes/origin/*"],
                Some(git2::FetchOptions::new().remote_callbacks(callbacks)),
                None,
            )
            .into_exn()?;

        // Rebase onto FETCH_HEAD
        let fetch_head = self.repo.find_reference("FETCH_HEAD").into_exn()?;
        let fetch_commit = fetch_head.peel_to_commit().into_exn()?;
        let annotated = self.repo.reference_to_annotated_commit(&fetch_head).into_exn()?;

        let head = self.repo.head().into_exn()?;
        let head_commit = head.peel_to_commit().into_exn()?;
        let head_annotated = self.repo.reference_to_annotated_commit(&head).into_exn()?;

        self.repo
            .rebase(
                Some(&head_annotated),
                Some(&annotated),
                None,
                Some(git2::RebaseOptions::new().inmemory(false).quiet(true)),
            )
            .into_exn()?;

        // Update HEAD to the rebased commit
        if let Ok(rebase_head) = self.repo.head()
            && let Ok(rebase_commit) = rebase_head.peel_to_commit()
            && rebase_commit.id() != head_commit.id()
        {
            // Rebase was successful, nothing more to do
            let _ = fetch_commit; // suppress unused warning
        }

        Ok(())
    }
}

use crate::error::{IntoExn, Result, RunestoneError};
use git2::{IndexAddOption, Repository, Signature};
use std::path::{Path, PathBuf};

/// Git repository wrapper, one per owner at `./data/{owner}/`.
pub struct GitRepo {
    repo: Repository,
    workdir: PathBuf,
}

impl GitRepo {
    /// Open an existing git repo at `path`, or initialize a new one.
    pub fn open_or_init(path: &Path) -> Result<Self> {
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

    pub fn workdir(&self) -> &Path {
        &self.workdir
    }

    /// Resolve a potentially relative path to absolute using the current dir.
    fn resolve(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    }

    /// Stage a single file for commit.
    pub fn add_path(&self, path: &Path) -> Result<()> {
        let mut index = self.repo.index().into_exn()?;
        let abs = self.resolve(path);
        let relative = abs.strip_prefix(&self.workdir).unwrap_or(&abs);
        index.add_path(relative).into_exn()?;
        index.write().into_exn()?;
        Ok(())
    }

    /// Stage all files under a directory.
    pub fn add_dir(&self, dir: &Path) -> Result<()> {
        let mut index = self.repo.index().into_exn()?;
        let abs = self.resolve(dir);
        let relative = abs.strip_prefix(&self.workdir).unwrap_or(&abs);
        let spec = relative.to_string_lossy();
        index
            .add_all([spec.as_ref()], IndexAddOption::DEFAULT, None)
            .into_exn()?;
        index.write().into_exn()?;
        Ok(())
    }

    /// Create a commit with the given message. Returns the commit OID.
    pub fn commit(&self, message: &str) -> Result<git2::Oid> {
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

        let oid = self
            .repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
            .into_exn()?;
        Ok(oid)
    }
}

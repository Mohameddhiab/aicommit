use anyhow::{anyhow, Context, Result};
use git2::{DiffFormat, DiffOptions, Repository, Signature};
use std::path::{Path, PathBuf};

const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// A wrapper around a git2::Repository that can be constructed from any path.
pub struct GitRepo {
    pub repo: Repository,
}

impl GitRepo {
    /// Open the repository containing the given path (discovery upward).
    pub fn from_path(path: &Path) -> Result<Self> {
        let repo = Repository::discover(path)
            .with_context(|| format!("discover git repository at {}", path.display()))?;
        Ok(Self { repo })
    }

    /// Open the repository at the current working directory.
    pub fn from_current_dir() -> Result<Self> {
        let cwd = std::env::current_dir().context("get current dir")?;
        Self::from_path(&cwd)
    }

    pub fn head_tree(&self) -> Result<git2::Tree<'_>> {
        if let Ok(head) = self.repo.head() {
            return head.peel_to_tree().context("resolve head tree");
        }
        let oid = git2::Oid::from_str(EMPTY_TREE)?;
        self.repo
            .find_tree(oid)
            .map_err(|e| anyhow!("resolve empty tree: {e}"))
    }

    pub fn staged_diff(&self) -> Result<String> {
        let head = self.head_tree()?;
        let index = self.repo.index().context("get index")?;
        let mut opts = DiffOptions::new();
        opts.include_untracked(false);
        let diff = self
            .repo
            .diff_tree_to_index(Some(&head), Some(&index), Some(&mut opts))
            .context("diff tree to index")?;
        diff_to_string(&diff)
    }

    pub fn stage_all(&self) -> Result<()> {
        let mut index = self.repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    pub fn stage_specific(&self, paths: &[PathBuf]) -> Result<()> {
        let mut index = self.repo.index()?;
        for p in paths {
            index
                .add_path(p)
                .with_context(|| format!("add {}", p.display()))?;
        }
        index.write()?;
        Ok(())
    }

    pub fn reset_index_to_head(&self) -> Result<()> {
        let head = self.repo.head().and_then(|h| h.peel_to_commit())?;
        self.repo
            .reset(head.as_object(), git2::ResetType::Mixed, None)?;
        Ok(())
    }

    pub fn commit(&self, message: &str, paths: Option<&[PathBuf]>) -> Result<()> {
        if let Some(ps) = paths {
            self.stage_specific(ps)?;
        }
        let mut index = self.repo.index()?;
        let tree_oid = index.write_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;
        let sig = self
            .repo
            .signature()
            .unwrap_or_else(|_| Signature::now("aicommit", "aicommit@users.noreply.github.com").expect("create fallback signature"));
        let parents: Vec<git2::Commit<'_>> = match self.repo.head().ok() {
            Some(h) => vec![self
                .repo
                .find_commit(h.target().ok_or_else(|| anyhow!("broken HEAD"))?)?],
            None => vec![],
        };
        let parent_refs: Vec<_> = parents.iter().collect();
        self.repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)?;
        Ok(())
    }

    pub fn staged_files(&self) -> Result<Vec<PathBuf>> {
        let index = self.repo.index()?;
        let mut out = Vec::new();
        for entry in index.iter() {
            if let Ok(path) = std::str::from_utf8(&entry.path) {
                out.push(PathBuf::from(path));
            }
        }
        Ok(out)
    }

    pub fn diff_for_group(&self, group: &crate::splitter::Group) -> Result<String> {
        let head = self.head_tree()?;
        let index = self.repo.index().context("get index")?;

        let mut opts = DiffOptions::new();
        for p in &group.paths {
            opts.pathspec(p);
        }

        let diff = self
            .repo
            .diff_tree_to_index(Some(&head), Some(&index), Some(&mut opts))
            .context("diff for group")?;
        diff_to_string(&diff)
    }

    pub fn has_staged_changes(&self) -> Result<bool> {
        let head = self.head_tree()?;
        let index = self.repo.index()?;
        let diff = self
            .repo
            .diff_tree_to_index(Some(&head), Some(&index), None)?;
        Ok(diff.deltas().count() > 0)
    }

    pub fn undo_last_commit(&self) -> Result<()> {
        let head = self.repo.head().context("no HEAD to undo")?;
        let commit = head.peel_to_commit().context("peel HEAD to commit")?;
        let parent = commit.parent(0).context("no parent commit — first commit cannot be undone with reset")?;
        let obj = parent.as_object().clone();
        self.repo
            .reset(&obj, git2::ResetType::Soft, None)
            .context("undo last commit")?;
        Ok(())
    }

    /// Check if the repository is in a merge, rebase, bisect, or cherry-pick state.
    pub fn is_operation_in_progress(&self) -> bool {
        let path = self.repo.path();
        for marker in &["MERGE_HEAD", "REBASE_HEAD", "BISECT_LOG", "CHERRY_PICK_HEAD"] {
            if path.join(marker).exists() {
                return true;
            }
        }
        false
    }
}

// --- Free functions (thin wrappers using GitRepo::from_current_dir) ---

pub fn open_repo() -> Result<Repository> {
    Ok(GitRepo::from_current_dir()?.repo)
}

pub fn open_repo_at(path: &Path) -> Result<Repository> {
    Ok(GitRepo::from_path(path)?.repo)
}

pub fn staged_diff() -> Result<String> {
    GitRepo::from_current_dir()?.staged_diff()
}

pub fn stage_all() -> Result<()> {
    GitRepo::from_current_dir()?.stage_all()
}

pub fn stage_specific(paths: &[PathBuf]) -> Result<()> {
    GitRepo::from_current_dir()?.stage_specific(paths)
}

pub fn reset_index_to_head() -> Result<()> {
    GitRepo::from_current_dir()?.reset_index_to_head()
}

pub fn commit(message: &str, paths: Option<&[PathBuf]>) -> Result<()> {
    GitRepo::from_current_dir()?.commit(message, paths)
}

pub fn staged_files() -> Result<Vec<PathBuf>> {
    GitRepo::from_current_dir()?.staged_files()
}

pub fn diff_for_group(group: &crate::splitter::Group) -> Result<String> {
    GitRepo::from_current_dir()?.diff_for_group(group)
}

pub fn has_staged_changes() -> Result<bool> {
    GitRepo::from_current_dir()?.has_staged_changes()
}

pub fn ensure_in_repo() -> Result<()> {
    GitRepo::from_current_dir()?;
    Ok(())
}

// --- Private helpers ---

fn diff_to_string(diff: &git2::Diff) -> Result<String> {
    let mut buf = Vec::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        let content = line.content();
        if let Ok(s) = std::str::from_utf8(content) {
            buf.extend_from_slice(s.as_bytes());
        }
        true
    })?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

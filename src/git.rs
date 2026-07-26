#![allow(dead_code)]

use anyhow::{anyhow, bail, Context, Result};
use git2::{DiffFormat, DiffOptions, Repository, Signature};
use std::path::PathBuf;

const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

pub fn open_repo() -> Result<Repository> {
    let cwd = std::env::current_dir().context("get current dir")?;
    Repository::discover(&cwd).context("discover git repository")
}

fn head_tree(repo: &Repository) -> Result<git2::Tree<'_>> {
    if let Ok(head) = repo.head() {
        let target = head.target().ok_or_else(|| anyhow!("HEAD is unborn"))?;
        return repo.find_tree(target).context("resolve head tree");
    }
    let oid = git2::Oid::from_str(EMPTY_TREE)?;
    repo.find_tree(oid)
        .map_err(|e| anyhow!("resolve empty tree: {e}"))
}

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

pub fn staged_diff() -> Result<String> {
    let repo = open_repo()?;
    let head = head_tree(&repo)?;
    let index = repo.index().context("get index")?;
    let mut opts = DiffOptions::new();
    opts.include_untracked(false);
    let diff = repo
        .diff_tree_to_index(Some(&head), Some(&index), Some(&mut opts))
        .context("diff tree to index")?;
    diff_to_string(&diff)
}

pub fn stage_all() -> Result<()> {
    let repo = open_repo()?;
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

pub fn stage_specific(paths: &[PathBuf]) -> Result<()> {
    let repo = open_repo()?;
    let mut index = repo.index()?;
    for p in paths {
        index
            .add_path(p)
            .with_context(|| format!("add {}", p.display()))?;
    }
    index.write()?;
    Ok(())
}

pub fn reset_index_to_head() -> Result<()> {
    let repo = open_repo()?;
    let head = repo.head().and_then(|h| h.peel_to_commit())?;
    repo.reset(head.as_object(), git2::ResetType::Mixed, None)?;
    Ok(())
}

pub fn commit(message: &str, paths: Option<&[PathBuf]>) -> Result<()> {
    let repo = open_repo()?;
    if let Some(ps) = paths {
        stage_specific(ps)?;
    }
    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;
    let sig = Signature::now("aicommit", "aicommit@users.noreply.github.com")
        .context("create signature")?;
    let parents: Vec<git2::Commit<'_>> = match repo.head().ok() {
        Some(h) => vec![repo.find_commit(h.target().ok_or_else(|| anyhow!("broken HEAD"))?)?],
        None => vec![],
    };
    let parent_refs: Vec<_> = parents.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)?;
    Ok(())
}

pub fn staged_files() -> Result<Vec<PathBuf>> {
    let repo = open_repo()?;
    let index = repo.index()?;
    let mut out = Vec::new();
    for entry in index.iter() {
        if let Ok(path) = std::str::from_utf8(&entry.path) {
            out.push(PathBuf::from(path));
        }
    }
    Ok(out)
}

pub fn diff_for_group(group: &crate::splitter::Group) -> Result<String> {
    let repo = open_repo()?;
    let head = head_tree(&repo)?;
    let index = repo.index().context("get index")?;

    let mut opts = DiffOptions::new();
    for p in &group.paths {
        opts.pathspec(p);
    }

    let diff = repo
        .diff_tree_to_index(Some(&head), Some(&index), Some(&mut opts))
        .context("diff for group")?;
    diff_to_string(&diff)
}

pub fn has_staged_changes() -> Result<bool> {
    let repo = open_repo()?;
    let head = head_tree(&repo)?;
    let index = repo.index()?;
    let diff = repo.diff_tree_to_index(Some(&head), Some(&index), None)?;
    Ok(diff.deltas().count() > 0)
}

pub fn ensure_in_repo() -> Result<()> {
    if open_repo().is_err() {
        bail!("not a git repository");
    }
    Ok(())
}

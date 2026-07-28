//! Atomic-commit splitter: groups staged files by logical domain (directory).

use anyhow::Result;
use std::path::{Path, PathBuf};

/// A logical group of files destined for a single atomic commit.
#[derive(Clone, Debug)]
pub struct Group {
    /// Human-readable label used for display ("auth", "db", "root").
    pub name: String,
    /// Repo-relative paths included in this group.
    pub paths: Vec<String>,
    /// Number of files in this group.
    pub file_count: usize,
    /// Total added lines across all files in this group.
    pub insertions: usize,
    /// Total deleted lines across all files in this group.
    pub deletions: usize,
}

/// Decide whether a diff is small enough to be a single commit.
/// We treat any diff with <= 1 changed file (or one logical group) as single.
pub fn should_treat_as_single(diff: &str) -> bool {
    let n = diff
        .lines()
        .filter(|l| l.starts_with("diff --git "))
        .count();
    n <= 1
}

/// Group staged files by their top-level directory.
///
/// Examples:
///   - `src/auth/login.rs` + `src/auth/logout.rs` → group "auth"
///   - `src/db/pool.rs` → group "db"
///   - `README.md` → group "root"
///   - `package.json` → group "root"
///
/// Files matching any glob pattern in `exclude` are skipped.
pub fn group_by_directory(diff: &str, max_groups: usize, exclude: &[String]) -> Result<Vec<Group>> {
    let files: Vec<PathBuf> = diff
        .lines()
        .filter(|l| l.starts_with("diff --git "))
        .filter_map(parse_diff_header_path)
        .filter(|p| !matches_exclude(p, exclude))
        .collect();

    if files.is_empty() {
        return Ok(vec![]);
    }

    // Bucket by top-level directory (or "root" if there is none).
    use std::collections::BTreeMap;
    let mut buckets: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in files {
        let key = group_key(&path);
        buckets
            .entry(key)
            .or_default()
            .push(path.to_string_lossy().into_owned());
    }

    // Compute stats for all files in the diff.
    let stats = compute_stats(diff);

    // If everything falls into one bucket, just one commit.
    if buckets.len() <= 1 {
        let mut paths: Vec<String> = buckets.into_values().flatten().collect();
        paths.sort();
        let (insertions, deletions) = sum_stats(&paths, &stats);
        return Ok(vec![Group {
            name: "root".to_string(),
            file_count: paths.len(),
            paths,
            insertions,
            deletions,
        }]);
    }

    let mut groups: Vec<Group> = buckets
        .into_iter()
        .map(|(name, mut paths)| {
            paths.sort();
            let (insertions, deletions) = sum_stats(&paths, &stats);
            Group {
                name,
                file_count: paths.len(),
                paths,
                insertions,
                deletions,
            }
        })
        .collect();

    // Honor the configured maximum number of atomic commits.
    if groups.len() > max_groups {
        // Merge the smallest tail into "misc".
        groups.sort_by_key(|g| g.paths.len());
        let misc_paths: Vec<String> = groups
            .split_off(max_groups.max(1) - 1)
            .into_iter()
            .flat_map(|g| g.paths)
            .collect();
        let m_insertions: usize = misc_paths.iter().map(|p| stats.get(p.as_str()).map(|s| s.0).unwrap_or(0)).sum();
        let m_deletions: usize = misc_paths.iter().map(|p| stats.get(p.as_str()).map(|s| s.1).unwrap_or(0)).sum();
        groups.push(Group {
            name: "misc".to_string(),
            file_count: misc_paths.len(),
            paths: misc_paths,
            insertions: m_insertions,
            deletions: m_deletions,
        });
    }

    // Reorder so largest groups come first (nicer UX).
    groups.sort_by_key(|b| std::cmp::Reverse(b.paths.len()));
    Ok(groups)
}

/// Extract the "b" side path from a `diff --git a/X b/Y` header.
fn parse_diff_header_path(line: &str) -> Option<PathBuf> {
    let mut it = line.split_whitespace();
    let _ = it.next()?; // "diff"
    let _ = it.next()?; // "git"
    let _ = it.next()?; // "a/X"
    let b = it.next()?; // "b/Y"
    let trimmed = b.trim_start_matches("b/");
    Some(PathBuf::from(trimmed))
}

/// Decide which "name" represents a file: the first subdirectory under the
/// repository root, or "root" for files at the root level.
///
/// Examples:
///   "src/auth/login.rs"     → "auth"
///   "src/db/pool.rs"        → "db"
///   "README.md"             → "root"
///   "lib.rs"                → "root"
///   "src/a/b/c.rs"          → "src"
///   "assets/icons/close.svg" → "icons"
fn group_key(path: &Path) -> String {
    let comps: Vec<_> = path.components().collect();
    if comps.len() < 2 {
        return "root".to_string();
    }
    let second = &comps[1];
    // If there are at least 3 components, the 2nd is a directory.
    // If there are exactly 2, the 2nd is the filename → use the 1st.
    if comps.len() > 2 {
        second.as_os_str().to_string_lossy().into_owned()
    } else {
        comps[0].as_os_str().to_string_lossy().into_owned()
    }
}

/// Parse a unified diff string and return per-file (insertions, deletions).
/// Lines starting with `+` (except `+++`) count as insertions.
/// Lines starting with `-` (except `---`) count as deletions.
fn compute_stats(diff: &str) -> std::collections::HashMap<String, (usize, usize)> {
    let mut map = std::collections::HashMap::new();
    let mut current_file: Option<String> = None;
    let mut ins = 0usize;
    let mut del = 0usize;

    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("diff --git a/").and_then(|s| {
            let mut it = s.split_whitespace();
            let _ = it.next()?;
            let b = it.next()?;
            b.strip_prefix("b/")
        }) {
            if let Some(file) = current_file.take() {
                map.insert(file, (ins, del));
            }
            current_file = Some(path.to_string());
            ins = 0;
            del = 0;
            continue;
        }
        let line = line.trim_start();
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if line.starts_with('+') {
            ins += 1;
        } else if line.starts_with('-') {
            del += 1;
        }
    }
    if let Some(file) = current_file {
        map.insert(file, (ins, del));
    }
    map
}

fn sum_stats(paths: &[String], stats: &std::collections::HashMap<String, (usize, usize)>) -> (usize, usize) {
    paths.iter().fold((0, 0), |(i, d), p| {
        let (si, sd) = stats.get(p.as_str()).copied().unwrap_or((0, 0));
        (i + si, d + sd)
    })
}

/// Check if a path matches any of the exclude glob patterns.
fn matches_exclude(path: &Path, exclude: &[String]) -> bool {
    if exclude.is_empty() {
        return false;
    }
    let path_str = path.to_string_lossy();
    exclude.iter().any(|pat| {
        if let Ok(m) = glob::Pattern::new(pat) {
            m.matches(&path_str)
        } else {
            false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diff_with(paths: &[&str]) -> String {
        paths
            .iter()
            .map(|p| format!("diff --git a/{p} b/{p}\n--- a/{p}\n+++ b/{p}\n@@ -1 +1 @@\n-x\n+y\n"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn single_file_is_single() {
        assert!(should_treat_as_single(&diff_with(&["README.md"])));
    }

    #[test]
    fn multiple_files_are_not_single() {
        assert!(!should_treat_as_single(&diff_with(&["a/x.rs", "b/y.rs"])));
    }

    #[test]
    fn groups_by_top_level_directory() {
        let d = diff_with(&["src/auth/login.rs", "src/auth/logout.rs", "src/db/pool.rs"]);
        let groups = group_by_directory(&d, 5, &[]).unwrap();
        assert_eq!(groups.len(), 2);
        let names: Vec<_> = groups.iter().map(|g| g.name.as_str()).collect();
        assert!(names.contains(&"auth"));
        assert!(names.contains(&"db"));
    }

    #[test]
    fn root_files_group_as_root() {
        let d = diff_with(&["README.md", "package.json"]);
        let groups = group_by_directory(&d, 5, &[]).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "root");
        assert_eq!(groups[0].paths.len(), 2);
    }

    #[test]
    fn caps_at_max_groups() {
        let d = diff_with(&["a/1.rs", "b/2.rs", "c/3.rs", "d/4.rs"]);
        let groups = group_by_directory(&d, 2, &[]).unwrap();
        assert_eq!(groups.len(), 2);
        // The misc group should contain the overflow.
        let has_misc = groups.iter().any(|g| g.name == "misc");
        assert!(has_misc, "expected a misc overflow group");
    }
}

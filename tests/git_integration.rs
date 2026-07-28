use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use aicommit::git::GitRepo;

struct TestRepo {
    _dir: TempDir,
    pub repo: GitRepo,
}

impl TestRepo {
    fn new() -> Self {
        let dir = TempDir::new().expect("create temp dir");
        let raw = git2::Repository::init(dir.path()).expect("init repo");
        let repo = GitRepo { repo: raw };
        Self { _dir: dir, repo }
    }

    fn write_and_stage(&self, name: &str, content: &str) {
        let file_path = self._dir.path().join(name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&file_path, content).unwrap();
        let mut index = self.repo.repo.index().unwrap();
        index.add_path(std::path::Path::new(name)).unwrap();
        index.write().unwrap();
    }

    fn create_initial_commit(&self) {
        let file_path = self._dir.path().join("init.rs");
        fs::write(&file_path, "// initial").unwrap();
        let mut index = self.repo.repo.index().unwrap();
        index.add_path(std::path::Path::new("init.rs")).unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = self.repo.repo.find_tree(tree_oid).unwrap();
        let sig = git2::Signature::now("test", "test@test.com").unwrap();
        self.repo
            .repo
            .commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
            .unwrap();
    }
}

// --- Parallel-safe tests (no env::set_current_dir) ---

#[test]
fn test_gitrepo_from_path_valid() {
    let dir = TempDir::new().unwrap();
    git2::Repository::init(dir.path()).unwrap();
    let repo = GitRepo::from_path(dir.path()).unwrap();
    assert!(!repo.repo.path().as_os_str().is_empty());
}

#[test]
fn test_gitrepo_from_path_invalid() {
    let dir = TempDir::new().unwrap();
    let result = GitRepo::from_path(dir.path());
    assert!(result.is_err());
}

#[test]
fn test_staged_diff_empty_repo_no_head() {
    let t = TestRepo::new();
    t.write_and_stage("hello.rs", "fn main() {}");
    let diff = t.repo.staged_diff().unwrap();
    assert!(
        diff.contains("hello.rs"),
        "diff should mention hello.rs, got:\n{diff}"
    );
    assert!(
        diff.contains("fn main() {}"),
        "diff should show added content, got:\n{diff}"
    );
}

#[test]
fn test_staged_diff_with_head() {
    let t = TestRepo::new();
    t.create_initial_commit();
    t.write_and_stage("main.rs", "fn main() { println!(); }");
    let diff = t.repo.staged_diff().unwrap();
    assert!(
        diff.contains("main.rs"),
        "diff should mention main.rs, got:\n{diff}"
    );
    assert!(
        diff.contains("fn main() { println!(); }"),
        "diff should show added content, got:\n{diff}"
    );
}

#[test]
fn test_staged_diff_no_changes() {
    let t = TestRepo::new();
    let diff = t.repo.staged_diff().unwrap();
    assert_eq!(diff, "", "no changes should produce empty diff");
}

#[test]
fn test_commit_creates_commit() {
    let t = TestRepo::new();
    t.write_and_stage("app.rs", "fn app() {}");
    t.repo.commit("feat: add app", None).unwrap();
    let head = t.repo.repo.head().unwrap();
    let commit = head.peel_to_commit().unwrap();
    let msg = commit.message().unwrap();
    assert_eq!(msg, "feat: add app");
}

#[test]
fn test_commit_initial_no_parent() {
    let t = TestRepo::new();
    t.write_and_stage("first.rs", "// first");
    t.repo.commit("feat: first commit", None).unwrap();
    let head = t.repo.repo.head().unwrap();
    let commit = head.peel_to_commit().unwrap();
    assert_eq!(commit.parent_count(), 0, "initial commit has zero parents");
}

#[test]
fn test_commit_with_parent() {
    let t = TestRepo::new();
    t.create_initial_commit();
    t.write_and_stage("second.rs", "// second");
    t.repo.commit("feat: second commit", None).unwrap();
    let head = t.repo.repo.head().unwrap();
    let commit = head.peel_to_commit().unwrap();
    assert_eq!(commit.parent_count(), 1, "second commit has one parent");
}

#[test]
fn test_stage_specific_only_stages_given_paths() {
    let t = TestRepo::new();
    fs::write(t._dir.path().join("a.rs"), "a").unwrap();
    fs::write(t._dir.path().join("b.rs"), "b").unwrap();
    t.repo.stage_specific(&[PathBuf::from("a.rs")]).unwrap();
    let files = t.repo.staged_files().unwrap();
    assert!(files.contains(&PathBuf::from("a.rs")));
    assert!(!files.contains(&PathBuf::from("b.rs")));
}

#[test]
fn test_has_staged_changes_true() {
    let t = TestRepo::new();
    t.write_and_stage("change.rs", "// changed");
    let has = t.repo.has_staged_changes().unwrap();
    assert!(has);
}

#[test]
fn test_has_staged_changes_false() {
    let t = TestRepo::new();
    let has = t.repo.has_staged_changes().unwrap();
    assert!(!has);
}

#[test]
fn test_staged_files_empty_on_fresh_repo() {
    let t = TestRepo::new();
    let files = t.repo.staged_files().unwrap();
    assert!(files.is_empty());
}

#[test]
fn test_diff_for_group_filters_paths() {
    let t = TestRepo::new();
    t.write_and_stage("src/lib.rs", "pub fn lib() {}");
    t.write_and_stage("src/bin.rs", "fn main() {}");
    let group = aicommit::splitter::Group {
        name: "src".into(),
        paths: vec!["src/lib.rs".into(), "src/bin.rs".into()],
        file_count: 2,
        insertions: 0,
        deletions: 0,
    };
    let diff = t.repo.diff_for_group(&group).unwrap();
    assert!(
        diff.contains("src/lib.rs"),
        "diff missing src/lib.rs:\n{diff}"
    );
    assert!(
        diff.contains("src/bin.rs"),
        "diff missing src/bin.rs:\n{diff}"
    );
}

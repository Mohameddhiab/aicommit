use std::fs;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

use aicommit::config::{load, Config};
use aicommit::git::GitRepo;

fn mock_config() -> Config {
    load(Some("mock".into()), None, None, None, None).expect("mock config should load")
}

#[test]
fn test_mock_config_loads() {
    let cfg = mock_config();
    assert_eq!(cfg.provider.as_str(), "mock");
}

#[tokio::test]
async fn test_full_workflow_stage_then_commit() {
    let dir = TempDir::new().expect("create temp dir");
    let raw = git2::Repository::init(dir.path()).expect("init repo");
    let repo = GitRepo { repo: raw };

    let file_path = dir.path().join("hello.rs");
    fs::write(&file_path, "fn main() {}").unwrap();

    let mut index = repo.repo.index().unwrap();
    index.add_path(std::path::Path::new("hello.rs")).unwrap();
    index.write().unwrap();

    let diff = repo.staged_diff().unwrap();
    assert!(diff.contains("hello.rs"), "staged diff should contain file");

    let cfg = mock_config();
    let provider = aicommit::llm::build_provider(&cfg).expect("build mock provider");
    let msg = aicommit::llm::generate_commit_message(&provider, &diff, &cfg)
        .await
        .expect("generate commit message");

    assert!(!msg.is_empty(), "generated message should not be empty");
    assert!(
        msg.starts_with("feat(test)"),
        "message should start with 'feat(test)', got: {msg}"
    );

    repo.commit(&msg, None).expect("commit should succeed");

    let head = repo.repo.head().expect("head should exist");
    let commit = head.peel_to_commit().expect("peel to commit");
    assert_eq!(
        commit.message().expect("commit message"),
        msg,
        "committed message should match generated message"
    );
}

// --- Tests that change cwd (serialised) ---

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
fn cwd_lock() -> &'static Mutex<()> {
    CWD_LOCK.get_or_init(|| Mutex::new(()))
}

#[tokio::test]
async fn test_workflow_via_free_functions() {
    let _lock = cwd_lock().lock().unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new().expect("create temp dir");
    let repo_handle = git2::Repository::init(dir.path()).expect("init repo");

    let cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    let mut index = repo_handle.index().unwrap();
    index.add_path(std::path::Path::new("main.rs")).unwrap();
    index.write().unwrap();

    let diff = aicommit::git::staged_diff().unwrap();
    assert!(diff.contains("main.rs"), "should detect staged file");

    let provider = aicommit::llm::build_provider(&mock_config()).unwrap();
    let msg = aicommit::llm::generate_commit_message(&provider, &diff, &mock_config())
        .await
        .unwrap();
    assert!(
        msg.starts_with("feat(test)"),
        "mock should generate feat(test), got: {msg}"
    );

    aicommit::git::commit(&msg, None).unwrap();

    let repo_for_head = git2::Repository::open(dir.path()).unwrap();
    let head = repo_for_head.head().unwrap();
    let commit = head.peel_to_commit().unwrap();
    assert_eq!(commit.message().unwrap(), msg);

    std::env::set_current_dir(cwd).ok();
}

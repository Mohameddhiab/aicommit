use anyhow::{Context, Result};
use std::path::Path;

pub fn install_pre_push_hook(git_dir: &Path) -> Result<()> {
    let hooks_dir = git_dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir).context("create hooks directory")?;

    let hook_path = hooks_dir.join("pre-push");
    let content = r#"#!/bin/sh
# git-doctor pre-push hook — check commit quality before pushing
exec doctor check --pre-push
"#
    .to_string();

    std::fs::write(&hook_path, content).context("write pre-push hook")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755))
            .context("make hook executable")?;
    }

    println!("  ✓ Pre-push hook installed at {}", hook_path.display());
    Ok(())
}

pub fn uninstall_hook(git_dir: &Path) -> Result<()> {
    let hook_path = git_dir.join("hooks").join("pre-push");
    if hook_path.exists() {
        std::fs::remove_file(&hook_path).context("remove pre-push hook")?;
        println!("  ✓ Pre-push hook removed");
    } else {
        println!("  ℹ No pre-push hook found");
    }
    Ok(())
}

use crate::git::GitRepo;
use crate::plan::Plan;
use anyhow::Result;

pub struct ApplyResult {
    pub backup_branch: String,
    pub operations_applied: usize,
    pub success: bool,
}

pub fn apply_plan(repo: &GitRepo, plan: &Plan, dry_run: bool, force: bool) -> Result<ApplyResult> {
    let backup_branch = format!("doctor-backup-{}", chrono_now());

    if dry_run {
        println!("  Dry-run: would create backup branch '{}'", backup_branch);
        println!("  Dry-run: would rewrite {} commits across {} operations",
                 plan.commit_count, plan.operation_count);
        for op in &plan.operations {
            println!("  Dry-run:   {}", describe_op(op));
        }
        return Ok(ApplyResult {
            backup_branch,
            operations_applied: 0,
            success: true,
        });
    }

    if !force {
        let has_remote = repo.remote_exists()?;
        if has_remote {
            anyhow::bail!(
                "remote detected — refusing to rewrite pushed commits\n\
                 → use --force to override\n\
                 → or create a local backup branch first"
            );
        }
    }

    repo.create_backup(&backup_branch)?;
    println!("  ✓ Backup branch '{}' created", backup_branch);

    let applied = plan.operations.len();

    Ok(ApplyResult {
        backup_branch,
        operations_applied: applied,
        success: true,
    })
}

fn describe_op(op: &crate::plan::Operation) -> String {
    match op {
        crate::plan::Operation::Squash { target_oid, into_oid, .. } => {
            format!("squash {target_oid} into {into_oid}")
        }
        crate::plan::Operation::Reword { oid, .. } => {
            format!("reword {oid}")
        }
        crate::plan::Operation::Split { oid, .. } => {
            format!("split {oid}")
        }
    }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{}", d.as_secs())
}

use clap::{Parser, Subcommand};
use colored::Colorize;
use git_doctor::{
    analyze, apply, config, display, git, hook, interactive, llm, parser, plan, report, splitter,
};
use std::io::IsTerminal;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "doctor",
    version,
    about = "Git History Doctor — diagnose, fix, and maintain clean git history",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: DoctorCommand,
}

#[derive(Subcommand, Debug)]
pub enum DoctorCommand {
    /// Analyze git history quality and produce a health score.
    Analyze {
        /// Number of recent commits to analyze (default: 10).
        #[arg(long, default_value_t = 10)]
        commits: usize,
        /// Output format: text, json, html, markdown (default: text).
        #[arg(long, default_value_t = String::from("text"))]
        format: String,
        /// Open HTML report in browser.
        #[arg(long)]
        open: bool,
        /// Save report to file.
        #[arg(long)]
        output: Option<String>,
    },
    /// Generate a cleanup plan based on analysis.
    Plan {
        /// Number of recent commits to analyze (default: 10).
        #[arg(long, default_value_t = 10)]
        commits: usize,
        /// Save plan to file.
        #[arg(long)]
        output: Option<String>,
    },
    /// Apply a cleanup plan safely.
    Apply {
        /// Path to plan JSON file.
        plan: String,
        /// Actually apply the plan (default: dry-run preview).
        #[arg(long)]
        confirm: bool,
        /// Skip safety checks (remote detection, in-progress operations).
        #[arg(long)]
        force: bool,
        /// Skip confirmation prompt (CI mode).
        #[arg(long)]
        yes: bool,
    },
    /// Pre-push hook or CI gate to check commit quality.
    Check {
        /// Run as pre-push hook (reads from stdin).
        #[arg(long)]
        pre_push: bool,
    },
    /// Generate a well-structured commit message from staged changes.
    Commit {
        /// Force a single commit (skip atomic split).
        #[arg(short = '1', long)]
        single: bool,
        /// Interactive mode: edit the message before committing.
        #[arg(short, long)]
        interactive: bool,
        /// Dry-run: preview without committing.
        #[arg(short = 'n', long)]
        dry_run: bool,
        /// Disable auto-staging.
        #[arg(long)]
        no_stage: bool,
        /// AI provider override.
        #[arg(short, long)]
        provider: Option<String>,
        /// Model override.
        #[arg(short, long)]
        model: Option<String>,
        /// Language for the commit message.
        #[arg(long)]
        lang: Option<String>,
        /// Ephemeral API key.
        #[arg(long)]
        api_key: Option<String>,
        /// Timeout in seconds (default: 120).
        #[arg(long, default_value_t = 120)]
        timeout: u64,
    },
    /// Manage configuration.
    Config {
        /// Set API key for a provider.
        #[arg(long)]
        api_key: Option<String>,
        /// Provider name.
        #[arg(long)]
        provider: Option<String>,
        /// Set default model.
        #[arg(long)]
        model: Option<String>,
        /// Set base URL.
        #[arg(long)]
        base_url: Option<String>,
        /// Show resolved configuration.
        #[arg(long)]
        show: bool,
    },
    /// Undo the last commit (git reset --soft HEAD~1).
    Undo,
    /// Initialize doctor: install hooks and create config.
    Init,
    /// Uninstall doctor hooks.
    Uninstall,
}

#[tokio::main]
async fn main() {
    if std::env::var("NO_COLOR").is_ok() || !std::io::stdout().is_terminal() {
        colored::control::set_override(false);
    }

    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("{} {:#}", "✗".red().bold(), e);
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        DoctorCommand::Analyze {
            commits,
            format,
            open,
            output,
        } => run_analyze(commits, &format, open, output).await,
        DoctorCommand::Plan { commits, output } => run_plan(commits, output).await,
        DoctorCommand::Apply {
            plan,
            confirm,
            force,
            yes,
        } => {
            let should_apply = confirm || yes;
            run_apply(&plan, should_apply, force)
        }
        DoctorCommand::Check { pre_push } => run_check(pre_push).await,
        DoctorCommand::Commit {
            single,
            interactive,
            dry_run,
            no_stage,
            provider,
            model,
            lang,
            api_key,
            timeout,
        } => {
            run_commit(
                single,
                interactive,
                dry_run,
                no_stage,
                provider,
                model,
                lang,
                api_key,
                timeout,
            )
            .await
        }
        DoctorCommand::Config {
            api_key,
            provider,
            model,
            base_url,
            show,
        } => config::handle_config_command(api_key, provider, model, base_url, show).await,
        DoctorCommand::Undo => {
            let repo = git::GitRepo::from_current_dir()?;

            // Try doctor apply rollback first; fall back to simple undo
            if let Ok(tags) = repo.list_backup_tags() {
                if !tags.is_empty() {
                    apply::undo_last_apply(&repo)?;
                    display::box_start("Undo");
                    display::box_line("Last doctor apply has been rolled back.");
                    display::box_end();
                    return Ok(());
                }
            }

            repo.undo_last_commit()?;
            display::box_start("Undo");
            display::box_line("Last commit has been undone (git reset --soft HEAD~1).");
            display::box_end();
            Ok(())
        }
        DoctorCommand::Init => run_init().await,
        DoctorCommand::Uninstall => run_uninstall().await,
    }
}

async fn run_analyze(
    commits: usize,
    format: &str,
    open: bool,
    output: Option<String>,
) -> anyhow::Result<()> {
    let repo = git::GitRepo::from_current_dir()?;
    let history = repo.walk_history(commits)?;

    let scores: Vec<analyze::CommitScore> = history
        .iter()
        .map(
            |(oid, subject, author, body, files, insertions, deletions)| {
                let domains = vec!["root".to_string()];
                analyze::score_commit(
                    oid,
                    subject,
                    author,
                    body,
                    *files,
                    *insertions,
                    *deletions,
                    &domains,
                )
            },
        )
        .collect();

    let report = analyze::build_report(scores);

    let content: String = match format {
        "json" => report::format_json(&report),
        "html" => {
            let html = report::generate_html(&report);
            if open {
                let path = std::env::temp_dir().join("doctor-report.html");
                std::fs::write(&path, &html)?;
                open::that(&path).ok();
                println!("  Report opened in browser");
            }
            html
        }
        "markdown" => {
            let mut md = report::format_markdown(&report);
            md.push('\n');
            md.push_str(&report::format_per_commit_markdown(&report.commits));
            md
        }
        _ => report::format_text(&report),
    };

    match output {
        Some(path) => {
            std::fs::write(&path, &content)?;
            println!("  Report saved to {path}");
        }
        None => println!("{content}"),
    }

    Ok(())
}

async fn run_plan(commits: usize, output: Option<String>) -> anyhow::Result<()> {
    let repo = git::GitRepo::from_current_dir()?;
    let history = repo.walk_history(commits)?;

    let domains = vec!["root".to_string()];
    let scores: Vec<analyze::CommitScore> = history
        .iter()
        .map(
            |(oid, subject, author, body, files, insertions, deletions)| {
                analyze::score_commit(
                    oid,
                    subject,
                    author,
                    body,
                    *files,
                    *insertions,
                    *deletions,
                    &domains,
                )
            },
        )
        .collect();

    let report = analyze::build_report(scores);
    let plan = plan::generate_plan(&report);

    match output {
        Some(ref path) => {
            let json = serde_json::to_string_pretty(&plan)?;
            std::fs::write(path, &json)?;
            println!("  Plan saved to {path}");
        }
        None => {
            println!("{}", plan::format_plan_text(&plan));
        }
    }

    Ok(())
}

fn run_apply(plan_path: &str, confirm: bool, force: bool) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(plan_path)?;
    let plan: plan::Plan = serde_json::from_str(&content)?;
    let repo = git::GitRepo::from_current_dir()?;

    let result = apply::apply_plan(&repo, &plan, confirm, force)?;

    if confirm {
        println!(
            "  ✓ Applied {} operations (backup: {})",
            result.operations_applied, result.backup_tag
        );
    }
    Ok(())
}

async fn run_check(pre_push: bool) -> anyhow::Result<()> {
    if pre_push {
        let repo = git::GitRepo::from_current_dir()?;
        let history = repo.walk_history(3)?;
        let domains = vec!["root".to_string()];
        let scores: Vec<analyze::CommitScore> = history
            .iter()
            .map(
                |(oid, subject, author, body, files, insertions, deletions)| {
                    analyze::score_commit(
                        oid,
                        subject,
                        author,
                        body,
                        *files,
                        *insertions,
                        *deletions,
                        &domains,
                    )
                },
            )
            .collect();

        let mut blocked = 0;
        for s in &scores {
            if s.is_wip || s.is_vague {
                println!(
                    "  ✗ {} — \"{}\" ({})",
                    &s.oid[..7],
                    s.subject,
                    if s.is_wip { "wip" } else { "vague" }
                );
                blocked += 1;
            } else {
                println!("  ✓ {} — \"{}\"", &s.oid[..7], s.subject);
            }
        }

        if blocked > 0 {
            anyhow::bail!("{blocked} commits blocked. Use --force to override.");
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_commit(
    single: bool,
    interactive: bool,
    dry_run: bool,
    no_stage: bool,
    provider: Option<String>,
    model: Option<String>,
    lang: Option<String>,
    api_key: Option<String>,
    timeout: u64,
) -> anyhow::Result<()> {
    let cfg = config::load(
        provider.clone(),
        model.clone(),
        lang.clone(),
        api_key.clone(),
        None,
        timeout,
    )?;
    let repo = git::GitRepo::from_current_dir()?;

    if repo.is_operation_in_progress() {
        anyhow::bail!("a merge, rebase, bisect, or cherry-pick is in progress");
    }

    let mut diff = repo.staged_diff()?;
    if diff.is_empty() && !no_stage {
        repo.stage_all()?;
        diff = repo.staged_diff()?;
    }
    if diff.is_empty() {
        display::box_start("Info");
        display::box_line("Nothing staged. Make changes and `git add` first.");
        display::box_end();
        return Ok(());
    }

    let provider = llm::build_provider(&cfg)?;
    let line_count = diff.lines().count();
    display::box_start(&format!(
        "{} — {} lines to analyze",
        provider.name(),
        line_count
    ));

    if single || splitter::should_treat_as_single(&diff) {
        let msg = llm::generate_commit_message(&provider, &diff, &cfg).await?;
        let parsed = parser::parse_commit_message(&msg)?;
        let formatted = parsed.format_with_template(&cfg.commit_template);
        let final_msg = if interactive {
            interactive::edit_message(&formatted)?
        } else {
            formatted
        };
        do_commit(&repo, &final_msg, None, dry_run)?;
    } else {
        let groups =
            splitter::group_by_directory(&diff, cfg.commit_max_commits, &cfg.exclude_patterns)?;
        if interactive {
            let mut msgs = Vec::new();
            for group in &groups {
                let gdiff = repo.diff_for_group(group)?;
                display::box_line(&format!(
                    "◉ Group '{}' — {} lines",
                    group.name,
                    gdiff.lines().count()
                ));
                let msg = llm::generate_commit_message(&provider, &gdiff, &cfg).await?;
                msgs.push(msg);
            }
            display::box_end();
            match interactive::select_commits(&groups, &msgs) {
                interactive::SelectResult::Some(selected) => {
                    for &idx in &selected {
                        let parsed = parser::parse_commit_message(&msgs[idx])?;
                        let final_msg = interactive::edit_message(
                            &parsed.format_with_template(&cfg.commit_template),
                        )?;
                        let paths: Vec<PathBuf> =
                            groups[idx].paths.iter().map(PathBuf::from).collect();
                        do_commit(&repo, &final_msg, Some(&paths), dry_run)?;
                    }
                }
                interactive::SelectResult::None | interactive::SelectResult::Cancelled => {
                    display::box_line("No commits selected — exiting.");
                }
            }
        } else {
            for group in &groups {
                let gdiff = repo.diff_for_group(group)?;
                display::box_line(&format!(
                    "◉ Group '{}' — {} lines",
                    group.name,
                    gdiff.lines().count()
                ));
                let msg = llm::generate_commit_message(&provider, &gdiff, &cfg).await?;
                let parsed = parser::parse_commit_message(&msg)?;
                let final_msg = parsed.format_with_template(&cfg.commit_template);
                let paths: Vec<PathBuf> = group.paths.iter().map(PathBuf::from).collect();
                do_commit(&repo, &final_msg, Some(&paths), dry_run)?;
            }
        }
    }
    display::box_end();
    Ok(())
}

fn do_commit(
    repo: &git::GitRepo,
    message: &str,
    paths: Option<&[PathBuf]>,
    dry_run: bool,
) -> anyhow::Result<()> {
    if dry_run {
        println!("  {} Would commit: {message}", "✔".green());
    } else {
        repo.commit(message, paths)?;
        println!("  {} Committed: {message}", "✔".green());
    }
    Ok(())
}

async fn run_init() -> anyhow::Result<()> {
    let repo = git::GitRepo::from_current_dir()?;
    let git_dir = repo.git_dir();

    display::box_start("doctor init");
    display::box_line("Setting up git-doctor...");
    display::box_end();

    hook::install_pre_push_hook(&git_dir)?;
    println!("  ✓ git-doctor initialized");
    Ok(())
}

async fn run_uninstall() -> anyhow::Result<()> {
    let repo = git::GitRepo::from_current_dir()?;
    let git_dir = repo.git_dir();
    hook::uninstall_hook(&git_dir)?;
    println!("  ✓ git-doctor uninstalled");
    Ok(())
}

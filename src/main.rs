use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use git_doctor::{
    analyze, apply, banner, config, display, git, hook, interactive, llm, plan, report, review,
    splitter,
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
    #[command(alias = "a", alias = "diag")]
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
    /// Audit staged code for security vulnerabilities (leaked keys/tokens) & debug leftovers.
    #[command(alias = "r", alias = "audit")]
    Review,
    /// Generate a cleanup plan based on analysis.
    #[command(alias = "p")]
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
    #[command(alias = "c", alias = "ci")]
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
    /// Generate shell autocompletion script (bash, zsh, fish, powershell, elvish).
    Completions {
        /// Target shell
        #[arg(value_enum)]
        shell: Shell,
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
        DoctorCommand::Review => run_review().await,
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
        DoctorCommand::Completions { shell } => {
            run_completions(shell);
            Ok(())
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
    let spinner = display::create_spinner("Analyzing git repository history and commit scores...");
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
    spinner.finish_and_clear();

    let content: String = match format {
        "json" => report::format_json(&report),
        "html" => {
            let html = report::generate_html(&report);
            if open {
                let path = std::env::temp_dir().join("doctor-report.html");
                std::fs::write(&path, &html)?;
                open::that(&path).ok();
                println!("  ✓ Interactive Health Dashboard opened in browser ({})", path.display());
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
            println!("  ✓ Health report saved to {path}");
        }
        None => println!("{content}"),
    }

    Ok(())
}

async fn run_review() -> anyhow::Result<()> {
    let spinner = display::create_spinner("Scanning staged changes for security leaks & debug artifacts...");
    let repo = git::GitRepo::from_current_dir()?;
    let diff = repo.staged_diff()?;
    spinner.finish_and_clear();

    if diff.trim().is_empty() {
        println!("  ℹ No staged changes found to review. Run `git add` first.");
        return Ok(());
    }

    let report = review::run_code_review(&diff);
    println!("{}", review::render_review_cli(&report));

    if !report.passed {
        anyhow::bail!("Code review failed due to critical security issues.");
    }
    Ok(())
}

fn run_completions(shell: Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "doctor", &mut std::io::stdout());
}

async fn run_plan(commits: usize, output: Option<String>) -> anyhow::Result<()> {
    let spinner = display::create_spinner("Generating git history optimization plan...");
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
    spinner.finish_and_clear();

    match output {
        Some(ref path) => {
            let json = serde_json::to_string_pretty(&plan)?;
            std::fs::write(path, &json)?;
            println!("  ✓ Cleanup plan saved to {path}");
        }
        None => {
            println!("{}", plan::format_plan_text(&plan));
        }
    }

    Ok(())
}

fn run_apply(plan_path: &str, should_apply: bool, force: bool) -> anyhow::Result<()> {
    let repo = git::GitRepo::from_current_dir()?;
    let json = std::fs::read_to_string(plan_path)?;
    let plan: plan::Plan = serde_json::from_str(&json)?;

    if !should_apply {
        println!("Dry-run mode (use --confirm to execute):\n");
        println!("{}", plan::format_plan_text(&plan));
        return Ok(());
    }

    apply::apply_plan(&repo, &plan, should_apply, force)?;
    display::box_start("Success");
    display::box_line("Plan applied successfully!");
    display::box_end();

    Ok(())
}

async fn run_check(pre_push: bool) -> anyhow::Result<()> {
    let repo = git::GitRepo::from_current_dir()?;

    if pre_push {
        let history = repo.walk_history(5)?;
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
                        &["root".to_string()],
                    )
                },
            )
            .collect();
        let report = analyze::build_report(scores);

        let has_wip = report.commits.iter().any(|c| c.is_wip);
        if has_wip {
            eprintln!("{} Blocked push: WIP commits detected in recent history.", "✗".red().bold());
            eprintln!("  Run `doctor analyze` for details or `doctor plan` to resolve.");
            std::process::exit(1);
        }
        println!("{} Pre-push check passed.", "✓".green().bold());
        return Ok(());
    }

    let history = repo.walk_history(10)?;
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
                    &["root".to_string()],
                )
            },
        )
        .collect();
    let report = analyze::build_report(scores);
    println!("{}", report::format_text(&report));

    if report.overall_score < 60 {
        anyhow::bail!("History quality score is below threshold (60/100).");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_commit(
    single: bool,
    interactive: bool,
    dry_run: bool,
    no_stage: bool,
    provider_override: Option<String>,
    model_override: Option<String>,
    lang_override: Option<String>,
    api_key_override: Option<String>,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    let repo = git::GitRepo::from_current_dir()?;

    if !no_stage {
        let spinner = display::create_spinner("Staging updated files...");
        repo.stage_all()?;
        spinner.finish_and_clear();
    }

    if !repo.has_staged_changes()? {
        println!("  ℹ No staged changes found. Use `git add` or run without `--no-stage`.");
        return Ok(());
    }

    let conf = config::load(
        provider_override,
        model_override,
        lang_override,
        api_key_override,
        None,
        timeout_secs,
    )?;

    let provider = llm::build_provider(&conf)?;

    let spinner = display::create_spinner("Grouping staged changes into atomic commits...");
    let diff = repo.staged_diff()?;
    let groups = if single {
        let files = repo.staged_files()?;
        vec![splitter::Group {
            name: "all".to_string(),
            paths: files.iter().map(|p| p.to_string_lossy().to_string()).collect(),
            file_count: files.len(),
            insertions: 0,
            deletions: 0,
        }]
    } else {
        splitter::group_by_directory(&diff, 5, &[])?
    };
    spinner.finish_and_clear();

    banner::maybe_print();

    let mut msgs = Vec::new();
    for group in &groups {
        let group_spinner = display::create_spinner(&format!(
            "Generating AI commit message for group '{}' ({} provider)...",
            group.name,
            provider.name()
        ));
        let group_diff = repo.diff_for_group(group)?;
        let msg = llm::generate_commit_message(&provider, &group_diff, &conf).await?;

        group_spinner.finish_and_clear();
        msgs.push(msg);
    }

    if interactive {
        match interactive::select_commits(&groups, &msgs) {
            interactive::SelectResult::Some(selected_indices) => {
                for &idx in &selected_indices {
                    let group = &groups[idx];
                    let mut msg = msgs[idx].clone();
                    msg = interactive::edit_message(&msg)?;

                    if dry_run {
                        println!("  Dry-run: Would commit group '{}' with: {msg}", group.name);
                    } else {
                        let group_paths: Vec<PathBuf> = group.paths.iter().map(PathBuf::from).collect();
                        repo.commit(&msg, Some(&group_paths))?;
                        println!("  ✓ Committed group '{}': {msg}", group.name);
                    }
                }
            }
            interactive::SelectResult::None => {
                println!("  No commits selected.");
            }
            interactive::SelectResult::Cancelled => {
                println!("  Commit operation cancelled.");
            }
        }
    } else {
        for (group, msg) in groups.iter().zip(msgs.iter()) {
            if dry_run {
                println!("  Dry-run: Would commit group '{}' with: {msg}", group.name);
            } else {
                let group_paths: Vec<PathBuf> = group.paths.iter().map(PathBuf::from).collect();
                repo.commit(msg, Some(&group_paths))?;
                println!("  ✓ Committed group '{}': {msg}", group.name);
            }
        }
    }

    Ok(())
}

async fn run_init() -> anyhow::Result<()> {
    let repo = git::GitRepo::from_current_dir()?;
    let spinner = display::create_spinner("Installing doctor git hooks and generating config...");
    hook::install_pre_push_hook(repo.repo.path())?;
    spinner.finish_and_clear();

    display::box_start("Init Complete");
    display::box_line("Git Doctor initialized successfully!");
    display::box_line("• Pre-push hook installed (.git/hooks/pre-push)");
    display::box_end();

    Ok(())
}

async fn run_uninstall() -> anyhow::Result<()> {
    let repo = git::GitRepo::from_current_dir()?;
    let spinner = display::create_spinner("Removing doctor git hooks...");
    hook::uninstall_hook(repo.repo.path())?;
    spinner.finish_and_clear();

    display::box_start("Uninstall Complete");
    display::box_line("Doctor hooks uninstalled.");
    display::box_end();

    Ok(())
}

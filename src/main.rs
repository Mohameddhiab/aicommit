//! aicommit — Generate perfect, atomic Git commits using AI.
//!
//! Entry point: parses the CLI (clap), loads config, dispatches to the
//! provider-agnostic workflow defined in `main.rs`.

use aicommit::{banner, config, display, git, interactive, llm, parser, splitter};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "aicommit",
    version,
    about = "Generate perfect, atomic Git commits using AI — fast, local, reliable.",
    long_about = None
)]
pub struct Cli {
    /// Force a single commit (skip atomic split).
    #[arg(short = '1', long)]
    pub single: bool,

    /// Interactive mode: edit the message before committing.
    #[arg(short, long)]
    pub interactive: bool,

    /// Dry-run: generate and print the message(s) without committing.
    #[arg(short = 'n', long, alias = "print-only")]
    pub dry_run: bool,

    /// Disable auto-staging (aicommit stages all changes by default).
    #[arg(long)]
    pub no_stage: bool,

    /// AI provider to use. Auto-detected if omitted.
    #[arg(short, long)]
    pub provider: Option<String>,

    /// Model override (e.g. "llama3", "gpt-4o").
    #[arg(short, long)]
    pub model: Option<String>,

    /// Base URL for the provider API (e.g. http://localhost:11434,
    /// http://localhost:8000/v1 for OpenAI-compatible local servers).
    #[arg(long)]
    pub base_url: Option<String>,

    /// Language of the generated commit message (e.g. "en", "fr").
    #[arg(long)]
    pub lang: Option<String>,

    /// Ephemeral API key. Prefer env vars or config file instead.
    #[arg(long)]
    pub api_key: Option<String>,

    /// Timeout in seconds for API calls (default: 120).
    #[arg(long, default_value_t = 120)]
    pub timeout: u64,

    /// List available models from the selected provider and exit.
    #[arg(long)]
    pub list_models: bool,

    /// Subcommand (e.g. `aicommit config`).
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// View or modify configuration.
    Config {
        /// Set the API key for a provider.
        #[arg(long)]
        api_key: Option<String>,

        /// Provider the key or model belongs to.
        #[arg(long)]
        provider: Option<String>,

        /// Set the default model for a provider.
        #[arg(long)]
        model: Option<String>,

        /// Set the base URL for a provider.
        #[arg(long)]
        base_url: Option<String>,

        /// Print the resolved configuration (merged env + file + CLI).
        #[arg(long)]
        show: bool,
    },
}

#[tokio::main]
async fn main() {
    banner::maybe_print();
    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        print_error(&e);
        std::process::exit(1);
    }
}

fn print_error(err: &anyhow::Error) {
    display::box_start("Error");
    let msg = format!("{err:#}");
    for line in msg.lines() {
        let colored = if line.contains("Caused by:") {
            format!("  {}", line.dimmed())
        } else {
            format!("  {}", line)
        };
        display::box_line(&colored);
    }
    display::box_end();
}

/// Dispatch entry. Separated from `main` for testability.
async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Some(Command::Config {
            api_key,
            provider,
            model,
            base_url,
            show,
        }) => config::handle_config_command(api_key, provider, model, base_url, show).await,
        None => {
            if cli.list_models {
                return list_models_workflow(cli).await;
            }
            commit_workflow(cli).await
        }
    }
}

/// List available models from the provider and exit.
async fn list_models_workflow(cli: Cli) -> anyhow::Result<()> {
    let cfg = config::load(
        cli.provider.clone(),
        cli.model.clone(),
        cli.lang.clone(),
        cli.api_key.clone(),
        cli.base_url.clone(),
        cli.timeout,
    )?;
    let provider = llm::build_provider(&cfg)?;
    let models = provider.list_models().await?;
    display::box_start(&format!("{} — Available models", provider.name()));
    for m in &models {
        display::box_line(&format!("◉ {m}"));
    }
    display::box_end();
    Ok(())
}

/// Default workflow:
/// 1. Load & merge config (env > file > defaults).
/// 2. Optionally auto-stage all changes.
/// 3. Read staged diff via git2.
/// 4. (optionally) split into logical groups.
/// 5. For each group: build prompt, call provider, parse message, commit (unless dry-run).
async fn commit_workflow(cli: Cli) -> anyhow::Result<()> {
    let cfg = config::load(
        cli.provider.clone(),
        cli.model.clone(),
        cli.lang.clone(),
        cli.api_key.clone(),
        cli.base_url.clone(),
        cli.timeout,
    )?;

    let mut diff = git::staged_diff()?;
    if diff.is_empty() && !cli.no_stage {
        git::stage_all()?;
        diff = git::staged_diff()?;
    }
    if diff.is_empty() {
        display::box_start("Info");
        display::box_line("Nothing staged. Make some changes and `git add` first.");
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

    if cli.single || splitter::should_treat_as_single(&diff) {
        let msg = llm::generate_commit_message(&provider, &diff, &cfg).await?;
        let parsed = parser::parse_commit_message(&msg)?;
        let final_msg = if cli.interactive {
            interactive::edit_message(&parsed.to_conventional())?
        } else {
            parsed.to_conventional()
        };
        do_commit(&final_msg, None, cli.dry_run)?;
        display::box_end();
        return Ok(());
    }

    let groups = splitter::group_by_directory(&diff, cfg.commit_max_commits)?;
    if cli.interactive {
        let mut msgs = Vec::new();
        for group in &groups {
            let gdiff = git::diff_for_group(group)?;
            let glines = gdiff.lines().count();
            display::box_line(&format!("◉ Group '{}' — {glines} lines", group.name));
            let msg = llm::generate_commit_message(&provider, &gdiff, &cfg).await?;
            msgs.push(msg);
        }
        display::box_end();
        let selected = interactive::select_commits(&groups, &msgs);
        for &idx in &selected {
            let parsed = parser::parse_commit_message(&msgs[idx])?;
            let final_msg = interactive::edit_message(&parsed.to_conventional())?;
            let commit_paths: Vec<PathBuf> = groups[idx].paths.iter().map(PathBuf::from).collect();
            do_commit(&final_msg, Some(&commit_paths), cli.dry_run)?;
        }
    } else {
        for group in &groups {
            let gdiff = git::diff_for_group(group)?;
            let glines = gdiff.lines().count();
            display::box_line(&format!("◉ Group '{}' — {glines} lines", group.name));
            let msg = llm::generate_commit_message(&provider, &gdiff, &cfg).await?;
            let parsed = parser::parse_commit_message(&msg)?;
            let final_msg = parsed.to_conventional();
            let commit_paths: Vec<PathBuf> = group.paths.iter().map(PathBuf::from).collect();
            do_commit(&final_msg, Some(&commit_paths), cli.dry_run)?;
        }
        display::box_end();
    }
    Ok(())
}

fn do_commit(message: &str, paths: Option<&[PathBuf]>, dry_run: bool) -> anyhow::Result<()> {
    if dry_run {
        display::box_line(&format!("{} Would commit: {message}", "✔".green()));
    } else {
        git::commit(message, paths)?;
        display::box_line(&format!("{} Committed: {message}", "✔".green()));
    }
    Ok(())
}

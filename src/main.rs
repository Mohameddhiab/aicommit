//! aicommit — Generate perfect, atomic Git commits using AI.
//!
//! Entry point: parses the CLI (clap), loads config, dispatches to the
//! provider-agnostic workflow defined in `main.rs`.

use aicommit::{config, git, interactive, llm, parser, splitter};
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
    /// One of: ollama, openai, anthropic, groq, deepseek, mistral, gemini, openrouter.
    #[arg(short, long, value_parser = ["mock", "ollama", "openai", "anthropic", "groq", "deepseek", "mistral", "gemini", "openrouter"])]
    pub provider: Option<String>,

    /// Model override (e.g. "llama3", "gpt-4o").
    #[arg(short, long)]
    pub model: Option<String>,

    /// Language of the generated commit message (e.g. "en", "fr").
    #[arg(long)]
    pub lang: Option<String>,

    /// Ephemeral API key. Prefer env vars or config file instead.
    #[arg(long)]
    pub api_key: Option<String>,

    /// Subcommand (e.g. `aicommit config`).
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// View or modify configuration.
    Config {
        /// Set the API key for a provider (writes to ~/.aicommit.toml).
        #[arg(long)]
        api_key: Option<String>,

        /// Provider the key belongs to.
        #[arg(long, value_parser = ["mock", "ollama", "openai", "anthropic", "groq", "deepseek", "mistral", "gemini", "openrouter"])]
        provider: Option<String>,

        /// Print the resolved configuration (merged env + file + CLI).
        #[arg(long)]
        show: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        print_error(&e);
        std::process::exit(1);
    }
}

fn print_error(err: &anyhow::Error) {
    eprintln!("{} {}", "✗".red().bold(), "aicommit error".red().bold());
    let msg = format!("{err:#}");
    for line in msg.lines() {
        if line.contains("Caused by:") {
            eprintln!("  {}", line.dimmed());
        } else {
            eprintln!("  {line}");
        }
    }
    eprintln!("\n  {} Run `aicommit --help` for usage.", "?".blue().bold());
}

/// Dispatch entry. Separated from `main` for testability.
async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Some(Command::Config {
            api_key,
            provider,
            show,
        }) => config::handle_config_command(api_key, provider, show).await,
        None => commit_workflow(cli).await,
    }
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
    )?;

    let mut diff = git::staged_diff()?;
    if diff.is_empty() && !cli.no_stage {
        git::stage_all()?;
        diff = git::staged_diff()?;
    }
    if diff.is_empty() {
        println!(
            "{} Nothing staged. Make some changes and `git add` first.",
            "●".yellow()
        );
        return Ok(());
    }

    let provider = llm::build_provider(&cfg)?;
    let line_count = diff.lines().count();
    println!(
        "{} {} — {} lines to analyze",
        "▶".cyan(),
        provider.name(),
        line_count
    );

    if cli.single || splitter::should_treat_as_single(&diff) {
        let msg = llm::generate_commit_message(&provider, &diff, &cfg).await?;
        let parsed = parser::parse_commit_message(&msg)?;
        let final_msg = if cli.interactive {
            interactive::edit_message(&parsed.to_conventional())?
        } else {
            parsed.to_conventional()
        };
        do_commit(&final_msg, None, cli.dry_run)?;
        return Ok(());
    }

    let groups = splitter::group_by_directory(&diff, cfg.commit.max_commits)?;
    if cli.interactive {
        let mut msgs = Vec::new();
        for group in &groups {
            let gdiff = git::diff_for_group(group)?;
            let glines = gdiff.lines().count();
            println!(
                "  {} Group '{}' — {glines} lines to analyze",
                "▸".cyan(),
                group.name
            );
            let msg = llm::generate_commit_message(&provider, &gdiff, &cfg).await?;
            msgs.push(msg);
        }
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
            println!(
                "  {} Group '{}' — {glines} lines to analyze",
                "▸".cyan(),
                group.name
            );
            let msg = llm::generate_commit_message(&provider, &gdiff, &cfg).await?;
            let parsed = parser::parse_commit_message(&msg)?;
            let final_msg = parsed.to_conventional();
            let commit_paths: Vec<PathBuf> = group.paths.iter().map(PathBuf::from).collect();
            do_commit(&final_msg, Some(&commit_paths), cli.dry_run)?;
        }
    }
    Ok(())
}

fn do_commit(message: &str, paths: Option<&[PathBuf]>, dry_run: bool) -> anyhow::Result<()> {
    if dry_run {
        println!("{} Would commit: {message}", "✔".green());
    } else {
        git::commit(message, paths)?;
        println!("{} Committed: {message}", "✔".green());
    }
    Ok(())
}

//! aicommit — Generate perfect, atomic Git commits using AI.
//!
//! Entry point: parses the CLI (clap), loads config, dispatches to the
//! provider-agnostic workflow defined in `main.rs`.

mod config;
mod git;
mod llm;
mod parser;
mod prompt;
mod splitter;

use clap::{Parser, Subcommand};
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

    /// AI provider to use. Auto-detected if omitted.
    /// One of: ollama, openai, anthropic, groq, deepseek, mistral, gemini, openrouter.
    #[arg(short, long, value_parser = ["ollama", "openai", "anthropic", "groq", "deepseek", "mistral", "gemini", "openrouter"])]
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
        #[arg(long, value_parser = ["ollama", "openai", "anthropic", "groq", "deepseek", "mistral", "gemini", "openrouter"])]
        provider: Option<String>,

        /// Print the resolved configuration (merged env + file + CLI).
        #[arg(long)]
        show: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    run(cli).await
}

/// Dispatch entry. Separated from `main` for testability.
async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Some(Command::Config {
            api_key,
            provider,
            show,
        }) => config::handle_config_command(api_key, provider, show).await,
        None => crate::commit_workflow(cli).await,
    }
}

/// Default workflow:
/// 1. Load & merge config (env > file > defaults).
/// 2. Read staged diff via git2.
/// 3. (optionally) split into logical groups.
/// 4. For each group: build prompt, call provider, parse message, commit.
async fn commit_workflow(cli: Cli) -> anyhow::Result<()> {
    let cfg = config::load(
        cli.provider.clone(),
        cli.model.clone(),
        cli.lang.clone(),
        cli.api_key.clone(),
    )?;
    let diff = git::staged_diff()?;
    if diff.is_empty() {
        println!("Nothing staged. Use `git add` first.");
        return Ok(());
    }

    let provider = llm::build_provider(&cfg)?;
    println!("Using provider: {}", provider.name());

    if cli.single || splitter::should_treat_as_single(&diff) {
        let msg = llm::generate_commit_message(&provider, &diff, &cfg).await?;
        let parsed = parser::parse_commit_message(&msg)?;
        let final_msg = if cli.interactive {
            parser::interactive_edit(&parsed)?
        } else {
            parsed.to_conventional()
        };
        git::commit(&final_msg, None)?;
        println!("Committed: {final_msg}");
        return Ok(());
    }

    let groups = splitter::group_by_directory(&diff, cfg.commit.max_commits)?;
    for group in &groups {
        let gdiff = git::diff_for_group(group)?;
        let msg = llm::generate_commit_message(&provider, &gdiff, &cfg).await?;
        let parsed = parser::parse_commit_message(&msg)?;
        let final_msg = if cli.interactive {
            parser::interactive_edit(&parsed)?
        } else {
            parsed.to_conventional()
        };
        let commit_paths: Vec<PathBuf> = group.paths.iter().map(PathBuf::from).collect();
        git::commit(&final_msg, Some(&commit_paths))?;
        println!("Committed: {final_msg}");
    }
    Ok(())
}

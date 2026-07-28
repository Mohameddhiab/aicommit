//! First-launch banner with ASCII logo.

use colored::Colorize;
use std::path::PathBuf;

const BANNER_HEAD: &str = r"
╭──────────────────────────────────────────╮
│  █████╗ ██╗ ██████╗ ██████╗ ███╗   ███╗ │
│ ██╔══██╗██║██╔════╝██╔═══██╗████╗ ████║ │
│ ███████║██║██║     ██║   ██║██╔████╔██║ │
│ ██╔══██║██║██║     ██║   ██║██║╚██╔╝██║ │
│ ██║  ██║██║╚██████╗╚██████╔╝██║ ╚═╝ ██║ │
│ ╚═╝  ╚═╝╚═╝ ╚═════╝ ╚═════╝ ╚═╝     ╚═╝ │";

fn sentinel_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("aicommit")
        .join(".banner-shown")
}

fn was_shown() -> bool {
    sentinel_path().exists()
}

fn mark_shown() {
    if let Some(parent) = sentinel_path().parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(sentinel_path(), "").ok();
}

pub fn maybe_print() {
    if was_shown() {
        return;
    }
    let version = env!("CARGO_PKG_VERSION");
    for line in BANNER_HEAD.lines() {
        println!("{}", line.cyan());
    }
    println!(
        "{}",
        format!("│  v{version} — AI-powered Git commits         │").cyan()
    );
    println!("{}", "╰──────────────────────────────────────────╯".cyan());
    println!();
    mark_shown();
}

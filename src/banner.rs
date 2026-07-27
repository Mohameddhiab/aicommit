//! First-launch banner with ASCII logo.

use colored::Colorize;
use std::path::PathBuf;

const BANNER: &str = r"
╭──────────────────────────────────────────╮
│  █████╗ ██╗ ██████╗ ██████╗ ███╗   ███╗ │
│ ██╔══██╗██║██╔════╝██╔═══██╗████╗ ████║ │
│ ███████║██║██║     ██║   ██║██╔████╔██║ │
│ ██╔══██║██║██║     ██║   ██║██║╚██╔╝██║ │
│ ██║  ██║██║╚██████╗╚██████╔╝██║ ╚═╝ ██║ │
│ ╚═╝  ╚═╝╚═╝ ╚═════╝ ╚═════╝ ╚═╝     ╚═╝ │
│  v0.2.0 — AI-powered Git commits         │
╰──────────────────────────────────────────╯";

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
    for line in BANNER.lines() {
        println!("{}", line.cyan());
    }
    println!();
    mark_shown();
}

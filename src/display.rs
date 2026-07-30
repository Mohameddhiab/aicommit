use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use unicode_width::UnicodeWidthStr;

const MIN_WIDTH: usize = 40;
const MAX_WIDTH: usize = 80;

fn term_width() -> usize {
    terminal_size::terminal_size()
        .map(|(w, _)| (w.0 as usize).saturating_sub(2).clamp(MIN_WIDTH, MAX_WIDTH))
        .unwrap_or(MAX_WIDTH)
}

pub fn box_start(title: &str) {
    let w = term_width();
    let title_w = title.width();
    let prefix = "╭─ ";
    let dash_count = w.saturating_sub(2 + title_w);
    let line = format!(
        "{}{}{}",
        format!("{prefix}{title}").cyan().bold(),
        "─".repeat(dash_count).cyan().bold(),
        "╮".cyan().bold()
    );
    println!("{line}");
}

pub fn box_line(content: &str) {
    let w = term_width();
    let inner = content.width();
    let pad = w.saturating_sub(4 + inner);
    println!("{}{}{}│", "│  ".cyan().bold(), content, " ".repeat(pad));
}

pub fn box_end() {
    let w = term_width();
    println!(
        "{}{}{}",
        "╰".cyan().bold(),
        "─".repeat(w).cyan().bold(),
        "╯".cyan().bold()
    );
}

/// Render a visual progress bar for a 0-100 score.
pub fn render_score_bar(score: u8, width: usize) -> String {
    let filled = ((score as usize * width) + 50) / 100;
    let empty = width.saturating_sub(filled);
    let bar_str = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

    if score >= 80 {
        bar_str.green().bold().to_string()
    } else if score >= 65 {
        bar_str.yellow().bold().to_string()
    } else {
        bar_str.red().bold().to_string()
    }
}

/// Create an animated spinner for CLI commands.
pub fn create_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan.bold} {msg:.bold}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

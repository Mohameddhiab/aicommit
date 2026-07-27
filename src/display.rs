//! Styled boxed output for a modern terminal UI.

use colored::Colorize;

/// Print a box header like `╭─ title ───────────────────────────╮`
pub fn box_start(title: &str) {
    let line = format!("╭─ {title} ");
    let padding = 60usize.saturating_sub(line.len());
    let s = format!(
        "{}{}",
        line.cyan().bold(),
        "─".repeat(padding).cyan().bold()
    );
    println!("{s}");
}

/// Print a content line inside a box like `│  content  │`
pub fn box_line(content: &str) {
    let line = format!("│  {content}");
    let padding = 62usize.saturating_sub(line.len());
    println!("{}{}│", line, " ".repeat(padding));
}

/// Print a box footer like `╰──────────────────────────────────────╯`
pub fn box_end() {
    let s = format!(
        "{}{}{}",
        "╰".cyan().bold(),
        "─".repeat(61).cyan().bold(),
        "╯".cyan().bold()
    );
    println!("{s}");
}

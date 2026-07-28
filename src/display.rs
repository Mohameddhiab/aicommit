use colored::Colorize;
use unicode_width::UnicodeWidthStr;

const MIN_WIDTH: usize = 40;
const MAX_WIDTH: usize = 76;

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
    println!(
        "{}{}{}│",
        "│  ".cyan().bold(),
        content,
        " ".repeat(pad)
    );
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

use colored::Colorize;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, Table};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ReviewSeverity {
    Critical, // Block commit (e.g. secret leak)
    Warning,  // Debug code left behind
    Info,     // Best practice recommendation
}

#[derive(Debug, Clone, Serialize)]
pub struct ReviewIssue {
    pub severity: ReviewSeverity,
    pub path: String,
    pub line_num: Option<usize>,
    pub message: String,
    pub snippet: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReviewReport {
    pub score: u8,
    pub passed: bool,
    pub issues: Vec<ReviewIssue>,
    pub files_scanned: usize,
}

pub fn run_code_review(diff: &str) -> ReviewReport {
    let mut issues = Vec::new();
    let mut files_scanned = 0;
    let mut current_file = String::from("unknown");
    let mut current_line = 0;

    for line in diff.lines() {
        if line.starts_with("diff --git") || line.starts_with("+++ b/") {
            if line.starts_with("+++ b/") {
                current_file = line.trim_start_matches("+++ b/").to_string();
                files_scanned += 1;
            }
            continue;
        }

        if line.starts_with("@@ ") {
            // Parse line number from chunk header e.g. @@ -1,5 +15,10 @@
            if let Some(pos) = line.find('+') {
                let rest = &line[pos + 1..];
                let num_str = rest.split(&[',', ' '][..]).next().unwrap_or("1");
                current_line = num_str.parse::<usize>().unwrap_or(1);
            }
            continue;
        }

        if line.starts_with('+') && !line.starts_with("+++") {
            let content = &line[1..];
            current_line += 1;

            // 1. Check for API Keys & Secrets
            if content.contains("AKIA") && content.len() >= 20 {
                issues.push(ReviewIssue {
                    severity: ReviewSeverity::Critical,
                    path: current_file.clone(),
                    line_num: Some(current_line),
                    message: "Possible AWS Access Key ID detected!".to_string(),
                    snippet: Some(content.trim().to_string()),
                });
            } else if content.contains("sk-") && content.len() >= 30 {
                issues.push(ReviewIssue {
                    severity: ReviewSeverity::Critical,
                    path: current_file.clone(),
                    line_num: Some(current_line),
                    message: "Possible API key (OpenAI/Secret token) detected!".to_string(),
                    snippet: Some(content.trim().to_string()),
                });
            } else if content.contains("-----BEGIN PRIVATE KEY-----")
                || content.contains("-----BEGIN RSA PRIVATE KEY-----")
            {
                issues.push(ReviewIssue {
                    severity: ReviewSeverity::Critical,
                    path: current_file.clone(),
                    line_num: Some(current_line),
                    message: "Private RSA/SSH Key detected!".to_string(),
                    snippet: Some("[REDACTED PRIVATE KEY]".to_string()),
                });
            } else if content.contains("ghp_") || content.contains("glpat-") {
                issues.push(ReviewIssue {
                    severity: ReviewSeverity::Critical,
                    path: current_file.clone(),
                    line_num: Some(current_line),
                    message: "GitHub/GitLab Personal Access Token detected!".to_string(),
                    snippet: Some(content.trim().to_string()),
                });
            }

            // 2. Check for Debug statements & Temporary Code
            let is_source_file = is_code_file(&current_file);
            if is_source_file {
                if content.contains("console.log(") || content.contains("debugger;") {
                    issues.push(ReviewIssue {
                        severity: ReviewSeverity::Warning,
                        path: current_file.clone(),
                        line_num: Some(current_line),
                        message: "Debug statement left in code (console.log / debugger)".to_string(),
                        snippet: Some(content.trim().to_string()),
                    });
                } else if content.contains("binding.pry") || content.contains("import pdb; pdb.set_trace()") {
                    issues.push(ReviewIssue {
                        severity: ReviewSeverity::Warning,
                        path: current_file.clone(),
                        line_num: Some(current_line),
                        message: "Debugger breakpoint left in code".to_string(),
                        snippet: Some(content.trim().to_string()),
                    });
                } else if content.to_lowercase().contains("todo:") || content.to_lowercase().contains("fixme:") {
                    issues.push(ReviewIssue {
                        severity: ReviewSeverity::Info,
                        path: current_file.clone(),
                        line_num: Some(current_line),
                        message: "Unresolved TODO/FIXME comment found".to_string(),
                        snippet: Some(content.trim().to_string()),
                    });
                }
            }
        }
    }

    let critical_count = issues
        .iter()
        .filter(|i| i.severity == ReviewSeverity::Critical)
        .count();
    let warning_count = issues
        .iter()
        .filter(|i| i.severity == ReviewSeverity::Warning)
        .count();

    let passed = critical_count == 0;
    let base_score: i32 = 100 - (critical_count as i32 * 40) - (warning_count as i32 * 10);
    let score = base_score.clamp(0, 100) as u8;

    ReviewReport {
        score,
        passed,
        issues,
        files_scanned,
    }
}

fn is_code_file(path: &str) -> bool {
    let p = Path::new(path);
    match p.extension().and_then(|e| e.to_str()) {
        Some("rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "go" | "java" | "c" | "cpp" | "h" | "rb" | "php") => true,
        _ => false,
    }
}

pub fn render_review_cli(report: &ReviewReport) -> String {
    let mut out = String::new();
    let status_str = if report.passed {
        "PASSED (Ready to Commit)".green().bold()
    } else {
        "FAILED (Critical Issues Detected)".red().bold()
    };

    out.push_str(&format!(
        "\n🔍 Staged Code Review: {} (Score: {}/100)\nFiles Scanned: {}\n\n",
        status_str, report.score, report.files_scanned
    ));

    if report.issues.is_empty() {
        out.push_str(&format!("{}\n", "  ✓ No security vulnerabilities or debug artifacts found.".green().bold()));
        return out;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Severity").add_attribute(Attribute::Bold),
            Cell::new("Location").add_attribute(Attribute::Bold),
            Cell::new("Issue").add_attribute(Attribute::Bold),
            Cell::new("Snippet").add_attribute(Attribute::Bold),
        ]);

    for issue in &report.issues {
        let (sev_str, color) = match issue.severity {
            ReviewSeverity::Critical => ("CRITICAL", Color::Red),
            ReviewSeverity::Warning => ("WARNING", Color::Yellow),
            ReviewSeverity::Info => ("INFO", Color::Cyan),
        };

        let loc = match issue.line_num {
            Some(line) => format!("{}:{}", issue.path, line),
            None => issue.path.clone(),
        };

        table.add_row(vec![
            Cell::new(sev_str).fg(color).add_attribute(Attribute::Bold),
            Cell::new(loc),
            Cell::new(&issue.message),
            Cell::new(issue.snippet.as_deref().unwrap_or("-")),
        ]);
    }

    out.push_str(&table.to_string());
    out.push('\n');
    out
}

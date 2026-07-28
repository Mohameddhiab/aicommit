use crate::analyze::{HistoryReport, Severity, grade};

pub fn format_text(report: &HistoryReport) -> String {
    let g = grade(report.overall_score);
    let mut out = format!(
        "\nHistory Health: {g} ({}/100)\n",
        report.overall_score
    );
    out.push_str(&format!("  Message quality:     {:>3}%\n", report.score_message_quality as u8));
    out.push_str(&format!("  Atomicity:           {:>3}%\n", report.score_atomicity as u8));
    out.push_str(&format!("  Size discipline:     {:>3}%\n", report.score_size as u8));
    out.push_str(&format!("  Convention:          {:>3}%\n", report.score_convention as u8));
    out.push('\n');

    if report.issues.is_empty() {
        out.push_str("  ✓ No issues found.\n");
        return out;
    }

    out.push_str(&format!("Top issues ({} total):\n", report.issues.len()));
    for issue in &report.issues[..report.issues.len().min(10)] {
        let icon = match issue.severity {
            Severity::Critical => "🔴",
            Severity::Warning => "🟡",
            Severity::Info => "🟢",
        };
        let short = if issue.oid.len() >= 7 { &issue.oid[..7] } else { &issue.oid };
        out.push_str(&format!("  {icon} {short} — {}\n", issue.message));
    }
    out
}

pub fn format_json(report: &HistoryReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_default()
}

pub fn generate_html(report: &HistoryReport) -> String {
    let g = grade(report.overall_score);
    let issues_html: String = report.issues.iter().map(|i| {
        let color = match i.severity {
            Severity::Critical => "#ef4444",
            Severity::Warning => "#f59e0b",
            Severity::Info => "#22c55e",
        };
        format!(
            "<tr style=\"color:{color}\"><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            match i.severity {
                Severity::Critical => "CRITICAL",
                Severity::Warning => "WARNING",
                Severity::Info => "INFO",
            },
            &i.oid[..i.oid.len().min(7)],
            i.message
        )
    }).collect::<Vec<_>>().join("");

    format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>Git History Report</title>
<style>
body {{ font-family: system-ui, sans-serif; max-width: 800px; margin: 2em auto; padding: 0 1em; }}
h1 {{ color: #333; }}
.grade {{ font-size: 4em; font-weight: bold; }}
.A {{ color: #22c55e; }} .B {{ color: #16a34a; }} .C {{ color: #f59e0b; }}
.D {{ color: #f97316; }} .F {{ color: #ef4444; }}
table {{ width: 100%; border-collapse: collapse; }}
td, th {{ padding: 0.5em; text-align: left; border-bottom: 1px solid #eee; }}
</style></head><body>
<h1>Git History Report</h1>
<div class="grade {g}">{g}</div>
<p>Overall score: {score}/100</p>
<table>
<tr><th>Severity</th><th>Commit</th><th>Issue</th></tr>
{issues}
</table>
</body></html>"#,
        score = report.overall_score,
        g = g,
        issues = issues_html,
    )
}

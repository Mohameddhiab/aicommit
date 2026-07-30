use crate::analyze::{grade, CommitScore, HistoryReport, Severity};
use crate::display;
use colored::Colorize;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, Table};

pub fn format_text(report: &HistoryReport) -> String {
    let g = grade(report.overall_score);
    let score_bar = display::render_score_bar(report.overall_score, 20);

    let mut out = String::new();
    out.push_str(&format!(
        "\n🩺 Git History Health Index: {} ({}/100) {}\n\n",
        g.bold().cyan(),
        report.overall_score,
        score_bar
    ));

    // Dimension breakdown table
    let mut dim_table = Table::new();
    dim_table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Dimension").add_attribute(Attribute::Bold),
            Cell::new("Score").add_attribute(Attribute::Bold),
            Cell::new("Gauge").add_attribute(Attribute::Bold),
        ]);

    dim_table.add_row(vec![
        Cell::new("Message Quality"),
        Cell::new(&format!("{:.0}%", report.score_message_quality)),
        Cell::new(display::render_score_bar(report.score_message_quality as u8, 15)),
    ]);
    dim_table.add_row(vec![
        Cell::new("Atomicity"),
        Cell::new(&format!("{:.0}%", report.score_atomicity)),
        Cell::new(display::render_score_bar(report.score_atomicity as u8, 15)),
    ]);
    dim_table.add_row(vec![
        Cell::new("Size Discipline"),
        Cell::new(&format!("{:.0}%", report.score_size)),
        Cell::new(display::render_score_bar(report.score_size as u8, 15)),
    ]);
    dim_table.add_row(vec![
        Cell::new("Convention Compliance"),
        Cell::new(&format!("{:.0}%", report.score_convention)),
        Cell::new(display::render_score_bar(report.score_convention as u8, 15)),
    ]);

    out.push_str("📊 Health Dimensions:\n");
    out.push_str(&dim_table.to_string());
    out.push_str("\n\n");

    // Statistics Summary
    let (micro, small, med, large) = report.size_distribution();
    let (crit, warn, info) = report.issue_counts();

    out.push_str(&format!(
        "📈 Repository Statistics:\n  • Commits Analyzed: {}\n  • Lines Changed: +{} / -{}\n  • Commit Size Distribution: Micro (<50L): {}, Small (50-200L): {}, Medium (200-500L): {}, Large (>500L): {}\n  • Issues Summary: 🔴 Critical: {}, 🟡 Warning: {}, 🟢 Info: {}\n\n",
        report.total_commits(),
        report.total_insertions(),
        report.total_deletions(),
        micro, small, med, large,
        crit, warn, info
    ));

    if report.issues.is_empty() {
        out.push_str(&format!("{}\n", "✓ Perfect history! No issues detected.".green().bold()));
        return out;
    }

    // Issues Table
    let mut issue_table = Table::new();
    issue_table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Severity").add_attribute(Attribute::Bold),
            Cell::new("Commit").add_attribute(Attribute::Bold),
            Cell::new("Issue Details").add_attribute(Attribute::Bold),
        ]);

    for issue in report.issues.iter().take(12) {
        let (sev_str, color) = match issue.severity {
            Severity::Critical => ("🔴 CRITICAL", Color::Red),
            Severity::Warning => ("🟡 WARNING", Color::Yellow),
            Severity::Info => ("🟢 INFO", Color::Cyan),
        };
        let short_oid = if issue.oid.len() >= 7 { &issue.oid[..7] } else { &issue.oid };

        issue_table.add_row(vec![
            Cell::new(sev_str).fg(color).add_attribute(Attribute::Bold),
            Cell::new(short_oid).add_attribute(Attribute::Bold),
            Cell::new(&issue.message),
        ]);
    }

    out.push_str(&format!("🚨 Detected Issues (showing top {}):\n", report.issues.len().min(12)));
    out.push_str(&issue_table.to_string());
    out.push('\n');
    out
}

pub fn format_json(report: &HistoryReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_default()
}

pub fn format_markdown(report: &HistoryReport) -> String {
    let g = grade(report.overall_score);
    let badge = match g {
        "A" => "![A](https://img.shields.io/badge/health-A-success)",
        "B" => "![B](https://img.shields.io/badge/health-B-yellow)",
        "C" => "![C](https://img.shields.io/badge/health-C-orange)",
        "D" => "![D](https://img.shields.io/badge/health-D-red)",
        _ => "![F](https://img.shields.io/badge/health-F-critical)",
    };

    let mut out = format!(
        r#"## Git History Doctor Report

{badge} **Grade {g}** — Score: **{score}/100**

### 📊 Health Dimensions
| Dimension | Score | Rating |
|-----------|-------|--------|
| Message Quality | {mq}% | {mq_g} |
| Atomicity | {at}% | {at_g} |
| Size Discipline | {sz}% | {sz_g} |
| Convention Compliance | {cv}% | {cv_g} |

"#,
        badge = badge,
        g = g,
        score = report.overall_score,
        mq = report.score_message_quality as u8,
        mq_g = grade(report.score_message_quality as u8),
        at = report.score_atomicity as u8,
        at_g = grade(report.score_atomicity as u8),
        sz = report.score_size as u8,
        sz_g = grade(report.score_size as u8),
        cv = report.score_convention as u8,
        cv_g = grade(report.score_convention as u8),
    );

    if report.issues.is_empty() {
        out.push_str("✅ **No issues found. Your Git history is clean!**\n");
        return out;
    }

    out.push_str(&format!("### 🚨 Issues ({} total)\n\n", report.issues.len()));
    out.push_str("| Severity | Commit | Issue Description |\n");
    out.push_str("|----------|--------|-------------------|\n");

    for issue in &report.issues {
        let severity_badge = match issue.severity {
            Severity::Critical => "🔴 CRITICAL",
            Severity::Warning => "🟡 WARNING",
            Severity::Info => "🟢 INFO",
        };
        let short = if issue.oid.len() >= 7 { &issue.oid[..7] } else { &issue.oid };
        out.push_str(&format!(
            "| {} | `{}` | {} |\n",
            severity_badge, short, issue.message
        ));
    }

    out.push_str("\n---\n");
    out.push_str(
        "_Report generated by [git-doctor](https://github.com/Mohameddhiab/git-doctor)_\n"
    );
    out
}

pub fn format_per_commit_markdown(scores: &[CommitScore]) -> String {
    let mut out = "### Per-Commit Breakdown\n\n".to_string();
    out.push_str("| Commit | Subject | Files | +/- | Quality | Atomicity | Size | Convention |\n");
    out.push_str("|--------|---------|-------|-----|---------|-----------|------|------------|\n");

    for c in scores {
        let short = if c.oid.len() >= 7 { &c.oid[..7] } else { &c.oid };
        let flags = {
            let mut f = Vec::new();
            if c.is_wip { f.push("⚠WIP"); }
            if c.is_vague { f.push("⚠vague"); }
            if c.is_oversized { f.push("⚠oversized"); }
            if c.is_mixed_concern { f.push("⚠mixed"); }
            if f.is_empty() { "✓".to_string() } else { f.join(" ") }
        };
        out.push_str(&format!(
            "| `{}` | {} {} | {} | +{}/-{} | {}% | {}% | {}% | {}% |\n",
            short, flags, c.subject, c.files_changed, c.insertions, c.deletions,
            c.message_quality, c.atomicity, c.size_discipline, c.convention,
        ));
    }
    out
}

/// Generate a React Doctor-inspired Web Dashboard HTML Report
pub fn generate_html(report: &HistoryReport) -> String {
    let g = grade(report.overall_score);
    let (micro, small, med, large) = report.size_distribution();
    let (crit, warn, info) = report.issue_counts();

    let grade_color = match g {
        "A" => "#22c55e",
        "B" => "#3b82f6",
        "C" => "#f59e0b",
        "D" => "#f97316",
        _ => "#ef4444",
    };

    let issues_rows: String = report
        .issues
        .iter()
        .map(|i| {
            let (sev_class, sev_label, badge_color) = match i.severity {
                Severity::Critical => ("critical", "CRITICAL", "#ef4444"),
                Severity::Warning => ("warning", "WARNING", "#f59e0b"),
                Severity::Info => ("info", "INFO", "#06b6d4"),
            };
            let short = if i.oid.len() >= 7 { &i.oid[..7] } else { &i.oid };
            format!(
                r#"<tr class="issue-row {sev_class}">
                    <td><span class="badge" style="background: {badge_color}22; color: {badge_color}; border: 1px solid {badge_color}44;">{sev_label}</span></td>
                    <td><code>{short}</code></td>
                    <td>{msg}</td>
                </tr>"#,
                sev_class = sev_class,
                badge_color = badge_color,
                sev_label = sev_label,
                short = short,
                msg = html_escape(&i.message)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let commit_rows: String = report
        .commits
        .iter()
        .map(|c| {
            let short = if c.oid.len() >= 7 { &c.oid[..7] } else { &c.oid };
            format!(
                r#"<tr>
                    <td><code>{short}</code></td>
                    <td><strong>{author}</strong></td>
                    <td class="subject-cell">{subject}</td>
                    <td><span class="diff-add">+{inc}</span> / <span class="diff-del">-{dec}</span> ({files} files)</td>
                    <td><div class="mini-bar"><div class="mini-fill" style="width:{mq}%; background:#38bdf8;"></div></div> {mq}%</td>
                    <td><div class="mini-bar"><div class="mini-fill" style="width:{at}%; background:#818cf8;"></div></div> {at}%</td>
                    <td><div class="mini-bar"><div class="mini-fill" style="width:{sz}%; background:#a855f7;"></div></div> {sz}%</td>
                    <td><div class="mini-bar"><div class="mini-fill" style="width:{cv}%; background:#22c55e;"></div></div> {cv}%</td>
                </tr>"#,
                short = short,
                author = html_escape(&c.author),
                subject = html_escape(&c.subject),
                inc = c.insertions,
                dec = c.deletions,
                files = c.files_changed,
                mq = c.message_quality,
                at = c.atomicity,
                sz = c.size_discipline,
                cv = c.convention,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Git Doctor — Health Dashboard</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;600&display=swap" rel="stylesheet">
    <style>
        :root {{
            --bg: #0b0f19;
            --card-bg: #151c2c;
            --border: #232d42;
            --text-main: #f1f5f9;
            --text-muted: #94a3b8;
            --accent-cyan: #38bdf8;
            --accent-indigo: #818cf8;
            --accent-purple: #c084fc;
            --grade-color: {grade_color};
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: 'Inter', system-ui, -apple-system, sans-serif;
            background-color: var(--bg);
            color: var(--text-main);
            line-height: 1.5;
            padding: 2rem;
        }}
        .dashboard-container {{ max-width: 1280px; margin: 0 auto; display: flex; flex-direction: column; gap: 2rem; }}
        header {{
            display: flex; justify-content: space-between; align-items: center;
            background: var(--card-bg); border: 1px solid var(--border); border-radius: 16px;
            padding: 1.5rem 2rem; box-shadow: 0 10px 30px rgba(0,0,0,0.4);
        }}
        .brand {{ display: flex; align-items: center; gap: 1rem; }}
        .brand-icon {{ font-size: 2rem; background: linear-gradient(135deg, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }}
        .brand h1 {{ font-size: 1.75rem; font-weight: 800; letter-spacing: -0.02em; }}
        .brand p {{ font-size: 0.875rem; color: var(--text-muted); }}

        .kpi-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 1.25rem; }}
        .kpi-card {{
            background: var(--card-bg); border: 1px solid var(--border); border-radius: 16px; padding: 1.5rem;
            display: flex; flex-direction: column; justify-content: space-between; transition: transform 0.2s, border-color 0.2s;
        }}
        .kpi-card:hover {{ transform: translateY(-2px); border-color: var(--accent-cyan); }}
        .kpi-title {{ font-size: 0.85rem; color: var(--text-muted); text-transform: uppercase; font-weight: 600; letter-spacing: 0.05em; }}
        .kpi-value {{ font-size: 2.25rem; font-weight: 800; margin-top: 0.5rem; }}

        .gauge-container {{ display: flex; align-items: center; gap: 1.5rem; }}
        .gauge-circle {{
            width: 80px; height: 80px; border-radius: 50%;
            background: conic-gradient(var(--grade-color) calc({score} * 1%), #1e293b 0);
            display: flex; align-items: center; justify-content: center; position: relative;
        }}
        .gauge-inner {{ width: 64px; height: 64px; border-radius: 50%; background: var(--card-bg); display: flex; align-items: center; justify-content: center; font-size: 1.5rem; font-weight: 800; color: var(--grade-color); }}

        .grid-2col {{ display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; }}
        @media(max-width: 900px) {{ .grid-2col {{ grid-template-columns: 1fr; }} }}

        .panel {{ background: var(--card-bg); border: 1px solid var(--border); border-radius: 16px; padding: 1.5rem; display: flex; flex-direction: column; gap: 1rem; }}
        .panel-header {{ font-size: 1.1rem; font-weight: 700; display: flex; justify-content: space-between; align-items: center; }}

        .dimension-bar {{ display: flex; flex-direction: column; gap: 0.35rem; }}
        .dim-info {{ display: flex; justify-content: space-between; font-size: 0.875rem; font-weight: 500; }}
        .progress-track {{ height: 10px; background: #1e293b; border-radius: 5px; overflow: hidden; }}
        .progress-fill {{ height: 100%; border-radius: 5px; transition: width 0.8s ease-out; }}

        table {{ width: 100%; border-collapse: collapse; text-align: left; font-size: 0.875rem; }}
        th {{ padding: 0.75rem 1rem; background: #0f172a; color: var(--text-muted); font-weight: 600; text-transform: uppercase; font-size: 0.75rem; letter-spacing: 0.05em; }}
        td {{ padding: 0.75rem 1rem; border-top: 1px solid var(--border); }}
        code {{ font-family: 'JetBrains Mono', monospace; background: #1e293b; padding: 0.2rem 0.4rem; border-radius: 4px; color: var(--accent-cyan); font-size: 0.825rem; }}
        
        .badge {{ padding: 0.2rem 0.5rem; border-radius: 6px; font-size: 0.75rem; font-weight: 700; letter-spacing: 0.03em; display: inline-block; }}
        .diff-add {{ color: #22c55e; font-weight: 600; }}
        .diff-del {{ color: #ef4444; font-weight: 600; }}
        .subject-cell {{ max-width: 320px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}

        .mini-bar {{ display: inline-block; width: 40px; height: 6px; background: #1e293b; border-radius: 3px; overflow: hidden; vertical-align: middle; margin-right: 4px; }}
        .mini-fill {{ height: 100%; border-radius: 3px; }}

        .search-input {{ background: #0f172a; border: 1px solid var(--border); color: var(--text-main); padding: 0.5rem 1rem; border-radius: 8px; font-size: 0.875rem; width: 220px; outline: none; }}
        .search-input:focus {{ border-color: var(--accent-cyan); }}

        footer {{ text-align: center; color: var(--text-muted); font-size: 0.85rem; padding: 1rem 0; border-top: 1px solid var(--border); margin-top: 1rem; }}
    </style>
</head>
<body>
    <div class="dashboard-container">
        <header>
            <div class="brand">
                <div class="brand-icon">🩺</div>
                <div>
                    <h1>Git Doctor Health Dashboard</h1>
                    <p>AI-Powered Commit Quality & History Analytics</p>
                </div>
            </div>
            <div class="gauge-container">
                <div>
                    <div style="font-size: 0.75rem; color: var(--text-muted); text-transform: uppercase; font-weight: 700; text-align: right;">Overall Grade</div>
                    <div style="font-size: 1.25rem; font-weight: 800; color: var(--grade-color); text-align: right;">Grade {g}</div>
                </div>
                <div class="gauge-circle">
                    <div class="gauge-inner">{score}</div>
                </div>
            </div>
        </header>

        <div class="kpi-grid">
            <div class="kpi-card">
                <div class="kpi-title">Health Score</div>
                <div class="kpi-value" style="color: var(--grade-color);">{score}/100</div>
            </div>
            <div class="kpi-card">
                <div class="kpi-title">Commits Analyzed</div>
                <div class="kpi-value">{total_commits}</div>
            </div>
            <div class="kpi-card">
                <div class="kpi-title">Lines Changed</div>
                <div class="kpi-value" style="font-size: 1.75rem;"><span class="diff-add">+{insertions}</span> / <span class="diff-del">-{deletions}</span></div>
            </div>
            <div class="kpi-card">
                <div class="kpi-title">Active Issues</div>
                <div class="kpi-value" style="color: {issue_color}; font-size: 1.5rem;">🔴{crit} / 🟡{warn} / 🟢{info}</div>
            </div>
        </div>

        <div class="grid-2col">
            <div class="panel">
                <div class="panel-header">
                    <span>📊 Health Dimension Breakdown</span>
                </div>
                <div style="display: flex; flex-direction: column; gap: 1rem;">
                    <div class="dimension-bar">
                        <div class="dim-info"><span>Message Quality</span><span>{mq}%</span></div>
                        <div class="progress-track"><div class="progress-fill" style="width:{mq}%; background: linear-gradient(90deg, #38bdf8, #818cf8);"></div></div>
                    </div>
                    <div class="dimension-bar">
                        <div class="dim-info"><span>Atomicity</span><span>{at}%</span></div>
                        <div class="progress-track"><div class="progress-fill" style="width:{at}%; background: linear-gradient(90deg, #818cf8, #c084fc);"></div></div>
                    </div>
                    <div class="dimension-bar">
                        <div class="dim-info"><span>Size Discipline</span><span>{sz}%</span></div>
                        <div class="progress-track"><div class="progress-fill" style="width:{sz}%; background: linear-gradient(90deg, #c084fc, #f472b6);"></div></div>
                    </div>
                    <div class="dimension-bar">
                        <div class="dim-info"><span>Convention Compliance</span><span>{cv}%</span></div>
                        <div class="progress-track"><div class="progress-fill" style="width:{cv}%; background: linear-gradient(90deg, #22c55e, #34d399);"></div></div>
                    </div>
                </div>
            </div>

            <div class="panel">
                <div class="panel-header">
                    <span>📈 Commit Size Distribution</span>
                </div>
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; margin-top: 0.5rem;">
                    <div style="background: #0f172a; padding: 1rem; border-radius: 10px; border: 1px solid var(--border);">
                        <div style="font-size: 0.75rem; color: var(--text-muted);">Micro (<50 lines)</div>
                        <div style="font-size: 1.5rem; font-weight: 700; color: #38bdf8;">{micro}</div>
                    </div>
                    <div style="background: #0f172a; padding: 1rem; border-radius: 10px; border: 1px solid var(--border);">
                        <div style="font-size: 0.75rem; color: var(--text-muted);">Small (50-200 lines)</div>
                        <div style="font-size: 1.5rem; font-weight: 700; color: #22c55e;">{small}</div>
                    </div>
                    <div style="background: #0f172a; padding: 1rem; border-radius: 10px; border: 1px solid var(--border);">
                        <div style="font-size: 0.75rem; color: var(--text-muted);">Medium (200-500 lines)</div>
                        <div style="font-size: 1.5rem; font-weight: 700; color: #f59e0b;">{med}</div>
                    </div>
                    <div style="background: #0f172a; padding: 1rem; border-radius: 10px; border: 1px solid var(--border);">
                        <div style="font-size: 0.75rem; color: var(--text-muted);">Large (>500 lines)</div>
                        <div style="font-size: 1.5rem; font-weight: 700; color: #ef4444;">{large}</div>
                    </div>
                </div>
            </div>
        </div>

        <div class="panel">
            <div class="panel-header">
                <span>🚨 Identified History Issues ({issue_count})</span>
            </div>
            <table>
                <thead>
                    <tr><th>Severity</th><th>Commit</th><th>Issue Description</th></tr>
                </thead>
                <tbody>
                    {issues_rows}
                </tbody>
            </table>
        </div>

        <div class="panel">
            <div class="panel-header">
                <span>📝 Per-Commit Health Matrix</span>
                <input type="text" id="commitSearch" class="search-input" placeholder="Search commit or author..." onkeyup="filterCommits()">
            </div>
            <table id="commitsTable">
                <thead>
                    <tr>
                        <th>Commit</th><th>Author</th><th>Subject</th><th>Changes</th>
                        <th>Quality</th><th>Atomicity</th><th>Size</th><th>Convention</th>
                    </tr>
                </thead>
                <tbody>
                    {commit_rows}
                </tbody>
            </table>
        </div>

        <footer>
            Report generated automatically by <strong>Git Doctor</strong> v0.2.0 • AI-Powered Commit Quality Doctor
        </footer>
    </div>

    <script>
        function filterCommits() {{
            const input = document.getElementById('commitSearch');
            const filter = input.value.toLowerCase();
            const table = document.getElementById('commitsTable');
            const tr = table.getElementsByTagName('tr');
            for (let i = 1; i < tr.length; i++) {{
                let txt = tr[i].textContent || tr[i].innerText;
                if (txt.toLowerCase().indexOf(filter) > -1) {{
                    tr[i].style.display = "";
                }} else {{
                    tr[i].style.display = "none";
                }}
            }}
        }}
    </script>
</body>
</html>"#,
        score = report.overall_score,
        g = g,
        grade_color = grade_color,
        total_commits = report.total_commits(),
        insertions = report.total_insertions(),
        deletions = report.total_deletions(),
        crit = crit,
        warn = warn,
        info = info,
        issue_color = if crit > 0 { "#ef4444" } else if warn > 0 { "#f59e0b" } else { "#22c55e" },
        mq = report.score_message_quality as u8,
        at = report.score_atomicity as u8,
        sz = report.score_size as u8,
        cv = report.score_convention as u8,
        micro = micro,
        small = small,
        med = med,
        large = large,
        issue_count = report.issues.len(),
        issues_rows = issues_rows,
        commit_rows = commit_rows,
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

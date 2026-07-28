const ALLOWED_TYPES: &[&str] = &[
    "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore",
];

const WIP_PATTERNS: &[&str] = &[
    "wip", "fix me", "fixme", "todo", "temp", "tmp", "asdf", "test",
    "draft", "needfix", "broken", "hack", "workaround",
];

const VAGUE_PATTERNS: &[&str] = &[
    "update", "fix", "stuff", "change", "thing", "misc", "whatever",
    "bugfix", "small fix", "minor", "cleanup", "refactor",
];

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommitScore {
    pub oid: String,
    pub subject: String,
    pub author: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub message_quality: u8,
    pub atomicity: u8,
    pub size_discipline: u8,
    pub convention: u8,
    pub is_wip: bool,
    pub is_vague: bool,
    pub is_oversized: bool,
    pub is_mixed_concern: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct HistoryReport {
    pub commits: Vec<CommitScore>,
    pub overall_score: u8,
    pub score_message_quality: f64,
    pub score_atomicity: f64,
    pub score_size: f64,
    pub score_convention: f64,
    pub issues: Vec<Issue>,
}

#[derive(Debug, serde::Serialize)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, serde::Serialize)]
pub struct Issue {
    pub severity: Severity,
    pub oid: String,
    pub message: String,
}

#[allow(clippy::too_many_arguments)]
pub fn score_commit(
    oid: &str,
    subject: &str,
    author: &str,
    body: &str,
    files_changed: usize,
    insertions: usize,
    deletions: usize,
    domains: &[String],
) -> CommitScore {
    let message_quality = score_message(subject, body);
    let atomicity = score_atomicity(files_changed, domains, insertions + deletions);
    let size_discipline = score_size(insertions + deletions, files_changed);
    let convention = score_convention(subject);
    let is_wip = detect_wip(subject);
    let is_vague = detect_vague(subject);
    let is_oversized = (insertions + deletions) > 500 || files_changed > 10;
    let is_mixed_concern = domains.len() > 1;

    CommitScore {
        oid: oid.to_string(),
        subject: subject.to_string(),
        author: author.to_string(),
        files_changed,
        insertions,
        deletions,
        message_quality,
        atomicity,
        size_discipline,
        convention,
        is_wip,
        is_vague,
        is_oversized,
        is_mixed_concern,
    }
}

fn score_message(subject: &str, body: &str) -> u8 {
    let mut score = 100u8;
    let words: Vec<&str> = subject.split_whitespace().collect();

    if words.len() < 3 {
        score = score.saturating_sub(30);
    }
    if subject.len() < 10 {
        score = score.saturating_sub(20);
    }
    if body.is_empty() || body.trim().is_empty() {
        score = score.saturating_sub(10);
    }
    if subject.chars().next().is_none_or(|c| c.is_lowercase()) {
        score = score.saturating_sub(10);
    }
    if subject.contains("...") {
        score = score.saturating_sub(15);
    }
    score
}

fn score_atomicity(files: usize, domains: &[String], total_changes: usize) -> u8 {
    if files <= 3 && domains.len() <= 1 {
        100
    } else if files <= 8 && domains.len() <= 2 && total_changes < 300 {
        70
    } else {
        40
    }
}

fn score_size(total_changes: usize, files: usize) -> u8 {
    if total_changes < 100 && files < 5 {
        100
    } else if total_changes < 300 && files < 10 {
        70
    } else if total_changes < 500 && files < 15 {
        50
    } else {
        30
    }
}

fn score_convention(subject: &str) -> u8 {
    let lower = subject.to_lowercase();
    for t in ALLOWED_TYPES {
        if lower.starts_with(t) && lower.contains(':') {
            return 100;
        }
    }
    if subject.contains(':') {
        return 50;
    }
    20
}

pub fn detect_wip(subject: &str) -> bool {
    let lower = subject.to_lowercase();
    for pat in WIP_PATTERNS {
        if lower.contains(pat) {
            return true;
        }
    }
    false
}

pub fn detect_vague(subject: &str) -> bool {
    let lower = subject.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    if words.len() <= 2 {
        return true;
    }
    for pat in VAGUE_PATTERNS {
        let p = *pat;
        if lower == p || lower.starts_with(p) && words.len() <= 3 {
            return true;
        }
    }
    false
}

pub fn build_report(commits: Vec<CommitScore>) -> HistoryReport {
    let n = commits.len() as f64;
    if n == 0.0 {
        return HistoryReport {
            commits,
            overall_score: 100,
            score_message_quality: 100.0,
            score_atomicity: 100.0,
            score_size: 100.0,
            score_convention: 100.0,
            issues: vec![],
        };
    }

    let avg_msg: f64 = commits.iter().map(|c| c.message_quality as f64).sum::<f64>() / n;
    let avg_atomicity: f64 = commits.iter().map(|c| c.atomicity as f64).sum::<f64>() / n;
    let avg_size: f64 = commits.iter().map(|c| c.size_discipline as f64).sum::<f64>() / n;
    let avg_convention: f64 = commits.iter().map(|c| c.convention as f64).sum::<f64>() / n;

    let overall = (avg_msg * 0.3 + avg_atomicity * 0.3 + avg_size * 0.2 + avg_convention * 0.2).round() as u8;

    let mut issues = Vec::new();
    for c in &commits {
        if c.is_wip {
            issues.push(Issue {
                severity: Severity::Critical,
                oid: c.oid.clone(),
                message: format!("WIP commit: \"{}\"", c.subject),
            });
        }
        if c.is_mixed_concern {
            issues.push(Issue {
                severity: Severity::Warning,
                oid: c.oid.clone(),
                message: format!("Mixed concern: {} files across {} domains", c.files_changed, count_domains(&c.subject)),
            });
        }
        if c.is_vague && !c.is_wip {
            issues.push(Issue {
                severity: Severity::Warning,
                oid: c.oid.clone(),
                message: format!("Vague message: \"{}\"", c.subject),
            });
        }
        if c.is_oversized {
            issues.push(Issue {
                severity: Severity::Warning,
                oid: c.oid.clone(),
                message: format!("Oversized: +{} -{} in {} files", c.insertions, c.deletions, c.files_changed),
            });
        }
        if c.message_quality < 60 {
            issues.push(Issue {
                severity: Severity::Info,
                oid: c.oid.clone(),
                message: format!("Low message quality ({})", c.message_quality),
            });
        }
    }
    issues.sort_by_key(|i| match i.severity {
        Severity::Critical => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    });

    HistoryReport {
        commits,
        overall_score: overall,
        score_message_quality: (avg_msg * 100.0).round() / 100.0,
        score_atomicity: (avg_atomicity * 100.0).round() / 100.0,
        score_size: (avg_size * 100.0).round() / 100.0,
        score_convention: (avg_convention * 100.0).round() / 100.0,
        issues,
    }
}

fn count_domains(_subject: &str) -> usize {
    2
}

pub fn grade(score: u8) -> &'static str {
    if score >= 90 {
        "A"
    } else if score >= 80 {
        "B"
    } else if score >= 65 {
        "C"
    } else if score >= 50 {
        "D"
    } else {
        "F"
    }
}

//! Parsing of LLM responses into strict `CommitMessage` values, with a regex
//! fallback for providers that don't honor JSON output.

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

/// Allowed Conventional Commit types.
pub const ALLOWED_TYPES: &[&str] = &[
    "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore",
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CommitMessage {
    #[serde(rename = "type")]
    pub kind: String,
    pub scope: String,
    pub description: String,
    #[serde(default)]
    pub body: String,
}

impl CommitMessage {
    /// Format as a Conventional Commit single-line message,
    /// appending an optional body separated by a blank line.
    pub fn to_conventional(&self) -> String {
        let scope = if self.scope.is_empty() || self.scope == "misc" {
            String::new()
        } else {
            format!("({})", self.scope)
        };
        let head = format!("{}{}: {}", self.kind, scope, self.description);
        if self.body.trim().is_empty() {
            head
        } else {
            format!("{head}\n\n{}", self.body.trim())
        }
    }
}

/// Parse the raw LLM response into a `CommitMessage`.
///
/// Strategy: first try strict JSON parsing; if that fails, fall back to a
/// permissive Conventional-Commits regex sweep over the text.
pub fn parse_commit_message(raw: &str) -> Result<CommitMessage> {
    if let Ok(msg) = parse_json(raw) {
        return validate(msg);
    }
    if let Ok(msg) = parse_text_fallback(raw) {
        return validate(msg);
    }
    bail!("could not parse LLM response as a commit message");
}

fn parse_json(raw: &str) -> Result<CommitMessage> {
    // Tolerate markdown fences and surrounding prose.
    let trimmed = extract_json_block(raw.trim());
    let msg: CommitMessage = serde_json::from_str(trimmed)?;
    Ok(msg)
}

fn extract_json_block(s: &str) -> &str {
    let s = s.trim();
    let s = if s.starts_with("```") {
        let without_fence = s.trim_start_matches("```json").trim_start_matches("```");
        if let Some(end) = without_fence.rfind("```") {
            without_fence[..end].trim()
        } else {
            without_fence.trim()
        }
    } else {
        s
    };
    if let Some(start) = s.find('{') {
        if let Some(end) = s.rfind('}') {
            return &s[start..=end];
        }
    }
    s
}

fn parse_text_fallback(raw: &str) -> Result<CommitMessage> {
    // We accept either a one-line "<type>(<scope>): <description>" or
    // just a "<type>: <description>".
    let first = raw
        .lines()
        .next()
        .ok_or_else(|| anyhow!("empty response"))?;
    let line = first.trim().trim_matches('"').trim_matches('`');
    let (head, body) = match line.split_once('\n') {
        Some((h, b)) => (h, b.trim().to_string()),
        None => (line, String::new()),
    };
    let (kind, rest) = head
        .split_once(':')
        .ok_or_else(|| anyhow!("missing ':' in conventional commit line"))?;
    let kind = kind.trim().to_ascii_lowercase();
    let (kind, scope) = match kind.split_once('(') {
        Some((k, s)) => {
            let scope = s.trim_end_matches(')').to_string();
            (k.to_string(), scope)
        }
        None => (kind, String::new()),
    };
    Ok(CommitMessage {
        kind,
        scope,
        description: rest.trim().to_string(),
        body,
    })
}

fn validate(msg: CommitMessage) -> Result<CommitMessage> {
    if !ALLOWED_TYPES.contains(&msg.kind.as_str()) {
        bail!("invalid type '{}'", msg.kind);
    }
    if msg.description.is_empty() {
        bail!("empty description");
    }
    if msg.description.len() > 120 {
        bail!("description too long ({} chars)", msg.description.len());
    }
    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_strict_json() {
        let raw = r#"{"type":"feat","scope":"auth","description":"add jwt validation","body":""}"#;
        let msg = parse_commit_message(raw).unwrap();
        assert_eq!(msg.kind, "feat");
        assert_eq!(msg.scope, "auth");
        assert_eq!(msg.description, "add jwt validation");
        assert_eq!(msg.to_conventional(), "feat(auth): add jwt validation");
    }

    #[test]
    fn parse_json_with_prose() {
        let raw = "Sure! Here is the commit message:\n{\"type\":\"fix\",\"scope\":\"db\",\"description\":\"handle timeout\",\"body\":\"\"}\nHope it helps!";
        let msg = parse_commit_message(raw).unwrap();
        assert_eq!(msg.kind, "fix");
    }

    #[test]
    fn parse_text_fallback_works() {
        let raw = "fix(auth): handle jwt expiration\n";
        let msg = parse_commit_message(raw).unwrap();
        assert_eq!(msg.kind, "fix");
        assert_eq!(msg.scope, "auth");
        assert_eq!(msg.description, "handle jwt expiration");
    }

    #[test]
    fn parse_text_no_scope() {
        let raw = "docs: update README\n";
        let msg = parse_commit_message(raw).unwrap();
        assert_eq!(msg.kind, "docs");
        assert_eq!(msg.scope, "");
        assert_eq!(msg.description, "update README");
    }

    #[test]
    fn rejects_invalid_type() {
        let raw = r#"{"type":"wip","scope":"","description":"stuff","body":""}"#;
        let _ = parse_commit_message(raw).unwrap_err();
    }

    #[test]
    fn rejects_empty_description() {
        let raw = r#"{"type":"feat","scope":"x","description":"","body":""}"#;
        let _ = parse_commit_message(raw).unwrap_err();
    }

    #[test]
    fn parse_json_with_markdown_fence() {
        let raw = "```json\n{\"type\":\"feat\",\"scope\":\"cli\",\"description\":\"add markdown fence support\",\"body\":\"\"}\n```";
        let msg = parse_commit_message(raw).unwrap();
        assert_eq!(msg.kind, "feat");
        assert_eq!(msg.scope, "cli");
        assert_eq!(msg.description, "add markdown fence support");
    }

    #[test]
    fn parse_json_with_markdown_fence_no_lang() {
        let raw = "```\n{\"type\":\"fix\",\"scope\":\"db\",\"description\":\"handle connection pool leak\",\"body\":\"\"}\n```";
        let msg = parse_commit_message(raw).unwrap();
        assert_eq!(msg.kind, "fix");
        assert_eq!(msg.description, "handle connection pool leak");
    }

    #[test]
    fn body_appended_when_present() {
        let msg = CommitMessage {
            kind: "fix".into(),
            scope: "cli".into(),
            description: "crash on empty diff".into(),
            body: "Was calling unwrap on None.".into(),
        };
        assert_eq!(
            msg.to_conventional(),
            "fix(cli): crash on empty diff\n\nWas calling unwrap on None."
        );
    }
}

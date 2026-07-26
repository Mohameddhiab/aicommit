//! Prompt construction (system + user).

use crate::config::Config;

/// Build the system prompt enforcing Conventional Commits and JSON output.
pub fn system_prompt(cfg: &Config) -> String {
    let lang_clause = if cfg.commit.language.eq_ignore_ascii_case("en") {
        String::new()
    } else {
        format!("\nWrite the commit message in {}.", cfg.commit.language)
    };
    format!(
"You are an AI expert in Git and the Conventional Commits specification.

Generate a commit message following the Conventional Commits specification.
Format: <type>(<scope>): <description>

Allowed types: feat, fix, docs, style, refactor, perf, test, build, ci, chore

Respond with ONLY a single JSON object, no markdown fences, no extra text, \
matching exactly this schema:
{{\"type\": \"<type>\", \"scope\": \"<scope>\", \"description\": \"<description>\", \"body\": \"<optional body>\"}}

The description must be lowercase, imperative, max 72 characters, no trailing period.\
The scope must be a single short noun (e.g. \"auth\", \"db\", \"cli\"). If unclear, use \"misc\".\
The body is optional; if absent use an empty string.{lang_clause}"
    )
}

/// Build the user prompt containing the compressed diff.
pub fn user_prompt(diff: &str) -> String {
    // Compress very large diffs by truncating each file's diff to a sane size.
    let compressed = compress_diff(diff);
    format!(
        "Below is the staged git diff. Analyze it and produce ONE commit message \
in the JSON schema required.\n\n```diff\n{compressed}\n```"
    )
}

/// Truncate extremely large diffs to avoid blowing the model context window.
/// We keep up to ~8 KB per file and drop "Binary files" lines.
fn compress_diff(diff: &str) -> String {
    const MAX_BYTES_PER_FILE: usize = 8_000;
    const MAX_TOTAL_BYTES: usize = 30_000;
    let capacity = diff.len().min(MAX_TOTAL_BYTES);
    let mut out = String::with_capacity(capacity);
    let mut current_size = 0usize;
    let mut current_buf = String::new();
    let mut current_header = String::new();
    let mut total_truncated = false;

    for line in diff.lines() {
        if line.starts_with("diff --git") {
            if !current_buf.is_empty() {
                out.push_str(&current_header);
                out.push_str(&truncate(&current_buf, MAX_BYTES_PER_FILE));
                out.push('\n');
                if out.len() >= MAX_TOTAL_BYTES {
                    total_truncated = true;
                    break;
                }
            }
            current_header.clear();
            current_header.push_str(line);
            current_header.push('\n');
            current_buf.clear();
            current_size = 0;
            continue;
        }
        if line.starts_with("Binary files") {
            current_buf.push_str(line);
            current_buf.push('\n');
            continue;
        }
        if current_size < MAX_BYTES_PER_FILE {
            current_buf.push_str(line);
            current_buf.push('\n');
            current_size += line.len() + 1;
        }
    }
    if !current_buf.is_empty() && !total_truncated {
        out.push_str(&current_header);
        out.push_str(&truncate(&current_buf, MAX_BYTES_PER_FILE));
        if out.len() >= MAX_TOTAL_BYTES {
            total_truncated = true;
        }
    }
    if total_truncated {
        out.push_str("\n... [diff truncated at ~30 KB, commit in smaller batches] ...\n");
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut t = s[..max].to_string();
    t.push_str("\n... [truncated by aicommit] ...\n");
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_preserves_small_diff() {
        let diff = "diff --git a/foo b/foo\n--- a/foo\n+++ b/foo\n@@ -1 +1 @@\n-x\n+y\n";
        assert_eq!(compress_diff(diff), diff);
    }

    #[test]
    fn compress_truncates_large_diff() {
        let big = format!("diff --git a/foo b/foo\n{}", "x".repeat(20_000));
        let compressed = compress_diff(&big);
        assert!(compressed.len() < big.len());
        assert!(compressed.contains("[truncated"));
    }

    #[test]
    fn compress_truncates_global_limit() {
        let mut big = String::new();
        for i in 0..6 {
            big.push_str(&format!(
                "diff --git a/file{i} b/file{i}\n{}\n",
                "x".repeat(7_000)
            ));
        }
        let compressed = compress_diff(&big);
        assert!(
            compressed.len() < big.len(),
            "global truncation should reduce size"
        );
        assert!(
            compressed.contains("commit in smaller batches"),
            "global truncation message should be present"
        );
    }
}

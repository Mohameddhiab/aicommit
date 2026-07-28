use crate::analyze::HistoryReport;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Operation {
    Squash {
        target_oid: String,
        into_oid: String,
        subject: String,
        reason: String,
    },
    Reword {
        oid: String,
        old_subject: String,
        suggested_subject: String,
        reason: String,
    },
    Split {
        oid: String,
        subject: String,
        groups: Vec<String>,
        reason: String,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Plan {
    pub operations: Vec<Operation>,
    pub description: String,
    pub commit_count: usize,
    pub operation_count: usize,
}

pub fn generate_plan(report: &HistoryReport) -> Plan {
    let mut operations = Vec::new();

    for issue in &report.issues {
        let score = report.commits.iter().find(|c| c.oid == issue.oid);
        let cs = match score {
            Some(s) => s,
            None => continue,
        };

        match issue.severity {
            crate::analyze::Severity::Critical => {
                if cs.is_wip {
                    operations.push(Operation::Reword {
                        oid: cs.oid.clone(),
                        old_subject: cs.subject.clone(),
                        suggested_subject: suggest_reword(&cs.subject),
                        reason: "WIP commit should be rewritten".to_string(),
                    });
                }
            }
            crate::analyze::Severity::Warning => {
                if cs.is_mixed_concern {
                    operations.push(Operation::Split {
                        oid: cs.oid.clone(),
                        subject: cs.subject.clone(),
                        groups: vec!["group-1".into(), "group-2".into()],
                        reason: "Commit touches multiple logical domains".to_string(),
                    });
                }
                if cs.is_vague && !cs.is_wip {
                    operations.push(Operation::Reword {
                        oid: cs.oid.clone(),
                        old_subject: cs.subject.clone(),
                        suggested_subject: suggest_reword(&cs.subject),
                        reason: "Vague commit message".to_string(),
                    });
                }
            }
            crate::analyze::Severity::Info => {}
        }
    }

    let n = operations.len();
    let desc = if n == 0 {
        "No operations needed — history looks healthy.".into()
    } else {
        format!("{n} operations to clean up the history")
    };

    Plan {
        operation_count: n,
        commit_count: report.commits.len(),
        description: desc,
        operations,
    }
}

fn suggest_reword(subject: &str) -> String {
    match subject.to_lowercase().trim() {
        s if s.contains("wip") || s.contains("fix") && s.len() < 10 => {
            "feat: implement feature".into()
        }
        s if s.contains("update") => "refactor: update component".into(),
        s if s.contains("cleanup") => "chore: clean up code".into(),
        _ => format!("feat: {}", subject),
    }
}

pub fn format_plan_text(plan: &Plan) -> String {
    if plan.operations.is_empty() {
        return "✓ No operations needed — history looks healthy.".into();
    }

    let mut out = format!("Proposed cleanup plan ({} operations):\n", plan.operation_count);
    for (i, op) in plan.operations.iter().enumerate() {
        match op {
            Operation::Squash { target_oid, into_oid, subject, reason } => {
                out.push_str(&format!(
                    "  {i}. SQUASH  {target_oid} into {into_oid}\n     → {reason}: \"{subject}\"\n"
                ));
            }
            Operation::Reword { oid, old_subject, suggested_subject, reason } => {
                out.push_str(&format!(
                    "  {i}. REWORD  {oid}\n     → \"{old_subject}\" → \"{suggested_subject}\"\n     → {reason}\n"
                ));
            }
            Operation::Split { oid, subject, groups, reason } => {
                out.push_str(&format!(
                    "  {i}. SPLIT   {oid} \"{subject}\"\n     → {reason}\n"
                ));
                for g in groups {
                    out.push_str(&format!("       ∘ {g}\n"));
                }
            }
        }
    }
    out
}

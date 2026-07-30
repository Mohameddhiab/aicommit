use git_doctor::review::{run_code_review, ReviewSeverity};

#[test]
fn test_review_detects_aws_key() {
    let diff = r#"
diff --git a/config.js b/config.js
--- a/config.js
+++ b/config.js
@@ -1,3 +1,4 @@
+const awsKey = "AKIAIOSFODNN7EXAMPLE";
"#;

    let report = run_code_review(diff);
    assert!(!report.passed);
    assert!(report.score < 100);
    assert!(report.issues.iter().any(|i| i.severity == ReviewSeverity::Critical));
}

#[test]
fn test_review_detects_console_log() {
    let diff = r#"
diff --git a/app.js b/app.js
--- a/app.js
+++ b/app.js
@@ -5,3 +5,4 @@
+console.log("debug data:", data);
"#;

    let report = run_code_review(diff);
    assert!(report.passed); // Warning doesn't fail pass criteria unless critical
    assert!(report.issues.iter().any(|i| i.severity == ReviewSeverity::Warning));
}

#[test]
fn test_review_clean_diff_passes() {
    let diff = r#"
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -10,3 +10,4 @@
+pub fn clean_function() -> u32 { 42 }
"#;

    let report = run_code_review(diff);
    assert!(report.passed);
    assert_eq!(report.score, 100);
    assert!(report.issues.is_empty());
}

//! 验证统一差异、补丁摘要、代码块和文件预览的解析边界。
//! 文件头不计入增删统计，省略占位仅在精确匹配时被过滤以免吞掉真实内容。

use super::diff_utils::{
    count_apply_patch_format_changes, count_unified_diff_changes, extract_diff_block, file_preview,
    is_likely_file_path, looks_like_unified_diff, parse_apply_patch_line_changes,
    parse_apply_patch_summary, string_or_string_array,
};
use serde_json::json;

#[test]
fn unified_diff_detection_and_counts_ignore_headers() {
    let diff = "--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n-old\n+new\n";

    assert!(looks_like_unified_diff(diff));
    assert!(looks_like_unified_diff("\n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n"));
    assert!(!looks_like_unified_diff(""));
    assert!(!looks_like_unified_diff("--- a/a.rs\n+++ b/a.rs\n"));
    assert!(!looks_like_unified_diff("diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@"));
    assert_eq!(count_unified_diff_changes(diff), (1, 1));
    assert_eq!(
        count_unified_diff_changes("\n context\n+++ header\n--- header\n+one\n-two\n"),
        (1, 1)
    );
}

#[test]
fn apply_patch_summary_and_counts_parse_file_lines() {
    let summary_output = "M src/lib.rs\n";
    let count_output = "M src/lib.rs +1 -1\n";

    assert_eq!(parse_apply_patch_summary(summary_output), vec![('M', "src/lib.rs".to_string())]);
    assert_eq!(parse_apply_patch_line_changes(count_output), (1, 1));
    assert_eq!(
        count_apply_patch_format_changes("*** Update File: src/lib.rs\n+new\n-old\n"),
        (1, 1)
    );
    assert_eq!(
        count_apply_patch_format_changes("*** Begin Patch\n context\n++new\n--old\n"),
        (1, 1)
    );
    assert_eq!(
        parse_apply_patch_line_changes("A src/new.rs +7\nD src/old.rs -3\nM src/both.rs +5-2\n"),
        (12, 5)
    );
    assert_eq!(
        parse_apply_patch_line_changes(
            "note\nX src/nope.rs +10\nM src/lib.rs +bad -bad +2-NaN +NaN-4\n"
        ),
        (2, 4)
    );
    assert_eq!(parse_apply_patch_summary("X not/a/status\nM no/slash\n"), Vec::new());
    assert_eq!(
        parse_apply_patch_summary("The following files have been updated:\n- src/lib.rs\n\n"),
        vec![('M', "src/lib.rs".to_string())]
    );
    assert_eq!(
        parse_apply_patch_summary(
            "The following files have been updated:\n- https://example.test/a.rs\n- src/lib.rs\n\nM src/main.rs\n"
        ),
        vec![('M', "src/lib.rs".to_string()), ('M', "src/main.rs".to_string())]
    );
    assert_eq!(
        parse_apply_patch_summary("M plain\nD docs/readme.md\n"),
        vec![('D', "docs/readme.md".to_string())]
    );
}

#[test]
fn diff_helpers_extract_code_blocks_and_paths() {
    assert_eq!(extract_diff_block("<diff>\n+new\n</diff>"), Some("+new".to_string()));
    assert_eq!(extract_diff_block("<diff></diff>"), None);
    assert_eq!(extract_diff_block("<diff>body"), None);
    assert_eq!(extract_diff_block("missing"), None);
    assert!(is_likely_file_path("src/main.rs"));
    assert!(is_likely_file_path("/tmp/demo.txt"));
    assert!(is_likely_file_path(r"src\main.rs"));
    assert!(is_likely_file_path("./local"));
    assert!(is_likely_file_path("../parent"));
    assert!(!is_likely_file_path("plain words"));
    assert!(!is_likely_file_path(""));
    assert!(!is_likely_file_path("src/main.rs\nother"));
    assert!(!is_likely_file_path("https://example.test/main.rs"));
    assert!(!is_likely_file_path("http://example.test/main.rs"));
    assert!(!is_likely_file_path("src/<main>.rs"));
    assert!(!is_likely_file_path("src/has  double.rs"));
    assert!(!is_likely_file_path("src/has\tspace.rs"));
    assert!(!is_likely_file_path("filename"));
    assert_eq!(string_or_string_array(&json!(["a", "b"])), Some("a\nb".to_string()));
    assert_eq!(string_or_string_array(&json!(["a", 42, "b"])), Some("a\nb".to_string()));
    assert_eq!(string_or_string_array(&json!([42])), None);
    assert_eq!(string_or_string_array(&json!("single")), Some("single".to_string()));
    assert_eq!(string_or_string_array(&json!(42)), None);
}

#[test]
fn file_preview_ignores_omitted_write_placeholder() {
    let input = r#"{"path":"docs/demo.md","content":"<omitted 812 chars>"}"#;

    assert_eq!(file_preview("file_write", input, ""), None);
    assert_eq!(file_preview("write", r#"{"content":"hello"}"#, ""), Some("hello".to_string()));
    assert_eq!(
        file_preview("write", r#"{"content":"<omitted chars>"}"#, ""),
        Some("<omitted chars>".to_string())
    );
    assert_eq!(
        file_preview("file_edit", r#"{"new_string":"snake"}"#, ""),
        Some("snake".to_string())
    );
    assert_eq!(file_preview("file_edit", r#"{"newString":"new"}"#, ""), Some("new".to_string()));
    assert_eq!(
        file_preview("file_edit", r#"{"newString":"<omitted abc chars>"}"#, ""),
        Some("<omitted abc chars>".to_string())
    );
    assert_eq!(
        file_preview("notebook_edit", r#"{"new_code":["a","b"]}"#, ""),
        Some("a\nb".to_string())
    );
    assert_eq!(
        file_preview("notebook_edit", r#"{"newCode":"single"}"#, ""),
        Some("single".to_string())
    );
    assert_eq!(
        file_preview("apply_patch", "*** Begin Patch\n*** End Patch", ""),
        Some("*** Begin Patch\n*** End Patch".to_string())
    );
    assert_eq!(file_preview("apply_patch", "   ", ""), None);
    assert_eq!(
        file_preview("apply_patch", "fallback", "<diff>\n+from output\n</diff>"),
        Some("+from output".to_string())
    );
    assert_eq!(file_preview("apply_patch", "fallback", "<diff>\n\n</diff>"), None);
    assert_eq!(
        file_preview("read", "", "<diff>\n--- a/a\n+++ b/a\n@@\n-old\n+new\n</diff>"),
        Some("--- a/a\n+++ b/a\n@@\n-old\n+new".to_string())
    );
    assert_eq!(file_preview("read", "", "<diff>\n\n</diff>"), None);
    assert_eq!(
        file_preview("read", "", "--- a/a\n+++ b/a\n@@\n-old\n+new\n"),
        Some("--- a/a\n+++ b/a\n@@\n-old\n+new".to_string())
    );
    assert_eq!(file_preview("read", "not json", ""), None);
    assert_eq!(file_preview("write", "{bad", ""), None);
    assert_eq!(file_preview("unknown", r#"{"content":"hello"}"#, ""), None);
}

#[test]
fn file_preview_keeps_non_char_omission_markers_as_content() {
    assert_eq!(
        file_preview("write", r#"{"content":"<omitted 12 bytes>"}"#, ""),
        Some("<omitted 12 bytes>".to_string())
    );
}

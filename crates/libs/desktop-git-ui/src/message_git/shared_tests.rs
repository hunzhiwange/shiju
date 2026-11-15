use crate::git_panel::diff_view::GitDiffLineRange;
#[test]
fn text_too_large_for_code_editor_detects_limits_and_nul() {
    assert!(!super::text_too_large_for_code_editor("small\ntext"));
    assert!(super::text_too_large_for_code_editor(&"a".repeat(1_000_001)));
    assert!(super::text_too_large_for_code_editor(&format!("{}\n", "a".repeat(20_001))));
    assert!(super::text_too_large_for_code_editor("abc\0def"));
}
#[test]
fn text_too_large_for_code_editor_accepts_exact_limits_and_rejects_extra_line() {
    let exact_max_bytes =
        format!("{}{}", ("a".repeat(20_000) + "\n").repeat(49), "a".repeat(19_951));

    assert_eq!(exact_max_bytes.len(), 1_000_000);
    assert!(!super::text_too_large_for_code_editor(&exact_max_bytes));
    assert!(!super::text_too_large_for_code_editor(&"a".repeat(20_000)));
    assert!(!super::text_too_large_for_code_editor(&"x\n".repeat(20_001)));
    assert!(super::text_too_large_for_code_editor(&"x\n".repeat(20_002)));
}
#[test]
fn normalize_range_orders_start_and_end() {
    let range = super::normalize_range(GitDiffLineRange {
        file: "src/lib.rs".to_string(),
        start: 9,
        end: 3,
        is_old: true,
    });

    assert_eq!(range.start, 3);
    assert_eq!(range.end, 9);
    assert!(range.is_old);
}
#[test]
fn normalize_range_preserves_an_ordered_single_line_range() {
    let range = super::normalize_range(GitDiffLineRange {
        file: "src/lib.rs".to_string(),
        start: 4,
        end: 4,
        is_old: false,
    });

    assert_eq!(range.start, 4);
    assert_eq!(range.end, 4);
    assert!(!range.is_old);
}

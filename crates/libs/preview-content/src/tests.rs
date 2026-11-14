//! 预览文本内核回归测试。

use super::*;

#[test]
fn strip_ansi_and_controls_removes_escape_sequences_and_preserves_layout() {
    let input = concat!(
        "plain\x1b[31mred\x1b[0m ",
        "\x1b]0;title\x07after-bel ",
        "\x1b]8;;https://example.invalid\x1b\\link\x1b]8;;\x1b\\ ",
        "\x1bXignored\x00\x08\x7f\n\t中文"
    );

    assert_eq!(strip_ansi_and_controls(input), "plainred after-bel link ignored\n\t中文");
}

#[test]
fn strip_ansi_and_controls_discards_unterminated_and_lone_escape_sequences() {
    assert_eq!(strip_ansi_and_controls("before\x1b[31"), "before");
    assert_eq!(strip_ansi_and_controls("before\x1b]title"), "before");
    assert_eq!(strip_ansi_and_controls("before\x1b"), "before");
}

#[test]
fn wrap_long_lines_counts_characters_and_resets_after_newlines() {
    assert_eq!(wrap_long_lines("甲乙丙丁\nabcdx", 4), "甲乙丙丁\n\nabcd\nx");
    assert_eq!(wrap_long_lines("", 4), "");
}

#[test]
fn truncate_to_char_boundary_preserves_complete_utf8_characters() {
    assert_eq!(truncate_to_char_boundary("a中b", 5), ("a中b", false));
    assert_eq!(truncate_to_char_boundary("a中b", 4), ("a中", true));
    assert_eq!(truncate_to_char_boundary("a中b", 2), ("a", true));
    assert_eq!(truncate_to_char_boundary("中", 0), ("", true));
}

#[test]
fn format_text_preview_content_cleans_controls_and_wraps_long_lines() {
    let content = format!("\x1b[32m{}\x1b[0m\x00", "x".repeat(PREVIEW_MAX_LINE + 1));

    let (formatted, truncated) = format_text_preview_content(&content);

    assert!(!truncated);
    assert_eq!(formatted, format!("{}\nx\n", "x".repeat(PREVIEW_MAX_LINE)));
}

#[test]
fn format_text_preview_content_truncates_at_byte_and_character_boundary() {
    let content = format!("{}中", "a".repeat(PREVIEW_MAX_BYTES - 1));

    let (formatted, truncated) = format_text_preview_content(&content);

    assert!(truncated);
    assert!(formatted.starts_with("文件较大，预览已截断。\n\n"));
    assert!(!formatted.contains('中'));
}

#[test]
fn format_text_preview_content_limits_output_lines() {
    let content = "line\n".repeat(PREVIEW_MAX_LINES + 1);

    let (formatted, truncated) = format_text_preview_content(&content);

    assert!(truncated);
    assert!(formatted.starts_with("文件较大，预览已截断。\n\n"));
    assert_eq!(formatted.lines().filter(|line| *line == "line").count(), PREVIEW_MAX_LINES);
}

#[test]
fn format_text_preview_content_accepts_exact_line_limit_without_truncation() {
    let content = "line\n".repeat(PREVIEW_MAX_LINES);

    let (formatted, truncated) = format_text_preview_content(&content);

    assert!(!truncated);
    assert_eq!(formatted.lines().filter(|line| *line == "line").count(), PREVIEW_MAX_LINES);
    assert!(!formatted.starts_with("文件较大，预览已截断。"));
}

#[test]
fn compute_search_matches_skips_empty_terms_and_sorts_byte_positions() {
    let terms = vec!["文".to_string(), "ana".to_string(), String::new(), "ban".to_string()];

    assert_eq!(
        compute_search_matches("banana 文文\nban", &terms),
        vec![(0, 0, 3), (0, 1, 3), (0, 7, 3), (0, 10, 3), (1, 0, 3)]
    );
    assert!(compute_search_matches("anything", &[]).is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn path_to_file_uri_encodes_spaces() {
    assert_eq!(path_to_file_uri(std::path::Path::new("/tmp/a b.rs")), "file:///tmp/a%20b.rs");
}

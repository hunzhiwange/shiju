use super::*;

#[test]
fn parse_markdown_heading_extracts_level_and_normalized_title() {
    assert_eq!(
        parse_markdown_heading("### Release notes ###\r"),
        Some(MarkdownHeading { level: 3, title: "Release notes".to_string() })
    );
}

#[test]
fn parse_markdown_heading_rejects_missing_separator_excess_depth_and_blank_title() {
    assert_eq!(parse_markdown_heading("#missing separator"), None);
    assert_eq!(parse_markdown_heading("####### too deep"), None);
    assert_eq!(parse_markdown_heading("#  "), None);
}

#[test]
fn fence_marker_accepts_indented_backticks_only_at_line_start() {
    assert!(is_fence_marker("   ```rust"));
    assert!(is_fence_marker("````"));
    assert!(!is_fence_marker("text ```"));
    assert!(!is_fence_marker("~~~rust"));
}

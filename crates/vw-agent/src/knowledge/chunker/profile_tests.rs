// 验证切块配置档的默认值、覆盖规则与参数约束。

// profile 先统计标题密度和层级，再决定是否启用标题优先切分。
use super::DocProfile;

#[test]
fn empty_document_has_no_lines_headings_or_split_strategy() {
    let profile = DocProfile::from_text("");

    assert_eq!(profile.total_chars, 0);
    assert_eq!(profile.total_lines, 0);
    assert_eq!(profile.markdown_heading_counts, [0; 6]);
    assert_eq!(profile.dominant_heading_level(), None);
    assert!(!profile.should_use_heading_split());
}

#[test]
fn fenced_heading_markers_are_excluded_from_document_profile() {
    let profile = DocProfile::from_text(
        "# Visible\n```markdown\n## Hidden\n### Also hidden\n```\n## Visible too",
    );

    assert_eq!(profile.markdown_heading_counts, [1, 1, 0, 0, 0, 0]);
    assert_eq!(profile.markdown_heading_total, 2);
    assert_eq!(profile.dominant_heading_level(), Some(2));
    assert!(!profile.should_use_heading_split());
}

#[test]
fn repeated_shallower_heading_wins_over_deeper_fallback() {
    let text = "# One\nbody\n# Two\nbody\n# Three\nbody\n### Detail";
    let profile = DocProfile::from_text(text);

    assert_eq!(profile.markdown_heading_counts, [3, 0, 1, 0, 0, 0]);
    assert_eq!(profile.dominant_heading_level(), Some(1));
    assert!(profile.should_use_heading_split());
}

#[test]
fn sparse_headings_do_not_enable_heading_split() {
    let mut lines = vec!["# One", "# Two", "# Three"];
    lines.extend(std::iter::repeat_n("body", 600));
    let profile = DocProfile::from_text(&lines.join("\n"));

    assert_eq!(profile.markdown_heading_total, 3);
    assert_eq!(profile.dominant_heading_level(), Some(1));
    assert!(!profile.should_use_heading_split());
}

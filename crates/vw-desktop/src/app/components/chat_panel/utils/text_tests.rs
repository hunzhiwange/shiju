use super::text::{
    normalize_display_text, single_line_preview, strip_internal_tool_trace, truncate_chars,
    truncate_lines_middle,
};
use std::borrow::Cow;

#[test]
fn truncate_chars_and_lines_keep_boundaries() {
    assert_eq!(truncate_chars("abcdef", 3), "abc…");
    assert_eq!(truncate_lines_middle("a\nb\nc\nd", 2, 10), "a\n…\nd");
}

#[test]
fn truncate_chars_counts_unicode_scalars_and_handles_zero_limit() {
    assert_eq!(truncate_chars("你好世界", 2), "你好…");
    assert_eq!(truncate_chars("内容", 0), "…");
}

#[test]
fn single_line_preview_collapses_whitespace_before_truncating() {
    assert_eq!(single_line_preview("  curl \\\n  -H\t'x: y'  ", 80), "curl \\ -H 'x: y'");
    assert_eq!(single_line_preview("一 二 三", 3), "一 二…");
}

#[test]
fn truncate_lines_middle_keeps_asymmetric_head_and_tail_and_truncates_each_line() {
    let text = "abc\ndef\nghi\njkl\nmno";

    assert_eq!(truncate_lines_middle(text, 3, 2), "ab…\n…\njk…\nmn…");
}

#[test]
fn normalize_display_text_collapses_excess_blank_lines() {
    assert_eq!(normalize_display_text("a\n\n\nb").as_ref(), "a\n\nb");
}

#[test]
fn normalize_display_text_preserves_code_fences() {
    let text = "head\n\n```\nline  \n\n```\n\nfoot";

    assert_eq!(normalize_display_text(text).as_ref(), "head\n\n```\nline  \n\n```\n\nfoot");
}

#[test]
fn normalize_display_text_borrows_text_that_needs_no_changes() {
    let text = "first\nsecond";

    assert!(matches!(normalize_display_text(text), Cow::Borrowed(value) if value == text));
}

#[test]
fn normalize_display_text_removes_cr_and_trailing_horizontal_whitespace() {
    let text = "head \r\n\r\n\r\ntail\t";

    assert_eq!(normalize_display_text(text).as_ref(), "head\n\ntail");
}

#[test]
fn strip_internal_tool_trace_returns_plain_text_without_markers() {
    let text = "plain answer\nwith normal content";

    assert_eq!(strip_internal_tool_trace(text), text);
    assert_eq!(strip_internal_tool_trace(""), "");
}

#[test]
fn strip_internal_tool_trace_removes_compact_tool_lines() {
    let stripped = strip_internal_tool_trace("tool bash(command=\"ls\")\nvisible");

    assert!(stripped.contains("visible"));
    assert!(!stripped.contains("tool bash"));
}

#[test]
fn strip_internal_tool_trace_drops_verbose_tool_leaks() {
    let stripped = strip_internal_tool_trace("Called the read tool\nmetadata\nvisible output");

    assert_eq!(stripped, "visible output");
}

#[test]
fn strip_internal_tool_trace_drops_called_tool_input_until_blank() {
    let trace =
        "intro\nCalled the shell tool with the following input:\n{\"command\":\"pwd\"}\n\nvisible";

    assert_eq!(strip_internal_tool_trace(trace), "intro\nvisible");
}

#[test]
fn strip_internal_tool_trace_removes_content_and_file_dump_bodies() {
    let trace =
        "before\n<content>\nhidden content\n</content>\nafter\n<file>\nhidden file\n</file>\nend";

    assert_eq!(strip_internal_tool_trace(trace), "before\nafter\nend");
}

#[test]
fn strip_internal_tool_trace_removes_standalone_file_dump_bodies() {
    let trace = "before\n<file>\nsecret\n</file>\nafter";

    assert_eq!(strip_internal_tool_trace(trace), "before\nafter");
}

#[test]
fn strip_internal_tool_trace_removes_inline_metadata_tags_and_line_ranges() {
    let trace = "before\n<path>/tmp/demo</path>\n<type>file</type>\n<file_link>demo</file_link>\n(showing lines 1-2 of 3)\nafter";

    assert_eq!(strip_internal_tool_trace(trace), "before\nafter");
}

#[test]
fn strip_internal_tool_trace_removes_output_caps_and_schema_errors() {
    let trace = "before\n(output capped at 2000 chars)\n(end of file)\nmissing required field `path`\nexpected schema\noutputPath: /tmp/out\nafter";

    assert_eq!(strip_internal_tool_trace(trace), "before\nafter");
}

#[test]
fn strip_internal_tool_trace_recognizes_fullwidth_tool_markers() {
    let trace = "ｔｏｏｌ　ｂａｓｈ(command=\"pwd\")\nvisible";

    assert_eq!(strip_internal_tool_trace(trace), "visible");
}

#[test]
fn strip_internal_tool_trace_is_stable_for_repeated_input() {
    let trace = "tool\nmetadata\n\nvisible after trace";

    assert_eq!(strip_internal_tool_trace(trace), "visible after trace");
    assert_eq!(strip_internal_tool_trace(trace), "visible after trace");
}

use super::think::split_think;
#[test]
fn split_think_removes_closed_think_and_reports_visible_text() {
    let (thinks, visible, open) = split_think("hello<think>secret</think> world");
    assert_eq!(thinks, vec!["secret".to_string()]);
    assert_eq!(visible, "hello world");
    assert!(!open);
}

#[test]
fn split_think_treats_reasoning_marker_as_closed_think() {
    let (thinks, visible, open) = split_think("answer\n\n[reasoning]\nchain");

    assert_eq!(thinks, vec!["chain".to_string()]);
    assert_eq!(visible, "answer");
    assert!(!open);
}

#[test]
fn split_think_strips_valid_tool_json_blocks_from_visible_text() {
    let raw = r#"before
tool call
{"ok":true}
after"#;
    let (thinks, visible, open) = split_think(raw);
    assert!(thinks.is_empty());
    assert_eq!(
        visible,
        "before
after"
    );
    assert!(!open);
}

#[test]
fn split_think_reports_open_think_when_close_tag_is_missing() {
    let (thinks, visible, open) = split_think("visible <think>still thinking");
    assert_eq!(thinks, vec!["still thinking".to_string()]);
    assert_eq!(visible, "visible ");
    assert!(open);
}

#[test]
fn split_think_truncates_closed_think_at_tool_block() {
    let raw = r#"before<think>secret
tool call
{"ok":true}
</think>after"#;
    let (thinks, visible, open) = split_think(raw);
    assert_eq!(thinks, vec!["secret\n".to_string()]);
    assert_eq!(visible, "beforeafter");
    assert!(!open);
}

#[test]
fn split_think_ignores_tags_with_non_whitespace_suffixes() {
    let (thinks, visible, open) = split_think("a <thinker>nope</thinker> b");
    assert!(thinks.is_empty());
    assert_eq!(visible, "a <thinker>nope</thinker> b");
    assert!(!open);
}

#[test]
fn split_think_accepts_thinking_alias_tags() {
    let (thinks, visible, open) = split_think("a <thinking>plan next step</thinking> b");
    assert_eq!(thinks, vec!["plan next step".to_string()]);
    assert_eq!(visible, "a  b");
    assert!(!open);
}

#[test]
fn split_think_accepts_mixed_think_and_thinking_close_tags() {
    let (thinks, visible, open) = split_think("<thinking>one</think><think>two</thinking>");
    assert_eq!(thinks, vec!["one".to_string(), "two".to_string()]);
    assert!(visible.is_empty());
    assert!(!open);
}

#[test]
fn split_think_normalizes_repeated_blank_lines_outside_code_fences() {
    let raw = "a  \n\n\n```rust\nfn main() { }  \n```\n\n b\t\n";
    let (_thinks, visible, _open) = split_think(raw);
    assert_eq!(visible, "a\n```rust\nfn main() { }  \n```\n\n b");
}

#[test]
fn split_think_returns_plain_text_without_think_chunks() {
    let (thinks, visible, open) = split_think("plain text");
    assert!(thinks.is_empty());
    assert_eq!(visible, "plain text");
    assert!(!open);
}

#[test]
fn split_think_collects_multiple_blocks_with_whitespace_tag_attributes() {
    let raw = "before<think data-kind=\"first\">one</think done>middle<think>two</think>after";
    let (thinks, visible, open) = split_think(raw);
    assert_eq!(thinks, vec!["one".to_string(), "two".to_string()]);
    assert_eq!(visible, "beforemiddleafter");
    assert!(!open);
}

#[test]
fn split_think_keeps_malformed_open_tag_visible() {
    let raw = "before <think without an end";
    let (thinks, visible, open) = split_think(raw);
    assert!(thinks.is_empty());
    assert_eq!(visible, raw);
    assert!(!open);
}

#[test]
fn split_think_treats_malformed_close_tag_as_open_content() {
    let (thinks, visible, open) = split_think("before<think>secret</think");
    assert_eq!(thinks, vec!["secret</think".to_string()]);
    assert_eq!(visible, "before");
    assert!(open);
}

#[test]
fn split_think_reports_empty_unclosed_block_without_empty_chunk() {
    let (thinks, visible, open) = split_think("before<think>");
    assert!(thinks.is_empty());
    assert_eq!(visible, "before");
    assert!(open);
}

#[test]
fn split_think_removes_orphan_close_tag_before_visible_text() {
    let (thinks, visible, open) = split_think("</think>");
    assert!(thinks.is_empty());
    assert!(visible.is_empty());
    assert!(!open);

    let (thinks, visible, open) = split_think("</think>answer");
    assert!(thinks.is_empty());
    assert_eq!(visible, "answer");
    assert!(!open);
}

#[test]
fn split_think_removes_orphan_close_tag_before_later_think_block() {
    let raw = "before</think>middle<think>secret</think>after";
    let (thinks, visible, open) = split_think(raw);
    assert_eq!(thinks, vec!["secret".to_string()]);
    assert_eq!(visible, "beforemiddleafter");
    assert!(!open);
}

#[test]
fn split_think_preserves_non_tag_close_text() {
    let raw = "before </thinker> after";
    let (thinks, visible, open) = split_think(raw);
    assert!(thinks.is_empty());
    assert_eq!(visible, raw);
    assert!(!open);
}

#[test]
fn split_think_normalizes_crlf_and_whitespace_only_blank_lines() {
    let raw = "first  \r\n \r\n\t\r\nsecond\t\r\n";
    let (thinks, visible, open) = split_think(raw);
    assert!(thinks.is_empty());
    assert_eq!(visible, "first\n\nsecond");
    assert!(!open);
}

#[test]
fn split_think_strips_multiline_and_consecutive_tool_json_blocks() {
    let raw =
        concat!("tool first\n", "{\n  \"ok\": true\n}\n", "tool second\n", "[1, 2, 3]\n", "after");
    let (thinks, visible, open) = split_think(raw);
    assert!(thinks.is_empty());
    assert_eq!(visible, "after");
    assert!(!open);
}

#[test]
fn split_think_strips_tool_json_value_at_end_of_input() {
    let (thinks, visible, open) = split_think("tool call\ntrue");
    assert!(thinks.is_empty());
    assert!(visible.is_empty());
    assert!(!open);
}

#[test]
fn split_think_preserves_tool_header_without_json_line() {
    let raw = "before\ntool call";
    let (thinks, visible, open) = split_think(raw);
    assert!(thinks.is_empty());
    assert_eq!(visible, raw);
    assert!(!open);
}

#[test]
fn split_think_preserves_invalid_tool_block_and_remaining_text() {
    let raw = concat!(
        "before\n",
        "tool broken\n",
        "not-json\n",
        "tool valid\n",
        "{\"ok\":true}\n",
        "after"
    );
    let (thinks, visible, open) = split_think(raw);
    assert!(thinks.is_empty());
    assert_eq!(visible, raw);
    assert!(!open);
}

#[test]
fn split_think_does_not_strip_inline_tool_marker() {
    let raw = "before tool call\n{\"ok\":true}\nafter";
    let (thinks, visible, open) = split_think(raw);
    assert!(thinks.is_empty());
    assert_eq!(visible, raw);
    assert!(!open);
}

#[test]
fn split_think_preserves_tool_payload_beyond_bounded_json_scan() {
    let mut raw = String::from("tool call\n");
    for _ in 0..65 {
        raw.push_str("not-json\n");
    }
    raw.push_str("{\"ok\":true}");

    let (thinks, visible, open) = split_think(&raw);

    assert!(thinks.is_empty());
    assert_eq!(visible, raw);
    assert!(!open);
}

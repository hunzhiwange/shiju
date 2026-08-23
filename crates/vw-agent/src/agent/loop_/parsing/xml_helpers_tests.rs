//! 测试 XML 辅助解析对标签、转义文本与不完整输入的处理。

use serde_json::json;

use super::xml_helpers::{
    extract_xml_pairs, find_first_tag, is_xml_meta_tag, matching_tool_call_close_tag,
    parse_xml_tool_calls, tool_call_close_tags, tool_call_open_tags,
};

#[test]
fn xml_meta_tags_are_case_insensitive() {
    assert!(is_xml_meta_tag("Thinking"));
    assert!(is_xml_meta_tag("tool-call"));
    assert!(is_xml_meta_tag("REFLECTION"));
    assert!(!is_xml_meta_tag("shell"));
}

#[test]
fn extract_xml_pairs_keeps_trimmed_inner_content() {
    let pairs = extract_xml_pairs("<shell> pwd </shell><note> ok </note>");

    assert_eq!(pairs, vec![("shell", "pwd"), ("note", "ok")]);
}

#[test]
fn extract_xml_pairs_skips_unclosed_and_continues_after_open_tag() {
    let pairs = extract_xml_pairs("<broken><shell> pwd </shell><tail> ok </tail>");

    assert_eq!(pairs, vec![("shell", "pwd"), ("tail", "ok")]);
}

#[test]
fn parse_xml_tool_calls_supports_json_and_nested_args() {
    let calls = parse_xml_tool_calls(
        r#"<shell>{"command":"pwd"}</shell><memory_recall><query>abc</query></memory_recall>"#,
    )
    .expect("tool calls should parse");

    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments, json!({"command": "pwd"}));
    assert_eq!(calls[1].arguments["query"], "abc");
}

#[test]
fn parse_xml_tool_calls_wraps_non_object_json_and_plain_text() {
    let calls =
        parse_xml_tool_calls(r#"<score>42</score><note> remember this </note><flag>true</flag>"#)
            .expect("tool calls should parse");

    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].arguments, json!({"value": 42}));
    assert_eq!(calls[1].arguments, json!({"content": "remember this"}));
    assert_eq!(calls[2].arguments, json!({"value": true}));
}

#[test]
fn parse_xml_tool_calls_skips_meta_empty_and_meta_arguments() {
    let calls = parse_xml_tool_calls(
        r#"
        <thinking>ignore</thinking>
        <shell><analysis>skip</analysis><command>date</command></shell>
        <empty>   </empty>
        "#,
    )
    .expect("non-meta tool call should parse");

    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments, json!({"command": "date"}));
}

#[test]
fn parse_xml_tool_calls_returns_none_for_non_xml_or_empty_results() {
    assert!(parse_xml_tool_calls("plain text").is_none());
    assert!(parse_xml_tool_calls("<thinking>only meta</thinking>").is_none());
    assert!(parse_xml_tool_calls("<shell>   </shell>").is_none());
}

#[test]
fn tool_call_tags_have_matching_close_tags() {
    let (_, open) = find_first_tag("<invoke>{}</invoke>", tool_call_open_tags())
        .expect("open tag should be found");

    assert_eq!(matching_tool_call_close_tag(open), Some("</invoke>"));
    assert!(tool_call_close_tags().contains(&"</invoke>"));
}

#[test]
fn find_first_tag_picks_earliest_tag_and_rejects_unknown_open_tags() {
    let (idx, tag) =
        find_first_tag("xx <invoke>{}</invoke> <toolcall>{}</toolcall>", tool_call_open_tags())
            .expect("open tag should be found");

    assert_eq!(idx, 3);
    assert_eq!(tag, "<invoke>");
    assert_eq!(matching_tool_call_close_tag("<unknown>"), None);
}

#[test]
fn all_open_tags_map_to_expected_close_tags() {
    for (open, close) in tool_call_open_tags().iter().zip(tool_call_close_tags()) {
        assert_eq!(matching_tool_call_close_tag(open), Some(*close));
    }
}

use super::tool_tags::strip_tool_call_tags;

#[test]
fn strip_tool_call_tags_removes_current_and_legacy_tags() {
    assert_eq!(strip_tool_call_tags("<tool_call>secret</tool_call>visible"), "visible");
    assert_eq!(strip_tool_call_tags("erte {\"tool\":\"x\"} ttrivisible"), "visible");
}

#[test]
fn strip_tool_call_tags_removes_legacy_array_json_and_multiple_tags() {
    let input = "before erte [{\"tool\":\"a\"}] ttri middle erte {\"tool\":\"b\"} ttri after";

    assert_eq!(strip_tool_call_tags(input), "before  middle  after");
}

#[test]
fn strip_tool_call_tags_removes_legacy_tag_with_mixed_whitespace() {
    let input = "before erte \u{3000}{\"tool\":\"x\"}\n\t ttri after";

    assert_eq!(strip_tool_call_tags(input), "before  after");
}

#[test]
fn strip_tool_call_tags_preserves_legacy_prefix_when_json_is_missing_or_invalid() {
    assert_eq!(strip_tool_call_tags("hello erte not-json ttri"), "hello erte not-json ttri");
    assert_eq!(strip_tool_call_tags("hello erte {broken} ttri"), "hello erte {broken} ttri");
}

#[test]
fn strip_tool_call_tags_continues_scanning_after_invalid_legacy_prefix() {
    let input = "a erte not-json b erte {\"tool\":\"x\"} ttri c";

    assert_eq!(strip_tool_call_tags(input), "a erte not-json b  c");
}

#[test]
fn strip_tool_call_tags_preserves_text_when_legacy_end_marker_is_wrong() {
    assert_eq!(
        strip_tool_call_tags("alpha erte {\"tool\":\"x\"} nope omega"),
        "alpha erte {\"tool\":\"x\"} nope omega"
    );
}

#[test]
fn strip_tool_call_tags_drops_complete_legacy_tag_at_end_of_input() {
    assert_eq!(strip_tool_call_tags("visible erte {\"tool\":\"x\"} ttri"), "visible");
}

#[test]
fn strip_tool_call_tags_handles_incomplete_legacy_tag_at_end() {
    assert_eq!(strip_tool_call_tags("visible erte {\"tool\":\"x\"}"), "visible");
    assert_eq!(strip_tool_call_tags("visible erte"), "visible erte");
}

#[test]
fn strip_tool_call_tags_preserves_plain_text_without_tags() {
    assert_eq!(strip_tool_call_tags("plain message"), "plain message");
    assert_eq!(strip_tool_call_tags(""), "");
}

#[test]
fn strip_tool_call_tags_trims_trailing_newlines_after_standard_cleanup() {
    assert_eq!(strip_tool_call_tags("visible\n<tool_call>{}</tool_call>\n"), "visible");
}

#[test]
fn strip_tool_call_tags_breaks_when_only_whitespace_follows_legacy_marker() {
    // "erte" 后只有空白字符直到输入末尾：跳过空白后没有有效字符，保留 "erte" 前缀。
    assert_eq!(strip_tool_call_tags("done erte   "), "done erte");
    assert_eq!(strip_tool_call_tags("done erte\n\t"), "done erte");
}

#[test]
fn strip_tool_call_tags_removes_legacy_tag_without_surrounding_whitespace() {
    // JSON 紧贴 "erte" 与 "ttri"，无空白分隔。
    assert_eq!(strip_tool_call_tags("pre erte{\"a\":1}ttri post"), "pre  post");
}

#[test]
fn strip_tool_call_tags_drops_complete_legacy_array_tag_at_end_of_input() {
    // 数组 JSON 的完整标签对，且 "ttri" 后紧跟输入末尾。
    assert_eq!(strip_tool_call_tags("keep erte [1,2,3] ttri"), "keep");
}

#[test]
fn strip_tool_call_tags_keeps_legacy_marker_when_no_text_follows_json() {
    // 完整 JSON 后无结束标记，且 JSON 后还有非空白文本但非 "ttri"。
    assert_eq!(strip_tool_call_tags("x erte {\"a\":1} y"), "x erte {\"a\":1} y");
}

#[test]
fn strip_tool_call_tags_handles_consecutive_legacy_tags() {
    let input = "erte {\"a\":1} ttrierte {\"b\":2} ttritail";

    assert_eq!(strip_tool_call_tags(input), "tail");
}

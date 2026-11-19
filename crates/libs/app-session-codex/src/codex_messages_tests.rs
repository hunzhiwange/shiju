use super::*;

#[test]
fn sniff_types_handles_short_and_long_records() {
    assert_eq!(sniff_types(br#"{"type":"response_item","payload":{"type":"message"}}"#), Some(("response_item", Some("message"))));
    assert_eq!(sniff_types(&vec![b' '; SNIFF_BYTES + 1]), None);
    assert!(INJECTED_USER_PREFIXES.iter().any(|prefix| "# AGENTS.md instructions for project".starts_with(prefix)));
}

#[test]
fn custom_tool_input_with_trailing_newline_keeps_its_source_reference() {
    let line = r#"{"type":"response_item","payload":{"type":"custom_tool_call","name":"exec","call_id":"call-1","input":"// 中文\n"}}"#.as_bytes();
    let span = JsonlSpan {
        offset: 0,
        len: line.len() as u32,
    };
    let mut parser = RolloutParser::default();
    parser.process_line(line, span);
    let messages = parser.finish();
    let call = messages
        .iter()
        .flat_map(|message| &message.blocks)
        .find(|block| matches!(block, SessionBlock::ToolCall { .. }));
    let Some(SessionBlock::ToolCall {
        input_preview,
        input_total_len,
        input_full,
        ..
    }) = call else {
        panic!("expected a tool call");
    };

    assert_eq!(input_preview, "// 中文");
    assert_eq!(*input_total_len, 6);
    assert_eq!(input_full.as_ref(), Some(&span.content_ref("/payload/input")));
}

use super::*;
use crate::types::{
    AcpJsonRpcMessage, OutputErrorCode, OutputErrorParams, OutputFormat, OutputFormatter,
};
use serde_json::{Value, json};

fn acp_message(value: Value) -> AcpJsonRpcMessage {
    serde_json::from_value(value).expect("valid ACP JSON-RPC message")
}

fn output_error(message: &str) -> OutputErrorParams {
    OutputErrorParams {
        code: OutputErrorCode::Runtime,
        detail_code: None,
        origin: None,
        message: message.to_string(),
        retryable: None,
        acp: None,
        timestamp: None,
    }
}

#[test]
fn any_text_formatter_delegates_message_updates() {
    let mut formatter = create_output_formatter(
        OutputFormat::Text,
        Vec::<u8>::new(),
        OutputFormatterOptions::default(),
    );

    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": "text branch" }
            }
        }
    })));
    formatter.flush();

    let output = String::from_utf8(formatter.into_inner()).expect("output is UTF-8");
    assert_eq!(output, "text branch\n");
}

#[test]
fn text_write_line_preserves_pending_chunk_boundary() {
    let mut formatter =
        TextOutputFormatter::new(Vec::<u8>::new(), OutputFormatterOptions::default());
    formatter.chunk_buffer.push_str("partial");

    formatter.write_line("next");

    let output = String::from_utf8(formatter.into_inner()).expect("output is UTF-8");
    assert_eq!(output, "\nnext\n");
}

#[test]
fn text_flush_chunk_buffer_clears_newline_terminated_chunks() {
    let mut formatter =
        TextOutputFormatter::new(Vec::<u8>::new(), OutputFormatterOptions::default());
    formatter.chunk_buffer.push_str("partial\n");

    formatter.flush_chunk_buffer();

    assert!(formatter.chunk_buffer.is_empty());
    assert!(formatter.into_inner().is_empty());
}

#[test]
fn text_tool_update_uses_default_title_for_running_tools() {
    let mut formatter =
        TextOutputFormatter::new(Vec::<u8>::new(), OutputFormatterOptions::default());

    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "tool_call",
                "toolCallId": "tool-default"
            }
        }
    })));
    formatter.flush();

    let output = String::from_utf8(formatter.into_inner()).expect("output is UTF-8");
    assert_eq!(output, "[tool] Tool (running)\n");
}

#[test]
fn text_formatter_hides_repeated_background_loads_and_resumes() {
    let mut formatter =
        TextOutputFormatter::new(Vec::<u8>::new(), OutputFormatterOptions::default());

    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/load",
        "id": "load-1"
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/load",
        "id": "load-2"
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/resume",
        "id": "resume-1"
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "id": 99,
        "result": {}
    })));
    formatter.flush();

    let output = String::from_utf8(formatter.into_inner()).expect("output is UTF-8");
    assert_eq!(output.matches("[client]").count(), 1);
    assert!(output.contains("session/load"));
    assert!(!output.contains("session/resume"));
}

#[test]
fn text_output_helpers_cover_absent_output_and_empty_terminal_content() {
    assert_eq!(
        render_tool_output(false, ToolDescriptorView { title: None, kind: None }, None, &[],),
        None
    );

    let content = vec![json!({ "type": "terminal", "output": "   " })];
    assert_eq!(summarize_tool_content(&content), None);
}

#[test]
fn quiet_formatter_ignores_messages_without_prompt_content() {
    let mut formatter = QuietOutputFormatter::new(Vec::<u8>::new(), None);

    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {}
    })));
    formatter.on_error(output_error("ignored"));
    formatter.flush();

    assert!(formatter.into_inner().is_empty());
}

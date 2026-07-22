use super::*;
use crate::types::{
    AcpJsonRpcMessage, OutputErrorCode, OutputErrorParams, OutputFormat, OutputFormatter,
    OutputFormatterContext,
};
use serde_json::{Value, json};

fn acp_message(value: Value) -> AcpJsonRpcMessage {
    serde_json::from_value(value).expect("valid ACP JSON-RPC message")
}

fn output_text(formatter: TextOutputFormatter<Vec<u8>>) -> String {
    String::from_utf8(formatter.into_inner()).expect("formatter writes UTF-8")
}

fn quiet_output_text(formatter: QuietOutputFormatter<Vec<u8>>) -> String {
    String::from_utf8(formatter.into_inner()).expect("formatter writes UTF-8")
}

fn output_error(message: &str, detail_code: Option<&str>) -> OutputErrorParams {
    OutputErrorParams {
        code: OutputErrorCode::Runtime,
        detail_code: detail_code.map(str::to_string),
        origin: None,
        message: message.to_string(),
        retryable: None,
        acp: None,
        timestamp: None,
    }
}

#[test]
fn create_output_formatter_selects_requested_format() {
    assert!(matches!(
        create_output_formatter(OutputFormat::Text, Vec::new(), OutputFormatterOptions::default()),
        AnyOutputFormatter::Text(_)
    ));
    assert!(matches!(
        create_output_formatter(OutputFormat::Quiet, Vec::new(), OutputFormatterOptions::default()),
        AnyOutputFormatter::Quiet(_)
    ));
    assert!(matches!(
        create_output_formatter(OutputFormat::Json, Vec::new(), OutputFormatterOptions::default()),
        AnyOutputFormatter::Json(_)
    ));
}

#[test]
fn tool_status_defaults_unknown_values_to_running() {
    assert_eq!(ToolStatus::from_value(Some("completed")).label(), "completed");
    assert_eq!(ToolStatus::from_value(Some("failed")).label(), "failed");
    assert_eq!(ToolStatus::from_value(Some("other")).label(), "running");
}

#[test]
fn truncate_json_preserves_valid_utf8_boundary() {
    let text = "好".repeat(MAX_OUTPUT_LENGTH);
    let rendered = truncate_text(&text);

    assert!(rendered.ends_with("..."));
    assert!(rendered.is_char_boundary(rendered.len()));
    assert!(!truncate_json(&json!({"a": 1})).is_empty());
}

#[test]
fn quiet_formatter_retains_inner_writer() {
    let formatter = create_output_formatter(
        OutputFormat::Quiet,
        Vec::<u8>::new(),
        OutputFormatterOptions {
            context: Some(OutputFormatterContext { session_id: "s1".to_string() }),
            suppress_reads: false,
            is_tty: false,
        },
    );

    assert!(formatter.into_inner().is_empty());
}

#[test]
fn any_output_formatter_delegates_to_inner_variants() {
    let context = OutputFormatterContext { session_id: "s1".to_string() };

    let mut text = create_output_formatter(
        OutputFormat::Text,
        Vec::<u8>::new(),
        OutputFormatterOptions::default(),
    );
    text.set_context(context.clone());
    text.on_error(output_error("text failed", Some("E_TEXT")));
    text.flush();
    let text_output = String::from_utf8(text.into_inner()).expect("text output is UTF-8");
    assert!(text_output.contains("[error] text failed (E_TEXT)"));

    let mut quiet = create_output_formatter(
        OutputFormat::Quiet,
        Vec::<u8>::new(),
        OutputFormatterOptions::default(),
    );
    quiet.set_context(context.clone());
    quiet.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": "quiet text" }
            }
        }
    })));
    quiet.on_error(output_error("ignored", None));
    quiet.flush();
    let quiet_output = String::from_utf8(quiet.into_inner()).expect("quiet output is UTF-8");
    assert_eq!(quiet_output, "quiet text");

    let mut json_formatter = create_output_formatter(
        OutputFormat::Json,
        Vec::<u8>::new(),
        OutputFormatterOptions::default(),
    );
    json_formatter.set_context(context);
    json_formatter.on_error(output_error("json failed", None));
    json_formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": { "stopReason": "end_turn" }
    })));
    json_formatter.flush();
    let json_output = String::from_utf8(json_formatter.into_inner()).expect("json output is UTF-8");
    assert!(json_output.contains("json failed"));
    assert!(json_output.contains("\"stopReason\":\"end_turn\""));
}

#[test]
fn helper_functions_extract_ids_text_locations_and_inputs() {
    assert_eq!(json_rpc_id_key(Some(&json!("abc"))), Some("abc".to_string()));
    assert_eq!(json_rpc_id_key(Some(&json!(42))), Some("42".to_string()));
    assert_eq!(json_rpc_id_key(Some(&json!(true))), None);
    assert_eq!(json_rpc_id_key(None), None);

    assert_eq!(extract_text_content(&json!({ "type": "text", "text": "hello" })), Some("hello"));
    assert_eq!(
        extract_text_content(&json!({ "type": "resource_link", "uri": "file:///tmp/a" })),
        Some("file:///tmp/a")
    );
    assert_eq!(
        extract_text_content(&json!({
            "type": "resource",
            "resource": { "uri": "file:///tmp/b" }
        })),
        Some("file:///tmp/b")
    );
    assert_eq!(extract_text_content(&json!({ "type": "image" })), None);
    assert_eq!(extract_text_content(&json!("plain")), None);

    let locations = format_locations(&[
        json!({ "path": "src/lib.rs", "line": 7 }),
        json!({ "path": "skip.rs" }),
        json!({ "line": 9 }),
    ]);
    assert_eq!(locations, "src/lib.rs:7");

    assert_eq!(summarize_tool_input(None), None);
    assert_eq!(summarize_tool_input(Some(&json!("raw"))), Some("input: \"raw\"".to_string()));
    assert_eq!(
        summarize_tool_input(Some(&json!({
            "command": "cargo check",
            "pattern": "needle",
            "query": ["a"],
            "prompt": "say hi",
            "path": "/tmp/file",
            "url": "https://example.test"
        }))),
        Some(
            "input: command=cargo check, pattern=needle, query=[\"a\"], prompt=say hi, path=/tmp/file, url=https://example.test"
                .to_string()
        )
    );
    assert_eq!(summarize_tool_input(Some(&json!({}))), Some("input: {}".to_string()));
}

#[test]
fn tool_output_helpers_cover_raw_content_suppression_and_truncation() {
    let read_output = json!("file body");
    assert_eq!(
        render_tool_output(
            true,
            ToolDescriptorView { title: Some("Read file"), kind: None },
            Some(&read_output),
            &[],
        ),
        Some(SUPPRESSED_READ_OUTPUT.to_string())
    );

    let raw_output = json!({
        "model_result": "",
        "output": null,
        "content": [],
        "text": "",
        "stdout": "stdout text",
        "stderr": "stderr text",
        "message": "message text",
        "data": "data text"
    });
    assert_eq!(extract_output_text(&raw_output), Some("stdout text".to_string()));
    assert_eq!(
        render_tool_output(
            false,
            ToolDescriptorView { title: Some("Shell"), kind: Some("execute") },
            Some(&raw_output),
            &[],
        ),
        Some("stdout text".to_string())
    );

    assert_eq!(extract_output_text(&json!(null)), None);
    assert_eq!(extract_output_text(&json!(true)), Some("true".to_string()));
    assert_eq!(extract_output_text(&json!(12)), Some("12".to_string()));
    assert_eq!(extract_output_text(&json!([null, "array text"])), Some("array text".to_string()));
    assert_eq!(
        extract_output_text(&json!({ "other": "value" })),
        Some("{\"other\":\"value\"}".to_string())
    );

    let content_items = vec![
        json!({ "type": "unknown", "output": "ignored" }),
        json!({ "type": "content", "content": { "type": "text", "text": "  " } }),
        json!({ "type": "diff", "diff": "" }),
        json!({ "type": "terminal", "output": "terminal output" }),
    ];
    assert_eq!(summarize_tool_content(&content_items), Some("terminal output".to_string()));

    let diff_items = vec![json!({ "type": "diff", "diff": "+added" })];
    assert_eq!(summarize_tool_content(&diff_items), Some("+added".to_string()));

    let text_items =
        vec![json!({ "type": "content", "content": { "type": "text", "text": "content text" } })];
    assert_eq!(
        render_tool_output(
            false,
            ToolDescriptorView { title: None, kind: None },
            None,
            &text_items,
        ),
        Some("content text".to_string())
    );

    let empty_output = json!("   ");
    assert_eq!(
        render_tool_output(
            false,
            ToolDescriptorView { title: None, kind: None },
            Some(&empty_output),
            &[],
        ),
        None
    );
    assert_eq!(summarize_tool_content(&[]), None);

    let long = "a".repeat(MAX_OUTPUT_LENGTH + 1);
    assert_eq!(truncate_text(&long).len(), MAX_OUTPUT_LENGTH + 3);
}

#[test]
fn text_formatter_renders_session_updates_and_flushes_buffers() {
    let mut formatter =
        TextOutputFormatter::new(Vec::<u8>::new(), OutputFormatterOptions::default());

    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "agent_thought_chunk",
                "content": { "type": "text", "text": "thinking" }
            }
        }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "resource_link", "uri": "file:///tmp/result" }
            }
        }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "plan",
                "entries": [
                    { "status": "completed", "content": "read output.rs" },
                    { "content": "add tests" },
                    { "status": "pending", "content": "" }
                ]
            }
        }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "id": 9,
        "result": { "stopReason": "end_turn" }
    })));
    formatter.flush();

    let output = output_text(formatter);
    assert!(output.contains("[thought]\nthinking"));
    assert!(output.contains("file:///tmp/result\n[plan] plan updated"));
    assert!(output.contains("- [completed] read output.rs"));
    assert!(output.contains("- [pending] add tests"));
    assert!(output.contains("[done] end_turn"));
}

#[test]
fn text_formatter_renders_tools_errors_and_client_operations() {
    let mut formatter = TextOutputFormatter::new(
        Vec::<u8>::new(),
        OutputFormatterOptions { context: None, suppress_reads: false, is_tty: true },
    );

    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": { "update": { "sessionUpdate": "tool_call_update" } }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "tool_call",
                "toolCallId": "tool-1",
                "title": "Shell",
                "kind": "execute",
                "status": "running",
                "locations": [
                    { "path": "src/main.rs", "line": 3 },
                    { "path": "missing-line.rs" }
                ],
                "rawInput": { "command": "echo hi" }
            }
        }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "tool_call_update",
                "toolCallId": "tool-1",
                "status": "completed",
                "rawOutput": { "stdout": " done " }
            }
        }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "tool_call",
                "toolCallId": "tool-2",
                "title": "Compile",
                "status": "failed",
                "content": [{ "type": "diff", "diff": "-old\n+new" }]
            }
        }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "id": "init-1"
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "id": "init-2"
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/load",
        "id": "load-1"
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "id": "load-1",
        "error": { "code": -32000, "message": "load failed" }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/new",
        "id": "new-hidden"
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/prompt",
        "id": "prompt-1"
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "id": "err-1",
        "error": { "code": -32000, "message": "visible failure" }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "id": "err-2",
        "error": { "code": -32603, "message": "Internal error: hidden" }
    })));
    formatter.on_error(output_error("explicit failure", Some("E_EXPLICIT")));
    formatter.on_error(output_error("plain failure", None));
    formatter.flush();

    let output = output_text(formatter);
    assert!(output.contains("\u{1b}[1m[tool]\u{1b}[0m Shell (\u{1b}[33mrunning\u{1b}[0m)"));
    assert!(output.contains("input: command=echo hi"));
    assert!(output.contains("locations: src/main.rs:3"));
    assert!(output.contains("done"));
    assert!(output.contains("\u{1b}[1m[tool]\u{1b}[0m Compile (\u{1b}[31mfailed\u{1b}[0m)"));
    assert!(output.contains("-old\n+new"));
    assert_eq!(output.matches("[client]").count(), 2);
    assert!(output.contains("initialize"));
    assert!(output.contains("session/load"));
    assert!(!output.contains("load failed"));
    assert!(output.contains("[error]\u{1b}[0m visible failure"));
    assert!(!output.contains("Internal error: hidden"));
    assert!(output.contains("explicit failure (E_EXPLICIT)"));
    assert!(output.contains("plain failure"));
}

#[test]
fn text_formatter_ignores_malformed_session_updates() {
    let mut formatter =
        TextOutputFormatter::new(Vec::<u8>::new(), OutputFormatterOptions::default());

    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": { "update": { "sessionUpdate": "plan" } }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "image", "text": "ignored" }
            }
        }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": { "update": { "sessionUpdate": "unknown" } }
    })));
    formatter.flush();

    assert!(output_text(formatter).is_empty());
}

#[test]
fn quiet_formatter_writes_only_message_and_string_thought_chunks() {
    let mut formatter = QuietOutputFormatter::new(Vec::<u8>::new(), None);
    formatter.set_context(OutputFormatterContext { session_id: "quiet-session".to_string() });
    formatter.on_error(output_error("ignored", None));

    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": "answer" }
            }
        }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "agent_thought_chunk",
                "content": " plus thought"
            }
        }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "agent_thought_chunk",
                "content": { "type": "text", "text": "ignored object thought" }
            }
        }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": { "stopReason": "end_turn" }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "result": { "stopReason": "end_turn" }
    })));
    formatter.flush();

    assert_eq!(quiet_output_text(formatter), "answer plus thought\n");
}

#[test]
fn text_formatter_merges_tool_updates_and_clears_completed_state() {
    let mut formatter = TextOutputFormatter::new(
        Vec::<u8>::new(),
        OutputFormatterOptions { context: None, suppress_reads: true, is_tty: false },
    );

    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "tool_call",
                "toolCallId": "tool-read",
                "title": "Read file",
                "kind": "read",
                "status": "running",
                "rawInput": true
            }
        }
    })));
    assert!(formatter.tool_states.contains_key("tool-read"));

    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "tool_call_update",
                "toolCallId": "tool-read",
                "status": "completed",
                "locations": [{ "path": "src/output.rs", "line": 12 }],
                "rawOutput": "secret file contents"
            }
        }
    })));

    assert!(!formatter.tool_states.contains_key("tool-read"));

    let output = output_text(formatter);
    assert!(output.contains("[tool] Read file (running)"));
    assert!(output.contains("input: true"));
    assert!(output.contains("[tool] Read file (completed)"));
    assert!(output.contains("locations: src/output.rs:12"));
    assert!(output.contains(SUPPRESSED_READ_OUTPUT));
    assert!(!output.contains("secret file contents"));
}

#[test]
fn text_formatter_resets_duplicate_client_operations_after_normal_traffic() {
    let mut formatter =
        TextOutputFormatter::new(Vec::<u8>::new(), OutputFormatterOptions::default());

    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "id": "init-1"
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "id": "init-2"
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/cancel",
        "id": "cancel-1"
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "id": "init-3"
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/prompt",
        "id": "prompt-1"
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "id": "init-4"
    })));
    formatter.flush();

    let output = output_text(formatter);
    assert_eq!(output.matches("[client]").count(), 3);
    assert_eq!(output.matches("initialize (running)").count(), 3);
}

#[test]
fn quiet_formatter_clears_buffer_without_adding_extra_newline() {
    let mut formatter = QuietOutputFormatter::new(Vec::<u8>::new(), None);

    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": "line one\n" }
            }
        }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": { "stopReason": "end_turn" }
    })));

    assert!(formatter.output.is_empty());

    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "resource", "resource": { "uri": "file:///tmp/next" } }
            }
        }
    })));
    formatter.on_acp_message(acp_message(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "result": { "stopReason": "end_turn" }
    })));

    let output = quiet_output_text(formatter);
    assert_eq!(output, "line one\nfile:///tmp/next\n");
}

#[test]
fn session_update_returns_none_for_non_object_shapes() {
    assert_eq!(session_update(&json!({ "params": [] })), None);
    assert_eq!(session_update(&json!({ "params": { "update": [] } })), None);
}

//! 后端输出解析测试，覆盖 OpenCode 与 Claude 的成功消息和终端错误形状。
//! 解析器只采纳合法文本候选，并按既定优先级从最后一条有效输出提取错误。

use super::*;
use std::sync::mpsc;

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("backend_output_tests"));
}

#[test]
fn extract_opencode_message_reads_first_non_empty_candidate() {
    assert_eq!(
        extract_opencode_message(r#"{"message":{"content":"  hello  "}}"#),
        Some("hello".to_string())
    );
    assert_eq!(extract_opencode_message(r#"{"message":"direct"}"#), Some("direct".to_string()));
    assert_eq!(extract_opencode_message(r#"{"content":"content"}"#), Some("content".to_string()));
    assert_eq!(extract_opencode_message(r#"{"text":"text"}"#), Some("text".to_string()));
    assert_eq!(extract_opencode_message(r#"{"delta":"delta"}"#), Some("delta".to_string()));
    assert_eq!(
        extract_opencode_message(r#"{"result":{"content":"result"}}"#),
        Some("result".to_string())
    );
    assert_eq!(extract_opencode_message(r#"{"output":"output"}"#), Some("output".to_string()));
    assert_eq!(extract_opencode_message(r#"{"text":"   "}"#), None);
    assert_eq!(extract_opencode_message("not-json"), None);
}

#[test]
fn extract_opencode_message_skips_empty_and_non_string_candidates() {
    assert_eq!(
        extract_opencode_message(
            r#"{"message":{"content":"  "},"content":42,"text":" fallback "}"#
        ),
        Some("fallback".to_string())
    );
}

#[test]
fn extract_opencode_error_message_formats_name_and_message_variants() {
    assert_eq!(
        extract_opencode_error_message(
            r#"{"type":"error","error":{"name":"Bad","data":{"message":"failed"}}}"#
        ),
        Some("Bad: failed".to_string())
    );
    assert_eq!(
        extract_opencode_error_message(r#"{"type":"error","error":{"name":"Bad"}}"#),
        Some("Bad".to_string())
    );
    assert_eq!(
        extract_opencode_error_message(r#"{"type":"error","message":"failed"}"#),
        Some("failed".to_string())
    );
    assert_eq!(
        extract_opencode_error_message(
            r#"{"type":"error","error":{"data":{"error":{"message":"deep"}}}}"#
        ),
        Some("deep".to_string())
    );
    assert_eq!(
        extract_opencode_error_message(r#"{"type":"error","error":{}}"#),
        Some("未知错误".to_string())
    );
    assert_eq!(extract_opencode_error_message(r#"{"type":"info","message":"ok"}"#), None);
    assert_eq!(extract_opencode_error_message("not-json"), None);
}

#[test]
fn extract_opencode_error_message_trims_and_skips_empty_fields() {
    assert_eq!(
        extract_opencode_error_message(
            r#"{"type":"error","error":{"name":"  Bad  ","data":{"message":"  "},"message":" detail "}}"#
        ),
        Some("Bad: detail".to_string())
    );
    assert_eq!(
        extract_opencode_error_message(
            r#"{"type":"error","error":{"name":"  "},"message":" detail "}"#
        ),
        Some("detail".to_string())
    );
}

#[test]
fn extract_opencode_terminal_error_prefers_stdout_last_error_then_stderr() {
    let stdout = "noise\n{\"type\":\"error\",\"message\":\"stdout error\"}\n";
    let stderr = "{\"type\":\"error\",\"message\":\"stderr error\"}\n";
    assert_eq!(extract_opencode_terminal_error(stdout, stderr), Some("stdout error".to_string()));

    assert_eq!(
        extract_opencode_terminal_error("noise\n", stderr),
        Some("stderr error".to_string())
    );
    assert_eq!(extract_opencode_terminal_error("noise\n", "plain\n"), None);
}

#[test]
fn extract_opencode_terminal_error_only_interprets_last_non_empty_line_per_stream() {
    let stdout = "{\"type\":\"error\",\"message\":\"old stdout\"}\nlatest stdout\n\n";
    let stderr = "{\"type\":\"error\",\"message\":\"old stderr\"}\nlatest stderr\n";

    assert_eq!(extract_opencode_terminal_error(stdout, stderr), None);
}

#[test]
fn extract_claude_message_reads_assistant_blocks_and_fallback_fields() {
    assert_eq!(
        extract_claude_message(
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"one"},{"type":"tool_use","text":"skip"},{"thinking":"two"}]}}"#
        ),
        Some("one\ntwo".to_string())
    );
    assert_eq!(
        extract_claude_message(
            r#"{"content":[{"delta":{"text":"delta"}},{"content":{"text":"deep"}}]}"#
        ),
        Some("delta\ndeep".to_string())
    );
    assert_eq!(
        extract_claude_message(r#"{"delta":{"text":"delta text"}}"#),
        Some("delta text".to_string())
    );
    assert_eq!(
        extract_claude_message(r#"{"content_block":{"text":"block text"}}"#),
        Some("block text".to_string())
    );
    assert_eq!(
        extract_claude_message(r#"{"message":{"text":"message text"}}"#),
        Some("message text".to_string())
    );
    assert_eq!(
        extract_claude_message(r#"{"result":"result text"}"#),
        Some("result text".to_string())
    );
    assert_eq!(extract_claude_message(r#"{"message":"direct"}"#), Some("direct".to_string()));
    assert_eq!(extract_claude_message(r#"{"content":"content"}"#), Some("content".to_string()));
    assert_eq!(extract_claude_message(r#"{"text":"text"}"#), Some("text".to_string()));
    assert_eq!(extract_claude_message(r#"{"delta":"delta"}"#), Some("delta".to_string()));
    assert_eq!(extract_claude_message(r#"{"text":"   "}"#), None);
    assert_eq!(extract_claude_message("not-json"), None);
}

#[test]
fn extract_claude_message_rejects_tool_blocks_and_skips_empty_candidates() {
    assert_eq!(
        extract_claude_message(
            r#"{"type":"assistant","message":{"content":[{"type":"tool_result","content":"tool result"},{"type":"text","text":"  "}]},"text":" fallback "}"#
        ),
        Some("fallback".to_string())
    );
    assert_eq!(
        extract_claude_message(
            r#"{"content":[{"type":"tool_use","text":"tool input"},{"type":"tool_result","thinking":"tool output"}]}"#
        ),
        None
    );
}

#[test]
fn extract_claude_message_reads_remaining_text_block_shapes() {
    assert_eq!(
        extract_claude_message(
            r#"{"content":[{"type":"thinking","thinking":" thought "},{"content":" plain content "}]}"#
        ),
        Some("thought\nplain content".to_string())
    );
}

#[test]
fn extract_claude_error_message_handles_error_shapes() {
    assert_eq!(
        extract_claude_error_message(r#"{"type":"error","error":{"message":"bad"}}"#),
        Some("bad".to_string())
    );
    assert_eq!(
        extract_claude_error_message(r#"{"is_error":true,"error":"bad string"}"#),
        Some("bad string".to_string())
    );
    assert_eq!(
        extract_claude_error_message(r#"{"is_error":true,"result":"result error"}"#),
        Some("result error".to_string())
    );
    assert_eq!(
        extract_claude_error_message(r#"{"is_error":true,"message":"message error"}"#),
        Some("message error".to_string())
    );
    assert_eq!(
        extract_claude_error_message(r#"{"is_error":true,"text":"text error"}"#),
        Some("text error".to_string())
    );
    assert_eq!(
        extract_claude_error_message(r#"{"type":"error","error":{}}"#),
        Some("未知错误".to_string())
    );
    assert_eq!(extract_claude_error_message(r#"{"type":"assistant","text":"ok"}"#), None);
    assert_eq!(extract_claude_error_message("not-json"), None);
}

#[test]
fn extract_claude_error_message_skips_empty_fields_before_fallback() {
    assert_eq!(
        extract_claude_error_message(
            r#"{"is_error":true,"error":{"message":"  "},"result":" result error "}"#
        ),
        Some("result error".to_string())
    );
    assert_eq!(
        extract_claude_error_message(r#"{"is_error":true,"result":"  "}"#),
        Some("未知错误".to_string())
    );
}

#[test]
fn extract_claude_terminal_error_prefers_stdout_last_error_then_stderr() {
    let stdout = "noise\n{\"type\":\"error\",\"message\":\"stdout error\"}\n";
    let stderr = "{\"type\":\"error\",\"message\":\"stderr error\"}\n";
    assert_eq!(extract_claude_terminal_error(stdout, stderr), Some("stdout error".to_string()));
    assert_eq!(extract_claude_terminal_error("noise\n", stderr), Some("stderr error".to_string()));
    assert_eq!(extract_claude_terminal_error("noise\n", "plain\n"), None);
}

#[test]
fn extract_claude_terminal_error_uses_stderr_when_stdout_last_line_is_not_error() {
    let stdout = "{\"type\":\"error\",\"message\":\"old stdout\"}\nlatest stdout\n";
    let stderr = "\n{\"type\":\"error\",\"message\":\"stderr error\"}\n\n";

    assert_eq!(extract_claude_terminal_error(stdout, stderr), Some("stderr error".to_string()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_model_prompt_prefixes_task_id_and_trims_prompt_start() {
    let task = Task { id: "task-1".to_string(), ..Task::default() };

    let prompt = build_model_prompt(&task, "\n\n  do work");

    assert!(prompt.starts_with("task_id=task-1\n"));
    assert!(prompt.contains("无法进行交互式应答"));
    assert!(prompt.contains("node_repl"));
    assert!(prompt.ends_with("do work"));
}

#[test]
fn emit_opencode_line_sends_parsed_or_raw_streams_and_ignores_empty() {
    let (tx, rx) = mpsc::channel();

    emit_opencode_line(&tx, "  ", false);
    emit_opencode_line(&tx, r#"{"text":"hello"}"#, false);
    emit_opencode_line(&tx, "raw out", false);
    emit_opencode_line(&tx, "raw err", true);

    match rx.recv().unwrap() {
        TaskLogStream::Stdout(value) => assert_eq!(value, "[OPENCODE] hello"),
        other => panic!("unexpected stream: {other:?}"),
    }
    match rx.recv().unwrap() {
        TaskLogStream::Stdout(value) => assert_eq!(value, "[OPENCODE_RAW] raw out"),
        other => panic!("unexpected stream: {other:?}"),
    }
    match rx.recv().unwrap() {
        TaskLogStream::Stderr(value) => assert_eq!(value, "[OPENCODE_RAW] raw err"),
        other => panic!("unexpected stream: {other:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn emit_claude_line_sends_parsed_or_raw_streams_and_ignores_empty() {
    let (tx, rx) = mpsc::channel();

    emit_claude_line(&tx, "  ", false);
    emit_claude_line(&tx, r#"{"text":"hello"}"#, false);
    emit_claude_line(&tx, "raw out", false);
    emit_claude_line(&tx, "raw err", true);

    match rx.recv().unwrap() {
        TaskLogStream::Stdout(value) => assert_eq!(value, "[CLAUDE] hello"),
        other => panic!("unexpected stream: {other:?}"),
    }
    match rx.recv().unwrap() {
        TaskLogStream::Stdout(value) => assert_eq!(value, "[CLAUDE_RAW] raw out"),
        other => panic!("unexpected stream: {other:?}"),
    }
    match rx.recv().unwrap() {
        TaskLogStream::Stderr(value) => assert_eq!(value, "[CLAUDE_RAW] raw err"),
        other => panic!("unexpected stream: {other:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn emit_lines_ignore_disconnected_receivers() {
    let (tx, rx) = mpsc::channel();
    drop(rx);

    emit_opencode_line(&tx, r#"{"text":"hello"}"#, false);
    emit_opencode_line(&tx, "raw error", true);
    emit_claude_line(&tx, r#"{"text":"hello"}"#, false);
    emit_claude_line(&tx, "raw error", true);
}

//! 工具事件、UI 载荷与历史去重的库内契约。

use crate::processor_events::StreamEvent;
use crate::runner_types::ToolSessionState;
use crate::session_memory::{Role, Session};
use session_port::{ToolCallStatus, ToolEventKind};
use tool_assembly::ToolCallError;
use tool_types::{ToolCallResult, ToolCallTelemetry, ToolRenderHint};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);

fn unique_session(prefix: &str) -> String {
    format!("{prefix}-{}-{}", std::process::id(), NEXT_SESSION.fetch_add(1, Ordering::Relaxed))
}

fn new_state() -> ToolSessionState {
    ToolSessionState::default()
}

fn computed_success(tool: &str, output: Option<&str>) -> super::BatchComputed {
    super::BatchComputed {
        call_id: format!("call-{tool}"),
        tool: tool.to_string(),
        input_sanitized: "{}".to_string(),
        ui_payload: json!({}),
        session_message: format!("tool {tool}\n{{}}\n"),
        is_non_todo: true,
        success: true,
        output_for_model: output.map(str::to_string),
        error_for_model: None,
    }
}

fn delta_texts(events: &[StreamEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::Delta(text) => Some(text.clone()),
            StreamEvent::Tool(event) => Some(format!("tool {}\n{}\n", event.name, event.payload)),
            _ => None,
        })
        .collect()
}

#[test]
fn typed_tool_event_keeps_call_id_status_and_sanitized_payload() {
    let event = super::tool_stream_event(
        "call-1".to_string(),
        "shell",
        super::running_tool_payload("{\"token\":\"[REDACTED]\"}", "shell"),
    );

    let StreamEvent::Tool(event) = event else {
        panic!("expected typed tool event");
    };
    assert_eq!(event.call_id, "call-1");
    assert_eq!(event.kind, ToolEventKind::Start);
    assert_eq!(event.status, ToolCallStatus::Running);
    assert_eq!(event.payload["input"], "{\"token\":\"[REDACTED]\"}");
}

#[test]
fn tool_stream_event_has_occurrence_time() {
    let start = super::tool_stream_event(
        "call-1".to_string(),
        "shell",
        super::running_tool_payload("{}", "shell"),
    );
    let complete = super::tool_update_event(
        "call-1".to_string(),
        "shell",
        serde_json::json!({"status": "completed", "output": "done"}),
    );

    let (StreamEvent::Tool(start), StreamEvent::Tool(complete)) = (start, complete) else {
        panic!("expected typed tool events");
    };
    assert_eq!(start.call_id, complete.call_id);
    assert!(start.occurred_ms > 0);
    assert!(start.occurred_ms <= complete.occurred_ms);
}

#[test]
// 只有编辑类工具保留完整输出，其他工具使用受控摘要降低泄露和膨胀。
fn preserve_full_output_only_for_editing_tools() {
    for name in ["apply_patch", "write", "file_write", "file_edit"] {
        assert!(super::preserve_full_output_in_session(name), "{name}");
    }
    for name in ["read", "file_read", "bash", "todowrite"] {
        assert!(!super::preserve_full_output_in_session(name), "{name}");
    }
}

#[test]
fn denied_tool_payload_includes_input_error_and_optional_permission_request() {
    let denied = ToolCallError::denied("blocked by policy");
    let payload = super::denied_tool_payload_with_input(r#"{"path":"secret"}"#, &denied);

    assert_eq!(payload["status"], "denied");
    assert_eq!(payload["input"], r#"{"path":"secret"}"#);
    assert_eq!(payload["error"], "blocked by policy");
    assert!(payload.get("permission_request").is_none());

    let denied = ToolCallError::denied_with_permission_request(
        "needs approval",
        vw_api_tools::PermissionRequestDto {
            reason: "write outside workspace".to_string(),
            warning: Some("careful".to_string()),
            updated_input: Some(json!({ "path": "demo.md" })),
        },
    );
    let payload = super::denied_tool_payload(&denied);

    assert_eq!(payload["status"], "denied");
    assert!(payload.get("input").is_none());
    assert_eq!(payload["permission_request"]["reason"], "write outside workspace");
    assert_eq!(payload["permission_request"]["warning"], "careful");
    assert_eq!(payload["permission_request"]["updated_input"]["path"], "demo.md");
}

#[test]
fn completed_tool_payload_for_ui_uses_render_hint_metadata_and_omits_empty_result() {
    let result = ToolCallResult {
        render_hint: Some(ToolRenderHint {
            title: Some("Friendly title".to_string()),
            kind: Some("demo".to_string()),
            summary: Some("summary".to_string()),
            metadata: json!({ "rows": 3 }),
        }),
        telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
        ..ToolCallResult::default()
    };

    let payload =
        super::completed_tool_payload_for_ui("demo_tool", "{}", &result, "compact output");

    assert_eq!(payload["status"], "completed");
    assert_eq!(payload["input"], "{}");
    assert_eq!(payload["title"], "Friendly title");
    assert_eq!(payload["metadata"]["rows"], 3);
    assert_eq!(payload["output"], "compact output");
    assert!(payload.get("result").is_none());
}

#[test]
fn completed_tool_payload_for_ui_keeps_structured_patch_content() {
    use vw_api_tools::{StructuredPatchHunkDto, ToolResultContentDto};

    let result = ToolCallResult {
        content_blocks: vec![ToolResultContentDto::StructuredPatch {
            hunks: vec![StructuredPatchHunkDto {
                header: "@@ -1 +1 @@".to_string(),
                path: Some("docs/tailwind/align-self.mdx".to_string()),
                old_start: Some(1),
                old_lines: Some(1),
                new_start: Some(1),
                new_lines: Some(1),
                lines: vec!["+full replacement line".to_string()],
            }],
        }],
        telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
        ..ToolCallResult::default()
    };

    let payload = super::completed_tool_payload_for_ui(
        "file_edit",
        r#"{"file_path":"docs/tailwind/align-self.mdx"}"#,
        &result,
        "Updated docs/tailwind/align-self.mdx.",
    );

    let content = payload
        .get("result")
        .and_then(|value| value.get("content"))
        .and_then(Value::as_array)
        .expect("structured content should be exposed to UI");

    assert_eq!(content[0].get("type").and_then(Value::as_str), Some("structured_patch"));
    assert_eq!(
        content[0]
            .get("hunks")
            .and_then(Value::as_array)
            .and_then(|hunks| hunks[0].get("lines"))
            .and_then(Value::as_array)
            .and_then(|lines| lines[0].as_str()),
        Some("+full replacement line")
    );
}

#[test]
fn model_tool_result_preserves_image_marker() {
    let marker = "[IMAGE:data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAAB]";
    let output = format!("Screenshot captured: /tmp/pixel.png (1 bytes, image/png, 1x1)\n{marker}");

    assert_eq!(super::compact_tool_output("screenshot", &output), output);
    assert!(
        super::batch_model_output(1, &[computed_success("screenshot", Some(&output))])
            .contains(marker)
    );
}

#[test]
fn record_computed_tool_emits_deduplicates_and_updates_non_todo_count() {
    let session_id = unique_session("tools-record-computed");
    let mut session = Session::new(session_id);
    let mut state = new_state();
    state.non_todo_tool_runs = u64::MAX;
    let mut events = Vec::new();

    let computed = computed_success("bash", Some("ok"));
    super::record_computed_tool(
        &mut session,
        computed.clone(),
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    );
    super::record_computed_tool(
        &mut session,
        computed,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    );

    assert_eq!(delta_texts(&events).len(), 2);
    assert_eq!(session.messages.len(), 1);
    assert_eq!(state.non_todo_tool_runs, u64::MAX);

    let mut todo = computed_success("todoread", Some("[]"));
    todo.is_non_todo = false;
    super::record_computed_tool(
        &mut session,
        todo,
        false,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    );

    assert_eq!(delta_texts(&events).len(), 2);
    assert_eq!(session.messages.len(), 2);
    assert_eq!(state.non_todo_tool_runs, u64::MAX);
}

#[test]
fn run_batch_error_and_record_emits_error_event_and_deduplicates_session_message() {
    let session_id = unique_session("tools-batch-error");
    let mut session = Session::new(session_id.clone());
    let mut state = new_state();
    let mut events = Vec::new();

    super::run_batch_error_and_record(
        &mut session,
        "{bad",
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
        "bad input".to_string(),
    );
    super::run_batch_error_and_record(
        &mut session,
        "{bad",
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
        "bad input".to_string(),
    );

    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, Role::Tool);
    assert!(session.messages[0].content.contains("\"status\":\"error\""));
    assert_eq!(delta_texts(&events).len(), 2);
    assert!(delta_texts(&events)[0].contains("bad input"));
}

#[test]
fn run_batch_error_and_record_without_ui_still_deduplicates_session_message() {
    let session_id = unique_session("tools-batch-error-no-ui");
    let mut session = Session::new(session_id);
    let mut state = new_state();
    let mut events = Vec::new();

    for _ in 0..2 {
        super::run_batch_error_and_record(
            &mut session,
            "not-json",
            false,
            &mut |event| {
                events.push(event);
                true
            },
            &mut state,
            "invalid input".to_string(),
        );
    }

    assert!(events.is_empty());
    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, Role::Tool);
    assert!(session.messages[0].content.contains("invalid input"));
}

use crate::app::agent::session::processor::StreamEvent;
use crate::app::agent::session::session::{Role, Session};
use crate::app::agent::tools::{ToolCallError, ToolRuntimeContext, ToolUseContext, todo};
use crate::app::agent::tools::{ToolCallResult, ToolCallTelemetry, ToolRenderHint};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);

struct EnabledTraceSubscriber;

impl tracing::Subscriber for EnabledTraceSubscriber {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::Id {
        tracing::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::Id, _follows: &tracing::Id) {}

    fn event(&self, _event: &tracing::Event<'_>) {}

    fn enter(&self, _span: &tracing::Id) {}

    fn exit(&self, _span: &tracing::Id) {}
}

fn unique_session(prefix: &str) -> String {
    format!("{prefix}-{}-{}", std::process::id(), NEXT_SESSION.fetch_add(1, Ordering::Relaxed))
}

fn ctx(session: &str) -> ToolRuntimeContext {
    ToolRuntimeContext::new(session.to_string(), None)
}

fn ctx_with_root(session: &str, root: &Path) -> ToolRuntimeContext {
    let root = root.to_string_lossy().to_string();
    ToolRuntimeContext::new(session.to_string(), Some(root.clone()))
        .with_tool_use_context(ToolUseContext::new(session.to_string(), Some(root)))
}

fn new_state() -> super::super::ToolSessionState {
    super::super::ToolSessionState::default()
}

fn delta_texts(events: &[StreamEvent]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::Delta(text) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn preserve_full_output_only_for_editing_tools() {
    for name in ["apply_patch", "write", "file_write", "file_edit", "notebook_edit"] {
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
        vw_api_types::tools::PermissionRequestDto {
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
    use vw_api_types::tools::{StructuredPatchHunkDto, ToolResultContentDto};

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

fn computed_success(tool: &str, output: Option<&str>) -> super::BatchComputed {
    super::BatchComputed {
        tool: tool.to_string(),
        input_sanitized: "{}".to_string(),
        ui_message: format!("tool {tool}\n{{}}\n"),
        session_message: format!("tool {tool}\n{{}}\n"),
        is_non_todo: true,
        success: true,
        output_for_model: output.map(str::to_string),
        error_for_model: None,
    }
}

fn computed_error(tool: &str, error: Option<&str>) -> super::BatchComputed {
    super::BatchComputed {
        tool: tool.to_string(),
        input_sanitized: "{}".to_string(),
        ui_message: format!("tool {tool}\n{{}}\n"),
        session_message: format!("tool {tool}\n{{}}\n"),
        is_non_todo: true,
        success: false,
        output_for_model: None,
        error_for_model: error.map(str::to_string),
    }
}

#[test]
fn batch_join_error_computed_formats_error_payload_for_ui_session_and_model() {
    let computed = super::batch_join_error_computed("worker panicked");

    assert_eq!(computed.tool, "batch");
    assert!(!computed.success);
    assert!(!computed.is_non_todo);
    assert!(computed.input_sanitized.is_empty());
    assert!(computed.ui_message.contains("\"status\":\"error\""));
    assert!(computed.ui_message.contains("\"input\":\"\""));
    assert!(computed.session_message.contains("\"status\":\"error\""));
    assert_eq!(
        computed.error_for_model.as_deref(),
        Some("Batch subtask join error: worker panicked")
    );
}

#[test]
fn batch_model_output_formats_empty_success_mixed_and_errorless_results() {
    assert_eq!(super::batch_model_output(0, &[]), "未执行任何子任务");

    let success = super::batch_model_output(
        3,
        &[
            computed_success("todoread", Some("  first output  ")),
            computed_success("empty-success", Some("   ")),
            computed_success("metadata-only", None),
        ],
    );
    assert!(success.contains("全部 3 个工具均执行成功"));
    assert!(success.contains("first output"));
    assert!(!success.contains("metadata-only"));

    let mixed = super::batch_model_output(
        4,
        &[
            computed_success("todoread", Some("ok")),
            computed_error("denied", Some("needs approval")),
            computed_error("silent-error", None),
        ],
    );
    assert!(mixed.contains("已成功执行 1/3 个工具，失败 2 个。"));
    assert!(mixed.contains("错误："));
    assert!(mixed.contains("- denied: needs approval"));
    assert!(!mixed.contains("silent-error:"));
}

#[test]
fn batch_ui_output_formats_empty_and_ran_lists() {
    assert_eq!(super::batch_ui_output(0, &[]), "未执行任何子任务");
    assert_eq!(
        super::batch_ui_output(2, &["- read src/lib.rs".to_string(), "- grep TODO".to_string()]),
        "已执行 2 个子任务：\n- read src/lib.rs\n- grep TODO"
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
fn tool_classification_helpers_cover_todo_streaming_and_regular_tools() {
    assert!(super::should_rewrite_todowrite("todowrite", 0));
    assert!(!super::should_rewrite_todowrite("todowrite", 1));
    assert!(!super::should_rewrite_todowrite("todoread", 0));
    assert!(!super::should_rewrite_todowrite("bash", 0));

    assert!(!super::is_non_todo_tool("todowrite"));
    assert!(!super::is_non_todo_tool("todoread"));
    assert!(super::is_non_todo_tool("bash"));
    assert!(super::is_non_todo_tool("write"));

    assert!(super::should_emit_running_tool(true, "file_read"));
    assert!(!super::should_emit_running_tool(false, "file_read"));
    assert!(!super::should_emit_running_tool(true, "todoread"));
}

#[test]
fn describe_batch_call_formats_known_tools_and_falls_back_to_sanitized_json() {
    assert_eq!(
        super::describe_batch_call("bash", &json!({ "command": "echo ok" })),
        "- bash echo ok"
    );
    assert_eq!(
        super::describe_batch_call(
            "read",
            &json!({ "path": "src/lib.rs", "offset": -2, "limit": 10 })
        ),
        "- read src/lib.rs [offset=1, limit=10]"
    );
    assert_eq!(
        super::describe_batch_call("file_read", &json!({ "offset": 5 })),
        "- file_read [offset=5]"
    );
    assert_eq!(
        super::describe_batch_call("file_edit", &json!({ "file_path": "src/main.rs" })),
        "- file_edit src/main.rs"
    );
    assert_eq!(
        super::describe_batch_call("glob", &json!({ "pattern": "**/*.rs" })),
        "- glob **/*.rs"
    );
    assert_eq!(
        super::describe_batch_call("grep", &json!({ "pattern": "TODO", "path": "src" })),
        "- grep pattern=TODO path=src"
    );
    assert_eq!(
        super::describe_batch_call("grep", &json!({ "pattern": "TODO" })),
        "- grep pattern=TODO"
    );
    assert_eq!(super::describe_batch_call("grep", &json!({ "path": "src" })), "- grep path=src");
    assert_eq!(super::describe_batch_call("ls", &json!({ "path": "src" })), "- ls src");
    assert_eq!(super::describe_batch_call("custom", &json!({})), "- custom {}");
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
fn run_tool_and_record_reports_parse_errors_for_streaming_tools_and_deduplicates() {
    let session_id = unique_session("tools-single-error");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id);
    let mut state = new_state();
    let mut events = Vec::new();

    let first = super::run_tool_and_record(
        &mut session,
        "file_read",
        "{bad",
        &ctx,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    )
    .expect("content");
    let second = super::run_tool_and_record(
        &mut session,
        "file_read",
        "{bad",
        &ctx,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    )
    .expect("content");

    assert!(first.contains("invalid JSON arguments"));
    assert_eq!(first, second);
    assert_eq!(session.messages.len(), 1);
    assert!(session.messages[0].content.contains("\"status\":\"error\""));
    assert_eq!(state.non_todo_tool_runs, 2);
    assert!(delta_texts(&events).iter().any(|text| text.contains("\"status\":\"running\"")));
    assert!(delta_texts(&events).iter().any(|text| text.contains("invalid JSON arguments")));
}

#[test]
fn run_tool_and_record_delegates_batch_tool() {
    let session_id = unique_session("tools-batch-delegate");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id);
    let mut state = new_state();
    let mut events = Vec::new();

    let output = super::run_tool_and_record(
        &mut session,
        "batch",
        "not-json",
        &ctx,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    )
    .expect("batch content");

    assert_eq!(output, "Invalid input format for batch");
    assert_eq!(session.messages.len(), 1);
    assert!(session.messages[0].content.contains("tool batch"));
    assert!(delta_texts(&events).iter().any(|text| text.contains("\"status\":\"error\"")));
}

#[test]
fn run_tool_and_record_rewrites_initial_todowrite_completed_status_until_non_todo_work_exists() {
    let session_id = unique_session("tools-todowrite-rewrite");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id);
    let mut state = new_state();
    let input = json!({
        "todos": [
            { "id": "1", "content": "finish", "status": "completed", "priority": "high" }
        ]
    })
    .to_string();

    super::run_tool_and_record(
        &mut session,
        "todowrite",
        &input,
        &ctx,
        false,
        &mut |_event| true,
        &mut state,
    )
    .expect("todowrite output");

    let todos: Vec<todo::Todo> =
        serde_json::from_str(&todo::read(&ctx).expect("read todos")).expect("todos json");
    assert_eq!(todos[0].status, "pending");
    assert_eq!(state.non_todo_tool_runs, 0);

    state.non_todo_tool_runs = 1;
    super::run_tool_and_record(
        &mut session,
        "todowrite",
        &input,
        &ctx,
        false,
        &mut |_event| true,
        &mut state,
    )
    .expect("todowrite output");

    let todos: Vec<todo::Todo> =
        serde_json::from_str(&todo::read(&ctx).expect("read todos")).expect("todos json");
    assert_eq!(todos[0].status, "completed");
}

#[test]
fn run_batch_tool_and_record_handles_invalid_json_recursive_and_empty_batches() {
    let session_id = unique_session("tools-batch-invalid");
    let ctx = ctx(&session_id);

    let mut invalid_session = Session::new(format!("{session_id}-invalid"));
    let mut invalid_state = new_state();
    let invalid = super::run_batch_tool_and_record(
        &mut invalid_session,
        "{bad",
        &ctx,
        false,
        &mut |_event| true,
        &mut invalid_state,
    );
    assert!(invalid.contains("invalid arguments"));
    assert_eq!(invalid_session.messages.len(), 1);

    let mut recursive_session = Session::new(format!("{session_id}-recursive"));
    let mut recursive_state = new_state();
    let recursive = super::run_batch_tool_and_record(
        &mut recursive_session,
        &json!({ "tool_calls": [{ "tool": "batch", "parameters": {} }] }).to_string(),
        &ctx,
        false,
        &mut |_event| true,
        &mut recursive_state,
    );
    assert_eq!(recursive, "Recursive batch calls are not allowed");
    assert_eq!(recursive_session.messages.len(), 1);

    let mut empty_session = Session::new(format!("{session_id}-empty"));
    let mut empty_state = new_state();
    let empty = super::run_batch_tool_and_record(
        &mut empty_session,
        &json!({ "tool_calls": [{ "parameters": {} }, { "tool": null }] }).to_string(),
        &ctx,
        false,
        &mut |_event| true,
        &mut empty_state,
    );
    assert_eq!(empty, "未执行任何子任务");
    assert_eq!(empty_session.messages.len(), 1);
    assert!(empty_session.messages[0].content.contains("未执行任何子任务"));
}

#[test]
fn run_batch_tool_and_record_records_failed_subtool_and_summary() {
    let session_id = unique_session("tools-batch-failed-subtool");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id);
    let mut state = new_state();
    let mut events = Vec::new();

    let output = super::run_batch_tool_and_record(
        &mut session,
        &json!({ "tool_calls": [{ "tool": "unknown_tool", "parameters": {} }] }).to_string(),
        &ctx,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    );

    assert!(output.contains("已成功执行 0/1 个工具"));
    assert!(output.contains("Unknown tool: unknown_tool"));
    assert_eq!(session.messages.len(), 2);
    assert!(session.messages.iter().any(|message| message.content.contains("tool unknown_tool")));
    assert!(session.messages.iter().any(|message| message.content.contains("tool batch")));
    assert_eq!(state.non_todo_tool_runs, 1);
    assert!(delta_texts(&events).iter().any(|text| text.contains("Unknown tool")));
    assert!(delta_texts(&events).iter().any(|text| text.contains("已执行 1 个子任务")));
}

#[test]
fn run_batch_tool_and_record_summarizes_single_and_parallel_successful_batches() {
    let session_id = unique_session("tools-batch-success");
    let ctx = ctx(&session_id);

    let mut single_session = Session::new(format!("{session_id}-single"));
    let mut single_state = new_state();
    let single = super::run_batch_tool_and_record(
        &mut single_session,
        &json!({ "tool_calls": [{ "tool": "todoread", "parameters": {} }] }).to_string(),
        &ctx,
        false,
        &mut |_event| true,
        &mut single_state,
    );
    assert!(single.contains("全部 1 个工具均执行成功"));
    assert_eq!(single_session.messages.len(), 2);
    assert_eq!(single_state.non_todo_tool_runs, 0);

    let mut parallel_session = Session::new(format!("{session_id}-parallel"));
    let mut parallel_state = new_state();
    let parallel = super::run_batch_tool_and_record(
        &mut parallel_session,
        &json!({
            "calls": [
                { "tool": "todoread", "parameters": {} },
                { "tool": "todoread", "parameters": {} }
            ]
        })
        .to_string(),
        &ctx,
        false,
        &mut |_event| true,
        &mut parallel_state,
    );
    assert!(parallel.contains("全部 2 个工具均执行成功"));
    assert!(parallel_session.messages.iter().any(|message| message.content.contains("tool batch")));
    assert_eq!(parallel_state.non_todo_tool_runs, 0);
}

#[test]
fn describe_batch_call_handles_aliases_trimmed_values_and_missing_paths() {
    assert_eq!(
        super::describe_batch_call("shell", &json!({ "command": "  printf ok  " })),
        "- shell printf ok"
    );
    assert_eq!(
        super::describe_batch_call("read", &json!({ "filePath": "src/lib.rs" })),
        "- read src/lib.rs"
    );
    assert_eq!(
        super::describe_batch_call("read", &json!({ "file_path": "src/alias.rs" })),
        "- read src/alias.rs"
    );
    assert_eq!(
        super::describe_batch_call("file_read", &json!({ "offset": 0, "limit": -5 })),
        "- file_read [offset=1, limit=-5]"
    );
    assert_eq!(
        super::describe_batch_call("write", &json!({ "filePath": "camel.md" })),
        "- write camel.md"
    );
    assert_eq!(
        super::describe_batch_call("file_write", &json!({ "file_path": "snake.md" })),
        "- file_write snake.md"
    );
    for tool in ["write", "file_write", "notebook_edit", "apply_patch"] {
        assert_eq!(
            super::describe_batch_call(tool, &json!({ "path": "notes.md" })),
            format!("- {tool} notes.md")
        );
    }
    assert_eq!(super::describe_batch_call("custom", &Value::Null), "- custom null");
}

#[test]
fn describe_batch_call_covers_empty_string_values_and_unmatched_grep_details() {
    assert_eq!(
        super::describe_batch_call("shell", &json!({ "command": "   " })),
        "- shell {\"command\":\"   \"}"
    );
    assert_eq!(super::describe_batch_call("grep", &json!({})), "- grep {}");
    assert_eq!(super::describe_batch_call("ls", &json!({ "path": "" })), "- ls {\"path\":\"\"}");
    assert_eq!(
        super::describe_batch_call("glob", &json!({ "pattern": "   " })),
        "- glob {\"pattern\":\"   \"}"
    );
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

#[test]
fn run_tool_and_record_records_successful_todoread_and_deduplicates_repeated_calls() {
    let session_id = unique_session("tools-single-success");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id);
    let mut state = new_state();
    let mut events = Vec::new();

    let first = super::run_tool_and_record(
        &mut session,
        "todoread",
        "{}",
        &ctx,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    )
    .expect("todoread output");
    let second = super::run_tool_and_record(
        &mut session,
        "todoread",
        "{}",
        &ctx,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    )
    .expect("todoread output");

    assert_eq!(first, second);
    assert_eq!(session.messages.len(), 1);
    assert!(session.messages[0].content.contains("\"status\":\"completed\""));
    assert_eq!(state.non_todo_tool_runs, 0);
    assert_eq!(delta_texts(&events).len(), 2);
    assert!(delta_texts(&events).iter().all(|text| text.contains("\"status\":\"completed\"")));
}

#[test]
fn run_tool_and_record_evaluates_trace_fields_when_subscriber_is_enabled() {
    let session_id = unique_session("tools-single-tracing");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id);
    let mut state = new_state();
    let dispatch = tracing::Dispatch::new(EnabledTraceSubscriber);

    tracing::dispatcher::with_default(&dispatch, || {
        super::run_tool_and_record(
            &mut session,
            "todoread",
            "{}",
            &ctx,
            false,
            &mut |_event| true,
            &mut state,
        )
        .expect("todoread output");
    });

    assert_eq!(session.messages.len(), 1);
    assert_eq!(state.non_todo_tool_runs, 0);
}

#[test]
fn run_tool_and_record_saturates_non_todo_counter_without_emitting_ui() {
    let session_id = unique_session("tools-single-saturating-count");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id);
    let mut state = new_state();
    state.non_todo_tool_runs = u64::MAX;
    let mut events = Vec::new();

    let output = super::run_tool_and_record(
        &mut session,
        "unknown_tool",
        "{}",
        &ctx,
        false,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    )
    .expect("unknown tool error");

    assert!(output.contains("Unknown tool: unknown_tool"));
    assert!(events.is_empty());
    assert_eq!(session.messages.len(), 1);
    assert_eq!(state.non_todo_tool_runs, u64::MAX);
}

#[test]
fn run_tool_and_record_records_permission_denial_with_request_context() {
    let dir = tempfile::tempdir().expect("temp dir");
    let session_id = unique_session("tools-single-denied");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx_with_root(&session_id, dir.path());
    let mut state = new_state();
    let mut events = Vec::new();

    let output = super::run_tool_and_record(
        &mut session,
        "bash",
        &json!({ "command": "touch needs-approval" }).to_string(),
        &ctx,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    )
    .expect("denied output");

    assert!(output.contains("Approval required"));
    assert!(!dir.path().join("needs-approval").exists());
    assert_eq!(session.messages.len(), 1);
    assert!(session.messages[0].content.contains("\"status\":\"denied\""));
    assert!(session.messages[0].content.contains("\"permission_request\""));
    assert!(!session.messages[0].content.contains("\"input\""));
    assert_eq!(state.non_todo_tool_runs, 1);
    assert!(
        delta_texts(&events)
            .iter()
            .any(|text| text.contains("\"status\":\"denied\"") && text.contains("\"input\""))
    );
}

#[test]
fn run_tool_and_record_preserves_editing_tool_success_in_session_history() {
    let dir = tempfile::tempdir().expect("temp dir");
    let session_id = unique_session("tools-single-write");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx_with_root(&session_id, dir.path());
    let mut state = new_state();
    let mut events = Vec::new();

    let output = super::run_tool_and_record(
        &mut session,
        "write",
        &json!({ "path": "created.txt", "content": "alpha\nbeta\n" }).to_string(),
        &ctx,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    )
    .expect("write output");

    assert_eq!(std::fs::read_to_string(dir.path().join("created.txt")).unwrap(), "alpha\nbeta\n");
    assert!(output.contains("Created created.txt."));
    assert_eq!(session.messages.len(), 1);
    assert!(session.messages[0].content.contains("tool write"));
    assert!(session.messages[0].content.contains("\"status\":\"completed\""));
    assert_eq!(state.non_todo_tool_runs, 1);
    assert!(
        delta_texts(&events)
            .iter()
            .any(|text| text.contains("Created created.txt") && text.contains("\"result\""))
    );
}

#[test]
fn run_batch_tool_and_record_accepts_tool_calls_alias_and_summarizes_mixed_results() {
    let session_id = unique_session("tools-batch-mixed");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id);
    let mut state = new_state();

    let output = super::run_batch_tool_and_record(
        &mut session,
        &json!({
            "toolCalls": [
                { "tool": "todoread" },
                { "tool": "unknown_tool", "parameters": {} }
            ]
        })
        .to_string(),
        &ctx,
        false,
        &mut |_event| true,
        &mut state,
    );

    assert!(output.contains("已成功执行 1/2 个工具，失败 1 个"));
    assert!(output.contains("错误："));
    assert!(output.contains("- unknown_tool: Unknown tool: unknown_tool"));
    assert_eq!(session.messages.len(), 3);
    assert_eq!(state.non_todo_tool_runs, 1);
}

#[test]
fn run_batch_tool_and_record_records_denied_subtool_with_permission_request() {
    let dir = tempfile::tempdir().expect("temp dir");
    let session_id = unique_session("tools-batch-denied");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx_with_root(&session_id, dir.path());
    let mut state = new_state();
    let mut events = Vec::new();

    let output = super::run_batch_tool_and_record(
        &mut session,
        &json!({
            "tool_calls": [
                { "tool": "bash", "parameters": { "command": "touch denied-batch" } }
            ]
        })
        .to_string(),
        &ctx,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    );

    assert!(output.contains("已成功执行 0/1 个工具，失败 1 个"));
    assert!(output.contains("Approval required"));
    assert!(!dir.path().join("denied-batch").exists());
    assert_eq!(session.messages.len(), 2);
    assert!(session.messages[0].content.contains("tool bash"));
    assert!(session.messages[0].content.contains("\"status\":\"denied\""));
    assert!(session.messages[0].content.contains("\"permission_request\""));
    assert_eq!(state.non_todo_tool_runs, 1);
    assert!(
        delta_texts(&events)
            .iter()
            .any(|text| text.contains("tool bash") && text.contains("\"status\":\"denied\""))
    );
}

#[test]
fn run_batch_tool_and_record_preserves_editing_subtool_and_parallel_ui_events() {
    let dir = tempfile::tempdir().expect("temp dir");
    let session_id = unique_session("tools-batch-write-and-ui");
    let ctx = ctx_with_root(&session_id, dir.path());

    let mut write_session = Session::new(format!("{session_id}-write"));
    let mut write_state = new_state();
    let write_output = super::run_batch_tool_and_record(
        &mut write_session,
        &json!({
            "tool_calls": [
                { "tool": "write", "parameters": { "path": "batch-created.txt", "content": "batch\n" } }
            ]
        })
        .to_string(),
        &ctx,
        false,
        &mut |_event| true,
        &mut write_state,
    );

    assert_eq!(std::fs::read_to_string(dir.path().join("batch-created.txt")).unwrap(), "batch\n");
    assert!(write_output.contains("全部 1 个工具均执行成功"));
    assert_eq!(write_session.messages.len(), 2);
    assert!(write_session.messages[0].content.contains("tool write"));
    assert_eq!(write_state.non_todo_tool_runs, 1);

    let mut parallel_session = Session::new(format!("{session_id}-parallel"));
    let mut parallel_state = new_state();
    let mut events = Vec::new();
    let parallel_output = super::run_batch_tool_and_record(
        &mut parallel_session,
        &json!({
            "tool_calls": [
                { "tool": "todoread", "parameters": {} },
                { "tool": "todoread", "parameters": {} }
            ]
        })
        .to_string(),
        &ctx,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut parallel_state,
    );

    assert!(parallel_output.contains("全部 2 个工具均执行成功"));
    assert!(
        delta_texts(&events)
            .iter()
            .filter(
                |text| text.contains("tool todoread") && text.contains("\"status\":\"completed\"")
            )
            .count()
            >= 2
    );
    assert!(delta_texts(&events).iter().any(|text| text.contains("tool batch")));
}

#[test]
fn run_batch_tool_and_record_emits_running_and_error_events_for_streaming_subtool() {
    let session_id = unique_session("tools-batch-streaming-error");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id);
    let mut state = new_state();
    let mut events = Vec::new();

    let output = super::run_batch_tool_and_record(
        &mut session,
        &json!({ "tool_calls": [{ "tool": "file_read", "parameters": {} }] }).to_string(),
        &ctx,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    );
    let deltas = delta_texts(&events);

    assert!(output.contains("已成功执行 0/1 个工具，失败 1 个"));
    assert!(deltas.iter().any(|text| {
        text.contains("tool file_read") && text.contains("\"status\":\"running\"")
    }));
    assert!(
        deltas.iter().any(|text| {
            text.contains("tool file_read") && text.contains("\"status\":\"error\"")
        })
    );
    assert!(
        deltas.iter().any(|text| {
            text.contains("tool batch") && text.contains("\"status\":\"completed\"")
        })
    );
    assert_eq!(state.non_todo_tool_runs, 1);
}

#[test]
fn run_batch_tool_and_record_deduplicates_subtool_and_batch_messages() {
    let session_id = unique_session("tools-batch-deduplicate");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id);
    let mut state = new_state();
    let input = json!({ "tool_calls": [{ "tool": "todoread", "parameters": {} }] }).to_string();

    let first = super::run_batch_tool_and_record(
        &mut session,
        &input,
        &ctx,
        false,
        &mut |_event| true,
        &mut state,
    );
    let second = super::run_batch_tool_and_record(
        &mut session,
        &input,
        &ctx,
        false,
        &mut |_event| true,
        &mut state,
    );

    assert_eq!(first, second);
    assert_eq!(session.messages.len(), 2);
    assert_eq!(state.seen.len(), 2);
    assert_eq!(state.non_todo_tool_runs, 0);
}

#[test]
fn run_batch_tool_and_record_rewrites_todowrite_only_before_non_todo_work() {
    let initial_session_id = unique_session("tools-batch-todowrite-initial");
    let mut initial_session = Session::new(initial_session_id.clone());
    let initial_ctx = ctx(&initial_session_id);
    let mut initial_state = new_state();
    let input = json!({
        "tool_calls": [{
            "tool": "todowrite",
            "parameters": {
                "todos": [
                    { "id": "1", "content": "finish", "status": "completed", "priority": "high" }
                ]
            }
        }]
    })
    .to_string();

    super::run_batch_tool_and_record(
        &mut initial_session,
        &input,
        &initial_ctx,
        false,
        &mut |_event| true,
        &mut initial_state,
    );
    let initial_todos: Vec<todo::Todo> =
        serde_json::from_str(&todo::read(&initial_ctx).expect("read rewritten batch todos"))
            .expect("rewritten batch todos json");
    assert_eq!(initial_todos[0].status, "pending");

    let worked_session_id = unique_session("tools-batch-todowrite-after-work");
    let mut worked_session = Session::new(worked_session_id.clone());
    let worked_ctx = ctx(&worked_session_id);
    let mut worked_state = new_state();
    worked_state.non_todo_tool_runs = 1;

    super::run_batch_tool_and_record(
        &mut worked_session,
        &input,
        &worked_ctx,
        false,
        &mut |_event| true,
        &mut worked_state,
    );
    let worked_todos: Vec<todo::Todo> =
        serde_json::from_str(&todo::read(&worked_ctx).expect("read completed batch todos"))
            .expect("completed batch todos json");
    assert_eq!(worked_todos[0].status, "completed");
    assert_eq!(worked_state.non_todo_tool_runs, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn run_batch_tool_and_record_uses_existing_tokio_runtime_for_parallel_calls() {
    let session_id = unique_session("tools-batch-existing-runtime");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id);
    let mut state = new_state();

    let output = super::run_batch_tool_and_record(
        &mut session,
        &json!({
            "tool_calls": [
                { "tool": "todoread", "parameters": {} },
                { "tool": "todoread", "parameters": {} }
            ]
        })
        .to_string(),
        &ctx,
        false,
        &mut |_event| true,
        &mut state,
    );

    assert!(output.contains("全部 2 个工具均执行成功"));
    assert_eq!(session.messages.len(), 2);
    assert_eq!(state.non_todo_tool_runs, 0);
}

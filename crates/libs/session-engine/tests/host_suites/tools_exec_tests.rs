//! 验证工具执行事件、批处理并发、结果记录与子代理生命周期。

use crate::app::agent::session::processor::{StreamEvent, ToolCallStatus, ToolEventKind};
use crate::app::agent::session::session::{Role, Session};
use crate::app::agent::tools::{ToolCallError, ToolRuntimeContext, ToolUseContext, todo};
use crate::app::agent::tools::{ToolCallResult, ToolRenderHint};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);

#[test]
fn codex_discovery_processor_batch_inner_call_respects_request_snapshot() {
    let workspace = tempfile::tempdir().unwrap();
    let file = workspace.path().join("private.txt");
    std::fs::write(&file, "PRIVATE_FIXTURE_CONTENT").unwrap();
    let ctx = ToolRuntimeContext::new(
        "snapshot-batch",
        Some(workspace.path().to_string_lossy().into_owned()),
    )
    .with_test_config(crate::config::Config::default())
    .with_tool_use_context(
        ToolUseContext::new(
            "snapshot-batch",
            Some(workspace.path().to_string_lossy().into_owned()),
        )
        .with_tool_exposure_snapshot(["batch".to_string()]),
    );
    let mut session = Session::new("snapshot-batch".to_string());
    let output = super::run_batch_tool_and_record(
        &mut session,
        &json!({"tool_calls":[{"tool":"file_read","parameters":{"path":file}}]}).to_string(),
        &ctx,
        false,
        &mut |_| true,
        &mut Default::default(),
    );
    assert!(output.contains("not exposed"), "{output}");
    assert!(!output.contains("PRIVATE_FIXTURE_CONTENT"));
}





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

fn ctx_requiring_shell_approval(session: &str, root: &Path) -> ToolRuntimeContext {
    let mut config = crate::config::Config::default();
    config.autonomy.level = crate::security::AutonomyLevel::Supervised;
    config.autonomy.workspace_only = true;
    config.autonomy.allowed_commands = vec!["touch".to_string()];
    config.autonomy.require_approval_for_medium_risk = true;
    config.security.bash.enabled = true;
    ctx_with_root(session, root).with_test_config(config)
}

fn session_todos(ctx: &ToolRuntimeContext) -> Vec<todo::Todo> {
    let result = crate::tools::execute_tool_call("todoread", "{}", ctx).expect("scoped todo read");
    serde_json::from_value(result.data["todos"].clone()).expect("scoped todos")
}

fn new_state() -> super::super::ToolSessionState {
    super::super::ToolSessionState::default()
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
fn background_agent_launch_preserves_running_lifecycle_in_session_output() {
    let planned = super::AgentToolPlannedCall::new(
        "AgentTool",
        r#"{"action":"launch","agent":"writer","prompt":"inspect"}"#,
        ctx("parent-session"),
    );
    let created = crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated {
        parent_session_id: "parent-session".to_string(),
        child_session_id: "child-session".to_string(),
        agent: "writer".to_string(),
        task: "inspect".to_string(),
    };
    let result = ToolCallResult::from_legacy_result(crate::app::agent::tools::ToolResult {
        success: true,
        output: serde_json::json!({
            "action": "launch",
            "state": "running",
            "parent_session_id": "parent-session",
            "child_session_id": "child-session",
            "agent": "writer",
            "task": "inspect",
        })
        .to_string(),
        error: None,
    });

    let computed = super::compute_agent_tool_result(
        &planned,
        super::ToolExecutionWithAgentLifecycle { result: Ok(result), created: Some(created) },
    );
    let ui_payload = computed.ui_payload.clone();
    let session_payload: Value = serde_json::from_str(
        computed.session_message.strip_prefix("tool AgentTool\n").expect("tool prefix").trim(),
    )
    .expect("session payload");

    assert_eq!(ui_payload["status"], "running");
    assert_eq!(session_payload["status"], "running");
    assert_eq!(ui_payload["metadata"]["child_session_id"], "child-session");
    assert_eq!(session_payload["metadata"]["child_session_id"], "child-session");
}




#[cfg(not(target_arch = "wasm32"))]
#[test]
// 子代理并发启动，但汇总结果仍按原调用顺序稳定返回。
fn three_agent_tool_executions_start_in_parallel_and_keep_result_order() {
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let planned = (0..3)
        .map(|index| {
            super::AgentToolPlannedCall::new(
                "AgentTool",
                format!(r#"{{"agent":"worker","prompt":"task-{index}"}}"#),
                ctx(&format!("parent-{index}")),
            )
        })
        .collect::<Vec<_>>();
    let created = Arc::new(std::sync::Mutex::new(Vec::new()));
    let created_events = Arc::clone(&created);

    let outcomes = super::execute_agent_tools_parallel_with(
        &planned,
        4,
        &mut move |index, child| {
            created_events
                .lock()
                .expect("created event lock should not be poisoned")
                .push((index, child.child_session_id));
        },
        &|_name, input, runtime_ctx, callback| {
            let now_active = active.fetch_add(1, Ordering::SeqCst) + 1;
            max_active.fetch_max(now_active, Ordering::SeqCst);
            let prompt = serde_json::from_str::<Value>(input).expect("valid input")["prompt"]
                .as_str()
                .expect("prompt")
                .to_string();
            callback(crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated {
                parent_session_id: runtime_ctx.session.clone(),
                child_session_id: format!("child-{prompt}"),
                agent: "worker".to_string(),
                task: prompt.clone(),
            });
            std::thread::sleep(std::time::Duration::from_millis(80));
            active.fetch_sub(1, Ordering::SeqCst);
            Ok(ToolCallResult {
                data: json!({ "output": format!("done-{prompt}") }),
                ..ToolCallResult::default()
            })
        },
    );

    assert_eq!(max_active.load(Ordering::SeqCst), 3);
    assert_eq!(outcomes.len(), 3);
    assert_eq!(
        outcomes
            .iter()
            .map(|outcome| outcome.result.as_ref().expect("success").data["output"].as_str())
            .collect::<Vec<_>>(),
        vec![Some("done-task-0"), Some("done-task-1"), Some("done-task-2")]
    );
    assert_eq!(created.lock().expect("created lock").len(), 3);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn configured_agent_tool_limit_caps_parallel_execution() {
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let planned = (0..5)
        .map(|index| {
            super::AgentToolPlannedCall::new(
                "AgentTool",
                format!(r#"{{"prompt":"task-{index}"}}"#),
                ctx(&format!("parent-{index}")),
            )
        })
        .collect::<Vec<_>>();

    let outcomes = super::execute_agent_tools_parallel_with(
        &planned,
        2,
        &mut |_, _| {},
        &|_name, input, _runtime_ctx, _callback| {
            let now_active = active.fetch_add(1, Ordering::SeqCst) + 1;
            max_active.fetch_max(now_active, Ordering::SeqCst);
            let prompt = serde_json::from_str::<Value>(input).expect("valid input")["prompt"]
                .as_str()
                .expect("prompt")
                .to_string();
            std::thread::sleep(std::time::Duration::from_millis(40));
            active.fetch_sub(1, Ordering::SeqCst);
            Ok(ToolCallResult { data: json!({ "output": prompt }), ..ToolCallResult::default() })
        },
    );

    assert_eq!(max_active.load(Ordering::SeqCst), 2);
    assert_eq!(
        outcomes
            .iter()
            .map(|outcome| outcome.result.as_ref().expect("success").data["output"].as_str())
            .collect::<Vec<_>>(),
        vec![Some("task-0"), Some("task-1"), Some("task-2"), Some("task-3"), Some("task-4"),]
    );
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

    let todos = session_todos(&ctx);
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

    let todos = session_todos(&ctx);
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
fn run_batch_tool_and_record_passes_schema_args_and_legacy_parameters() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("input.txt");
    std::fs::write(&file, "batch argument fixture").unwrap();
    for key in ["args", "parameters"] {
        let session_id = unique_session("batch-schema-args");
        let mut session = Session::new(session_id.clone());
        let mut call = json!({"tool":"file_read"});
        call[key] = json!({"path":file});
        let output = super::run_batch_tool_and_record(
            &mut session,
            &json!({"tool_calls":[call]}).to_string(),
            &ctx_with_root(&session_id, dir.path()),
            false,
            &mut |_| true,
            &mut new_state(),
        );
        assert!(output.contains("batch argument fixture"), "{key}: {output}");
        assert!(output.contains("全部 1 个工具均执行成功"));
    }
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
    let ctx = ctx_requiring_shell_approval(&session_id, dir.path());
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

    assert!(output.contains("Approval required"), "{output}");
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
// 批处理中单个子工具被拒绝时保留权限请求上下文，不中断其他结果记录。
fn run_batch_tool_and_record_records_denied_subtool_with_permission_request() {
    let dir = tempfile::tempdir().expect("temp dir");
    let session_id = unique_session("tools-batch-denied");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx_requiring_shell_approval(&session_id, dir.path());
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
    assert!(output.contains("Approval required"), "{output}");
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
    let initial_todos = session_todos(&initial_ctx);
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
    let worked_todos = session_todos(&worked_ctx);
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









#[cfg(unix)]
#[test]
fn shell_session_batch_records_real_independent_handles() {
    use crate::tools::shell::{
        ShellTool,
        session_tools::{BashCancelTool, BashWaitTool},
    };
    let dir = tempfile::tempdir().unwrap();
    let id = unique_session("shell-batch");
    let policy = Arc::new(crate::security::SecurityPolicy {
        autonomy: crate::security::AutonomyLevel::Full,
        workspace_only: false,
        workspace_dir: dir.path().to_path_buf(),
        allowed_commands: vec!["printf".into(), "sleep".into()],
        forbidden_paths: vec![],
        ..Default::default()
    });
    let tools: Vec<Arc<dyn crate::tools::Tool>> = vec![
        Arc::new(ShellTool::new(policy.clone(), Arc::new(crate::runtime::NativeRuntime::new()))),
        Arc::new(BashWaitTool::new(policy.clone())),
        Arc::new(BashCancelTool::new(policy)),
    ];
    let ctx = ctx_with_root(&id, dir.path())
        .with_session_tools(crate::session::runtime::SessionToolset::new(tools));
    let mut history = Session::new(id);
    let mut state = new_state();
    let mut events = Vec::new();
    let output = super::run_batch_tool_and_record(
        &mut history,
        &json!({"toolCalls":[
            {"tool":"bash", "parameters":{"command":"printf first; sleep 60", "yield_time_ms":250}},
            {"tool":"bash", "parameters":{"command":"printf second; sleep 60", "yield_time_ms":250}}
        ]})
        .to_string(),
        &ctx,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    );
    let handles: Vec<_> = history
        .messages
        .iter()
        .filter_map(|message| {
            let body = message.content.strip_prefix("tool bash\n")?;
            let payload: Value = serde_json::from_str(body).unwrap();
            assert_eq!(payload["status"], "running");
            Some(payload["metadata"]["process_id"].as_str().unwrap().to_string())
        })
        .collect();
    // Reap real children before reporting any assertion failure.
    for handle in &handles {
        let cancelled = crate::tools::execute_tool_call(
            "bash_cancel",
            &json!({"process_id":handle}).to_string(),
            &ctx,
        )
        .unwrap();
        assert_eq!(cancelled.data["state"], "cancelled");
    }
    assert_eq!(handles.len(), 2);
    assert_ne!(handles[0], handles[1]);
    for handle in &handles {
        assert!(output.contains(handle));
    }
    let starts = events.iter().filter(|event| matches!(event, StreamEvent::Tool(tool) if tool.name == "bash" && tool.kind == ToolEventKind::Start)).count();
    let updates = events.iter().filter(|event| matches!(event, StreamEvent::Tool(tool) if tool.name == "bash" && tool.kind == ToolEventKind::Update && tool.status == ToolCallStatus::Running)).count();
    let completes = events.iter().filter(|event| matches!(event, StreamEvent::Tool(tool) if tool.name == "bash" && tool.kind == ToolEventKind::Complete)).count();
    assert_eq!((starts, updates, completes), (2, 2, 0));
    eprintln!(
        "real batch: two independent handles retained in model/history; two starts/two running updates/no completion; children reaped"
    );
}

#[cfg(unix)]
#[test]
fn shell_session_failed_command_retains_stderr_and_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let id = unique_session("shell-failed");
    let policy = Arc::new(crate::security::SecurityPolicy {
        autonomy: crate::security::AutonomyLevel::Full,
        workspace_only: false,
        workspace_dir: dir.path().to_path_buf(),
        allowed_commands: vec!["printf".into(), "exit".into()],
        forbidden_paths: vec![],
        ..Default::default()
    });
    let tools: Vec<Arc<dyn crate::tools::Tool>> = vec![
        Arc::new(crate::tools::shell::ShellTool::new(
            policy.clone(),
            Arc::new(crate::runtime::NativeRuntime::new()),
        )),
        Arc::new(crate::tools::shell::session_tools::BashWaitTool::new(policy)),
    ];
    let ctx = ctx_with_root(&id, dir.path())
        .with_session_tools(crate::session::runtime::SessionToolset::new(tools));
    let mut history = Session::new(id);
    let mut state = new_state();
    let mut events = Vec::new();
    let mut output = super::run_tool_and_record(
        &mut history,
        "bash",
        &json!({"command":"printf stderr >&2; exit 7", "yield_time_ms":250}).to_string(),
        &ctx,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut state,
    )
    .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut value: Value = serde_json::from_str(&output).unwrap();
    while value["state"] == "running" && std::time::Instant::now() < deadline {
        output = super::run_tool_and_record(
            &mut history,
            "bash_wait",
            &json!({"process_id":value["process_id"], "cursor":0, "yield_time_ms":250}).to_string(),
            &ctx,
            true,
            &mut |event| {
                events.push(event);
                true
            },
            &mut state,
        )
        .unwrap();
        value = serde_json::from_str(&output).unwrap();
    }
    assert_eq!(value["state"], "exited");
    assert_eq!(value["exit_code"], 7);
    assert_eq!(value["output"], "stderr");
    assert_eq!(value["success"], false);
    let payload: Value = serde_json::from_str(
        history
            .messages
            .last()
            .unwrap()
            .content
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
            .join("\n")
            .trim(),
    )
    .unwrap();
    assert_eq!(payload["status"], "failed");
    assert_eq!(payload["metadata"]["exit_code"], 7);
    assert!(events.iter().any(|event| matches!(event, StreamEvent::Tool(tool) if tool.kind == ToolEventKind::Complete && tool.status == ToolCallStatus::Failed && tool.payload["metadata"]["exit_code"] == 7)));
    eprintln!("real command reaped: state=exited exit_code=7 stderr retained in model/history/UI");
}

#[test]
fn codex_plugin_compat_t31_stream_preserves_app_without_content_blocks() {
    let result = crate::tools::ToolCallResult {
        data: serde_json::json!({"mcp_app":{"server":"panel","execution_binding":"host-binding"}}),
        ..Default::default()
    };
    let payload = super::completed_tool_payload_for_ui("panel_show", "{}", &result, "ready");
    assert_eq!(payload["data"]["mcp_app"], result.data["mcp_app"]);
    assert_eq!(payload["output"], "ready");
    assert!(payload.get("result").is_none());
}

#[test]
fn dynamic_tools_processor_catalog_preserves_approval_and_hook_denials() {
    struct DenyHook(Arc<AtomicUsize>);
    #[async_trait::async_trait]
    impl crate::hooks::HookHandler for DenyHook {
        fn name(&self) -> &str {
            "catalog-deny-fixture"
        }
        async fn before_tool_call(
            &self,
            _: String,
            _: Value,
        ) -> crate::hooks::HookResult<(String, Value)> {
            self.0.fetch_add(1, Ordering::SeqCst);
            crate::hooks::HookResult::Cancel("catalog hook denied".into())
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let session_id = unique_session("catalog-security");
    let ctx = ctx_requiring_shell_approval(&session_id, dir.path());
    let mut config = crate::config::Config::default();
    config.autonomy.level = crate::security::AutonomyLevel::Supervised;
    config.autonomy.workspace_only = true;
    config.autonomy.allowed_commands = vec!["touch".into()];
    config.autonomy.require_approval_for_medium_risk = true;
    let security = Arc::new(crate::security::SecurityPolicy::from_configs(
        &config.autonomy,
        &config.security,
        dir.path(),
    ));
    let registry = crate::tools::default_tools(security);
    let tools: Vec<Arc<dyn crate::tools::Tool>> = registry
        .into_iter()
        .filter(|tool| matches!(tool.spec().id.as_str(), "bash" | "file_read"))
        .map(Arc::from)
        .collect();
    let ids = tools.iter().map(|tool| tool.spec().id).collect::<Vec<_>>();
    let catalog = crate::tools::catalog::ToolCatalog::default();
    let snapshot = catalog.replace(tools, ids.clone()).unwrap();
    let ctx = super::with_request_tool_catalog(
        &ctx,
        &crate::session::runtime::SessionToolset::default(),
        &snapshot,
    );
    let ctx = ctx.clone().with_tool_use_context(
        ctx.tool_use_context().as_ref().clone().with_tool_exposure_snapshot(ids),
    );
    let mut session = Session::new(session_id);
    let mut events = Vec::new();
    let denied = super::run_tool_and_record(
        &mut session,
        "bash",
        &json!({"command":"touch denied-by-approval"}).to_string(),
        &ctx,
        true,
        &mut |event| {
            events.push(event);
            true
        },
        &mut new_state(),
    )
    .unwrap();
    assert!(denied.contains("Approval required"), "{denied}");
    assert!(!dir.path().join("denied-by-approval").exists());
    assert!(events.iter().any(
        |event| matches!(event, StreamEvent::Tool(tool) if tool.status == ToolCallStatus::Denied)
    ));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut hooks = crate::hooks::new_hook_runner();
    hooks.register(Box::new(DenyHook(calls.clone())));
    let ctx = ctx.clone().with_tool_use_context(
        ctx.tool_use_context().as_ref().clone().with_hook_runner(Arc::new(hooks)),
    );
    let file = dir.path().join("read.txt");
    std::fs::write(&file, "PRIVATE_READ_CONTENT").unwrap();
    let denied = super::run_tool_and_record(
        &mut session,
        "file_read",
        &json!({"path":file}).to_string(),
        &ctx,
        true,
        &mut |_| true,
        &mut new_state(),
    )
    .unwrap();
    assert!(denied.contains("catalog hook denied"), "{denied}");
    assert!(!denied.contains("PRIVATE_READ_CONTENT"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn native_tool_search_desktop_executor_requires_real_success_and_preserves_hooks() {
    struct SearchHook(bool);
    #[async_trait::async_trait]
    impl crate::hooks::HookHandler for SearchHook {
        fn name(&self) -> &str {
            "native-search-fixture"
        }
        async fn before_tool_call(
            &self,
            name: String,
            _: Value,
        ) -> crate::hooks::HookResult<(String, Value)> {
            if self.0 {
                crate::hooks::HookResult::Cancel("search denied by hook".into())
            } else {
                crate::hooks::HookResult::Continue((name, json!({"query":"beta"})))
            }
        }
    }
    for deny in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let session_id = unique_session("native-search-executor");
        let mut specs = ["alpha", "beta"]
            .into_iter()
            .map(|name| {
                let mut spec = crate::tools::ToolSpec::new(
                    name,
                    name,
                    json!({"type":"object","properties":{}}),
                );
                spec.exposure = vw_api_tools::ToolExposure::Deferred;
                spec
            })
            .collect::<Vec<_>>();
        let catalog = crate::tools::discovery::ToolDiscoveryCatalog::new(specs.clone());
        use crate::tools::Tool;
        specs.push(crate::tools::tool_search::ToolSearchTool::new(&catalog).spec());
        let discovery = Arc::new(crate::tools::tool_search::ToolDiscoverySession::new(specs));
        let mut hooks = crate::hooks::new_hook_runner();
        hooks.register(Box::new(SearchHook(deny)));
        let ctx = ctx_with_root(&session_id, dir.path())
            .with_test_config(crate::config::Config::default());
        let ctx = ctx.clone().with_tool_use_context(
            ctx.tool_use_context()
                .as_ref()
                .clone()
                .with_tool_discovery(discovery.clone())
                .with_tool_call_id("search-id")
                .with_tool_exposure_snapshot(["tool_search".into()])
                .with_hook_runner(Arc::new(hooks)),
        );
        let call = crate::session::llm::ToolCall {
            kind: crate::session::llm::ToolCallKind::Search,
            namespace: None,
            id: "search-id".into(),
            name: "tool_search".into(),
            arguments: json!({"query":"alpha"}).to_string(),
        };
        let result = super::run_native_search_and_record(
            &mut Session::new(session_id),
            &call,
            &ctx,
            &mut |_| true,
            &mut new_state(),
        );
        let ids = discovery.exposed_specs().into_iter().map(|spec| spec.id).collect::<Vec<_>>();
        if deny {
            assert!(result.unwrap_err().to_string().contains("search denied by hook"));
            assert_eq!(ids, vec!["tool_search"]);
            assert!(discovery.take_native_output(&call).is_err());
        } else {
            let value = serde_json::to_value(result.unwrap()).unwrap();
            assert_eq!(value["call_id"], "search-id");
            assert!(value["tools"].to_string().contains("beta"));
            assert!(!value["tools"].to_string().contains("alpha"));
            assert!(ids.contains(&"beta".into()));
            assert!(!ids.contains(&"alpha".into()));
        }
    }
}

#[test]
fn native_tool_search_desktop_ordinary_approval_and_hooks_still_deny() {
    dynamic_tools_processor_catalog_preserves_approval_and_hook_denials();
}

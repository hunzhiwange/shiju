//! 工具执行模块
//!
//! 本模块负责在会话上下文中执行工具调用，并记录执行结果。
//! 支持单个工具执行和批量工具并行执行两种模式。
//!
//! # 主要功能
//!
//! - **单个工具执行**：通过 `run_tool_and_record` 执行单个工具并记录到会话
//! - **批量工具执行**：通过 `run_batch_tool_and_record` 并行执行多个工具，提高执行效率
//! - **结果格式化**：将工具执行结果格式化为 UI 消息和会话消息
//! - **去重处理**：使用工具指纹避免重复记录相同的工具调用
//!
//! # 执行流程
//!
//! 1. 解析工具调用参数
//! 2. 执行工具并获取结果
//! 3. 格式化输出（包括 UI 显示和会话存储两种格式）
//! 4. 通过去重机制记录到会话历史

use super::types::{StreamEvent, ToolCallStatus, ToolEvent, ToolEventKind};
use super::utils::{
    compact_tool_output, compact_tool_output_for_ui, maybe_inject_file_link, sanitize_tool_input,
    sanitize_tool_input_for_ui, tool_fingerprint,
};
use crate::app::agent::tools::{self, ToolCallError, ToolRuntimeContext};
use crate::app::agent::tools::{is_todo_read_tool_id, is_todo_write_tool_id};
use crate::session::child_agent::coordinator::{
    ChildAgentCoordinator, ChildAgentTaskSnapshot, ChildAgentTaskState,
    global_child_agent_coordinator,
};
use std::sync::Arc;
use std::time::Duration;

const FULL_SESSION_OUTPUT_TOOLS: &[&str] =
    &["apply_patch", "write", "file_write", "file_edit", "notebook_edit"];

fn string_value(value: &str) -> serde_json::Value {
    serde_json::Value::String(value.to_string())
}

fn metadata_not_truncated() -> serde_json::Value {
    let mut metadata = serde_json::Map::new();
    metadata.insert("truncated".to_string(), serde_json::Value::Bool(false));
    serde_json::Value::Object(metadata)
}

fn empty_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

fn tool_call_status(payload: &serde_json::Value) -> ToolCallStatus {
    match payload.get("status").and_then(serde_json::Value::as_str) {
        Some("running") => ToolCallStatus::Running,
        Some("waiting") => ToolCallStatus::Waiting,
        Some("completed") => ToolCallStatus::Completed,
        Some("failed" | "error") => ToolCallStatus::Failed,
        Some("denied") => ToolCallStatus::Denied,
        Some("cancelled") => ToolCallStatus::Cancelled,
        Some(status) => ToolCallStatus::Other(status.to_string()),
        None => ToolCallStatus::Other("unknown".to_string()),
    }
}

fn lifecycle_kind(status: &ToolCallStatus) -> ToolEventKind {
    match status {
        ToolCallStatus::Running | ToolCallStatus::Waiting => ToolEventKind::Start,
        ToolCallStatus::Completed
        | ToolCallStatus::Failed
        | ToolCallStatus::Denied
        | ToolCallStatus::Cancelled
        | ToolCallStatus::Other(_) => ToolEventKind::Complete,
    }
}

fn tool_stream_event(
    call_id: String,
    name: &str,
    payload: serde_json::Value,
) -> StreamEvent {
    let status = tool_call_status(&payload);
    StreamEvent::Tool(ToolEvent {
        call_id,
        name: name.to_string(),
        kind: lifecycle_kind(&status),
        status,
        payload,
    })
}

fn tool_update_event(
    call_id: String,
    name: &str,
    payload: serde_json::Value,
) -> StreamEvent {
    let status = tool_call_status(&payload);
    let kind = if matches!(
        status,
        ToolCallStatus::Running | ToolCallStatus::Waiting
    ) {
        ToolEventKind::Update
    } else {
        ToolEventKind::Complete
    };
    StreamEvent::Tool(ToolEvent { call_id, name: name.to_string(), kind, status, payload })
}

fn fallback_tool_call_id(name: &str, sanitized_input: &str) -> String {
    format!("tool-{:016x}", tool_fingerprint(name, sanitized_input, ""))
}

fn stable_tool_call_id(ctx: &ToolRuntimeContext, name: &str, sanitized_input: &str) -> String {
    ctx.tool_use_context()
        .tool_call_id()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| fallback_tool_call_id(name, sanitized_input))
}

fn running_tool_payload(input: &str, title: &str) -> serde_json::Value {
    let mut payload = serde_json::Map::new();
    payload.insert("status".to_string(), string_value("running"));
    payload.insert("input".to_string(), string_value(input));
    payload.insert("title".to_string(), string_value(title));
    payload.insert("metadata".to_string(), metadata_not_truncated());
    payload.insert("output".to_string(), string_value(""));
    serde_json::Value::Object(payload)
}

fn agent_tool_running_payload(
    input: &str,
    created: &crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated,
) -> serde_json::Value {
    let mut payload = serde_json::Map::new();
    payload.insert("status".to_string(), string_value("running"));
    payload.insert("input".to_string(), string_value(input));
    payload.insert("title".to_string(), string_value("AgentTool"));
    payload.insert(
        "metadata".to_string(),
        serde_json::json!({
            "parent_session_id": created.parent_session_id,
            "child_session_id": created.child_session_id,
            "agent": created.agent,
            "task": created.task,
        }),
    );
    payload.insert("output".to_string(), string_value(""));
    serde_json::Value::Object(payload)
}

fn insert_agent_tool_metadata(
    payload: &mut serde_json::Map<String, serde_json::Value>,
    created: Option<&crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated>,
) {
    let Some(created) = created else {
        return;
    };
    payload.insert(
        "metadata".to_string(),
        serde_json::json!({
            "parent_session_id": created.parent_session_id,
            "child_session_id": created.child_session_id,
            "agent": created.agent,
            "task": created.task,
        }),
    );
}

fn error_payload_map(error: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut payload = serde_json::Map::new();
    payload.insert("status".to_string(), string_value("error"));
    payload.insert("error".to_string(), string_value(error));
    payload
}

fn error_tool_payload(error: &str) -> serde_json::Value {
    serde_json::Value::Object(error_payload_map(error))
}

fn error_tool_payload_with_input(input: &str, error: &str) -> serde_json::Value {
    let mut payload = error_payload_map(error);
    payload.insert("input".to_string(), string_value(input));
    serde_json::Value::Object(payload)
}

fn error_tool_payload_with_agent(
    input: Option<&str>,
    error: &str,
    created: Option<&crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated>,
) -> serde_json::Value {
    let mut payload = error_payload_map(error);
    if let Some(input) = input {
        payload.insert("input".to_string(), string_value(input));
    }
    insert_agent_tool_metadata(&mut payload, created);
    serde_json::Value::Object(payload)
}

fn insert_permission_request(
    payload: &mut serde_json::Map<String, serde_json::Value>,
    permission_request: Option<&vw_api_types::tools::PermissionRequestDto>,
) {
    let Some(permission_request) = permission_request else {
        return;
    };
    let value =
        serde_json::to_value(permission_request).expect("permission request DTO should serialize");
    payload.insert("permission_request".to_string(), value);
}

fn completed_session_payload(output: &str) -> serde_json::Value {
    let mut payload = serde_json::Map::new();
    payload.insert("status".to_string(), string_value("completed"));
    payload.insert("output".to_string(), string_value(output));
    serde_json::Value::Object(payload)
}

fn agent_tool_lifecycle_status(output: &str) -> &'static str {
    let Ok(body) = serde_json::from_str::<serde_json::Value>(output.trim()) else {
        return "completed";
    };
    let direct_state = body
        .get("state")
        .or_else(|| body.get("terminal_state"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .map(str::to_ascii_lowercase);
    match direct_state.as_deref() {
        Some("queued" | "running") => return "running",
        Some("cancelled") => return "cancelled",
        Some("failed") => return "failed",
        Some("succeeded" | "completed") => return "completed",
        _ => {}
    }
    if body.get("timed_out").and_then(serde_json::Value::as_bool) == Some(true) {
        return "waiting";
    }

    let Some(tasks) = body.get("tasks").and_then(serde_json::Value::as_array) else {
        return "completed";
    };
    let states = tasks
        .iter()
        .filter_map(|task| task.get("state"))
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    if states.iter().any(|state| state == "failed") {
        "failed"
    } else if states.iter().any(|state| matches!(state.as_str(), "queued" | "running")) {
        if body.get("action").and_then(serde_json::Value::as_str) == Some("status") {
            "running"
        } else {
            "waiting"
        }
    } else if !states.is_empty() && states.iter().all(|state| state == "cancelled") {
        "cancelled"
    } else {
        "completed"
    }
}

fn agent_tool_session_payload(
    status: &str,
    output: &str,
    created: Option<&crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated>,
    render_metadata: serde_json::Value,
) -> serde_json::Value {
    let mut payload = serde_json::Map::new();
    payload.insert("status".to_string(), string_value(status));
    payload.insert("output".to_string(), string_value(output));
    if created.is_some() {
        insert_agent_tool_metadata(&mut payload, created);
    } else if render_metadata.as_object().is_some_and(|metadata| !metadata.is_empty()) {
        payload.insert("metadata".to_string(), render_metadata);
    }
    serde_json::Value::Object(payload)
}

fn completed_batch_payload(input: &str, output: &str) -> serde_json::Value {
    let mut payload = serde_json::Map::new();
    payload.insert("status".to_string(), string_value("completed"));
    payload.insert("input".to_string(), string_value(input));
    payload.insert("title".to_string(), string_value("batch"));
    payload.insert("metadata".to_string(), metadata_not_truncated());
    payload.insert("output".to_string(), string_value(output));
    serde_json::Value::Object(payload)
}

fn preserve_full_output_in_session(name: &str) -> bool {
    FULL_SESSION_OUTPUT_TOOLS.contains(&name)
}

fn denied_payload_map(denied: &ToolCallError) -> serde_json::Map<String, serde_json::Value> {
    let mut payload = serde_json::Map::new();
    payload.insert("status".to_string(), string_value("denied"));
    payload.insert("error".to_string(), string_value(denied.message()));
    insert_permission_request(&mut payload, denied.permission_request());
    payload
}

fn denied_tool_payload(denied: &ToolCallError) -> serde_json::Value {
    serde_json::Value::Object(denied_payload_map(denied))
}

fn denied_tool_payload_with_input(input: &str, denied: &ToolCallError) -> serde_json::Value {
    let mut payload = denied_payload_map(denied);
    payload.insert("input".to_string(), string_value(input));
    serde_json::Value::Object(payload)
}

fn completed_tool_payload_for_ui(
    tool_name: &str,
    input: &str,
    result: &tools::ToolCallResult,
    output: &str,
) -> serde_json::Value {
    let mut payload = serde_json::Map::new();
    payload.insert("status".to_string(), string_value("completed"));
    payload.insert("input".to_string(), string_value(input));
    payload.insert("title".to_string(), string_value(&result.render_title(tool_name)));
    payload.insert("metadata".to_string(), result.render_metadata());
    payload.insert("output".to_string(), string_value(output));

    if result.content_blocks.is_empty() {
        return serde_json::Value::Object(payload);
    }

    let value = serde_json::to_value(result.to_dto()).expect("tool result DTO should serialize");
    payload.insert("result".to_string(), value);

    serde_json::Value::Object(payload)
}

#[derive(Debug, Clone)]
struct BatchPlannedCall {
    call_id: String,
    tool: String,
    input_effective: String,
    input_sanitized: String,
    input_ui: String,
    is_non_todo: bool,
}

#[derive(Debug, Clone)]
struct BatchComputed {
    call_id: String,
    tool: String,
    input_sanitized: String,
    ui_payload: serde_json::Value,
    session_message: String,
    is_non_todo: bool,
    success: bool,
    output_for_model: Option<String>,
    error_for_model: Option<String>,
}

#[derive(Debug, Clone)]
struct BatchSummaryRecord {
    call_id: String,
    input_sanitized: String,
    ui_payload: serde_json::Value,
    session_message: String,
}

fn completed_batch_summary_record(input: &str, output_ui: &str) -> BatchSummaryRecord {
    let input_sanitized = sanitize_tool_input("batch", input);
    let input_ui = sanitize_tool_input_for_ui("batch", input);
    BatchSummaryRecord {
        call_id: fallback_tool_call_id("batch", &input_sanitized),
        input_sanitized,
        ui_payload: completed_batch_payload(&input_ui, output_ui),
        session_message: format!("tool {}\n{}\n", "batch", completed_session_payload(output_ui)),
    }
}

fn batch_join_error_computed(error: impl std::fmt::Display) -> BatchComputed {
    let error = format!("Batch subtask join error: {error}");
    BatchComputed {
        call_id: fallback_tool_call_id("batch", ""),
        tool: "batch".to_string(),
        input_sanitized: String::new(),
        ui_payload: error_tool_payload_with_input("", &error),
        session_message: format!("tool {}\n{}\n", "batch", error_tool_payload(&error)),
        is_non_todo: false,
        success: false,
        output_for_model: None,
        error_for_model: Some(error),
    }
}

fn batch_model_output(ran: usize, computed_results: &[BatchComputed]) -> String {
    let successful = computed_results.iter().filter(|result| result.success).count();
    let failed = computed_results.len().saturating_sub(successful);
    let output_message = if ran == 0 {
        "未执行任何子任务".to_string()
    } else if failed > 0 {
        format!("已成功执行 {}/{} 个工具，失败 {} 个。", successful, computed_results.len(), failed)
    } else {
        format!(
            "全部 {} 个工具均执行成功。\n\n后续也可以继续使用 batch 工具以获得更高的执行效率。",
            successful
        )
    };

    let mut out = vec![output_message];
    let mut success_outputs: Vec<String> = computed_results
        .iter()
        .filter(|result| result.success)
        .filter_map(|result| result.output_for_model.as_deref())
        .map(str::trim)
        .filter(|output| !output.is_empty())
        .map(str::to_string)
        .collect();
    if !success_outputs.is_empty() {
        out.push(String::new());
        out.append(&mut success_outputs);
    }

    let errors: Vec<(&str, &str)> = computed_results
        .iter()
        .filter(|result| !result.success)
        .filter_map(|result| {
            result.error_for_model.as_deref().map(|error| (result.tool.as_str(), error))
        })
        .collect();
    if !errors.is_empty() {
        out.push(String::new());
        out.push("错误：".to_string());
        for (tool, error) in errors {
            out.push(format!("- {tool}: {error}"));
        }
    }

    out.join("\n\n")
}

fn batch_ui_output(ran: usize, ran_list: &[String]) -> String {
    if ran == 0 {
        "未执行任何子任务".to_string()
    } else {
        format!("已执行 {} 个子任务：\n{}", ran, ran_list.join("\n"))
    }
}

fn should_rewrite_todowrite(tool: &str, non_todo_runs: u64) -> bool {
    if !is_todo_write_tool_id(tool) {
        return false;
    }
    non_todo_runs == 0
}

fn is_non_todo_tool(tool: &str) -> bool {
    if is_todo_write_tool_id(tool) {
        return false;
    }
    !is_todo_read_tool_id(tool)
}

fn should_emit_running_tool(emit_ui: bool, tool: &str) -> bool {
    if !emit_ui {
        return false;
    }
    super::utils::is_streaming_tool(tool) && !is_agent_tool(tool)
}

pub(crate) fn is_agent_tool(tool: &str) -> bool {
    matches!(tool.trim().to_ascii_lowercase().as_str(), "agenttool" | "agent_tool" | "agent")
}

fn agent_tool_launches_background(input: &str) -> bool {
    let Ok(args) = serde_json::from_str::<serde_json::Value>(input.trim()) else {
        return false;
    };
    matches!(
        args.get("action")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        None | Some("launch" | "spawn")
    )
}

fn track_background_agent_child(
    tool_state: &mut super::ToolSessionState,
    input: &str,
    created: &crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated,
) {
    if !agent_tool_launches_background(input) {
        return;
    }
    tool_state.successful_agent_delegations =
        tool_state.successful_agent_delegations.saturating_add(1);
    if tool_state
        .agent_background_children
        .iter()
        .any(|child_session_id| child_session_id == &created.child_session_id)
    {
        return;
    }
    tool_state.agent_background_children.push(created.child_session_id.clone());
}

fn agent_tool_snapshot_status(snapshot: &ChildAgentTaskSnapshot, waiting: bool) -> &'static str {
    if waiting && !snapshot.state.is_terminal() {
        return "waiting";
    }
    match snapshot.state {
        ChildAgentTaskState::Queued | ChildAgentTaskState::Running => "running",
        ChildAgentTaskState::Succeeded => "completed",
        ChildAgentTaskState::Failed => "failed",
        ChildAgentTaskState::Cancelled => "cancelled",
    }
}

fn agent_tool_snapshot_payload(
    snapshot: &ChildAgentTaskSnapshot,
    waiting: bool,
) -> serde_json::Value {
    let output = serde_json::to_string(&serde_json::json!({
        "action": "status",
        "state": snapshot.state,
        "parent_session_id": snapshot.parent_session_id,
        "child_session_id": snapshot.child_session_id,
        "agent": snapshot.agent,
        "task": snapshot.task,
        "output": snapshot.output,
        "error": snapshot.error,
    }))
    .expect("child agent snapshot should serialize");
    serde_json::json!({
        "status": agent_tool_snapshot_status(snapshot, waiting),
        "title": "AgentTool",
        "metadata": {
            "parent_session_id": snapshot.parent_session_id,
            "child_session_id": snapshot.child_session_id,
            "agent": snapshot.agent,
            "task": snapshot.task,
        },
        "output": output,
    })
}

pub(crate) fn finalize_agent_tool_children(
    parent_session_id: &str,
    session: &mut super::Session,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &super::ToolSessionState,
) -> bool {
    finalize_agent_tool_children_with(
        &global_child_agent_coordinator(),
        parent_session_id,
        session,
        on_event,
        &tool_state.agent_background_children,
    )
}

fn finalize_agent_tool_children_with(
    coordinator: &ChildAgentCoordinator,
    parent_session_id: &str,
    session: &mut super::Session,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    child_session_ids: &[String],
) -> bool {
    if child_session_ids.is_empty() {
        return true;
    }

    let mut waiting_emitted = std::collections::HashSet::new();
    loop {
        let outcome = match coordinator.wait_for_descendants_blocking(
            parent_session_id,
            child_session_ids,
            Duration::from_millis(250),
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                let _ = coordinator.cancel_descendants(parent_session_id, child_session_ids);
                on_event(StreamEvent::Error(format!("AgentTool 子会话终态同步失败：{error}")));
                return false;
            }
        };

        if outcome.timed_out {
            for snapshot in outcome.tasks.iter().filter(|snapshot| {
                snapshot.parent_session_id == parent_session_id
                    && !snapshot.state.is_terminal()
                    && waiting_emitted.insert(snapshot.child_session_id.clone())
            }) {
                let payload = agent_tool_snapshot_payload(snapshot, true);
                if !on_event(tool_update_event(
                    snapshot.child_session_id.clone(),
                    "AgentTool",
                    payload,
                )) {
                    return false;
                }
            }
            continue;
        }

        for snapshot in
            outcome.tasks.iter().filter(|snapshot| snapshot.parent_session_id == parent_session_id)
        {
            let payload = agent_tool_snapshot_payload(snapshot, false);
            if !on_event(tool_update_event(
                snapshot.child_session_id.clone(),
                "AgentTool",
                payload.clone(),
            )) {
                return false;
            }
            let message = format!("tool AgentTool\n{payload}\n");
            session.push(super::Role::Tool, message);
        }
        return true;
    }
}

pub(crate) struct ToolExecutionWithAgentLifecycle {
    result: Result<tools::ToolCallResult, ToolCallError>,
    created: Option<crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated>,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentToolPlannedCall {
    name: String,
    input: String,
    input_sanitized: String,
    input_ui: String,
    ctx: ToolRuntimeContext,
}

impl AgentToolPlannedCall {
    pub(crate) fn new(
        name: impl Into<String>,
        input: impl Into<String>,
        ctx: ToolRuntimeContext,
    ) -> Self {
        let name = name.into();
        let input = input.into();
        Self {
            input_sanitized: sanitize_tool_input(&name, &input),
            input_ui: sanitize_tool_input_for_ui(&name, &input),
            name,
            input,
            ctx,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
enum AgentToolWorkerEvent {
    Created {
        index: usize,
        child: crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated,
    },
    Finished {
        index: usize,
        execution: ToolExecutionWithAgentLifecycle,
    },
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn execute_agent_tools_parallel_with<E, C>(
    planned: &[AgentToolPlannedCall],
    max_parallel: usize,
    on_created: &mut C,
    executor: &E,
) -> Vec<ToolExecutionWithAgentLifecycle>
where
    E: Fn(
            &str,
            &str,
            &ToolRuntimeContext,
            crate::app::agent::tools::agent_tool::AgentToolChildSessionCreatedCallback,
        ) -> Result<tools::ToolCallResult, ToolCallError>
        + Sync,
    C: FnMut(usize, crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated),
{
    let mut outcomes = (0..planned.len()).map(|_| None).collect::<Vec<_>>();
    let max_parallel = max_parallel.max(1);

    for (chunk_index, chunk) in planned.chunks(max_parallel).enumerate() {
        let chunk_start = chunk_index * max_parallel;
        std::thread::scope(|scope| {
            let (event_tx, event_rx) = std::sync::mpsc::channel();
            for (offset, call) in chunk.iter().enumerate() {
                let index = chunk_start + offset;
                let event_tx = event_tx.clone();
                scope.spawn(move || {
                    let created_slot = Arc::new(std::sync::Mutex::new(None::<
                        crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated,
                    >));
                    let callback_slot = Arc::clone(&created_slot);
                    let callback_tx = event_tx.clone();
                    let callback = Arc::new(
                        move |child: crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated| {
                        if let Ok(mut slot) = callback_slot.lock() {
                            *slot = Some(child.clone());
                        }
                        let _ = callback_tx.send(AgentToolWorkerEvent::Created { index, child });
                        },
                    );
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        executor(&call.name, &call.input, &call.ctx, callback)
                    }))
                    .unwrap_or_else(|_| {
                        Err(ToolCallError::Failed(
                            "AgentTool execution thread panicked".to_string(),
                        ))
                    });
                    let created = created_slot.lock().ok().and_then(|slot| slot.clone());
                    let _ = event_tx.send(AgentToolWorkerEvent::Finished {
                        index,
                        execution: ToolExecutionWithAgentLifecycle { result, created },
                    });
                });
            }
            drop(event_tx);

            let mut finished = 0;
            while finished < chunk.len() {
                match event_rx.recv() {
                    Ok(AgentToolWorkerEvent::Created { index, child }) => {
                        on_created(index, child);
                    }
                    Ok(AgentToolWorkerEvent::Finished { index, execution }) => {
                        outcomes[index] = Some(execution);
                        finished += 1;
                    }
                    Err(_) => break,
                }
            }
        });
    }

    outcomes
        .into_iter()
        .map(|outcome| {
            outcome.unwrap_or_else(|| ToolExecutionWithAgentLifecycle {
                result: Err(ToolCallError::Failed(
                    "AgentTool execution ended without a result".to_string(),
                )),
                created: None,
            })
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn execute_agent_tools_parallel_with<E, C>(
    planned: &[AgentToolPlannedCall],
    _max_parallel: usize,
    on_created: &mut C,
    executor: &E,
) -> Vec<ToolExecutionWithAgentLifecycle>
where
    E: Fn(
        &str,
        &str,
        &ToolRuntimeContext,
        crate::app::agent::tools::agent_tool::AgentToolChildSessionCreatedCallback,
    ) -> Result<tools::ToolCallResult, ToolCallError>,
    C: FnMut(usize, crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated),
{
    planned
        .iter()
        .enumerate()
        .map(|(index, call)| {
            let created_slot = Arc::new(std::sync::Mutex::new(
                None::<crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated>,
            ));
            let callback_slot = Arc::clone(&created_slot);
            let callback = Arc::new(
                move |child: crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated| {
                    if let Ok(mut slot) = callback_slot.lock() {
                        *slot = Some(child);
                    }
                },
            );
            let result = executor(&call.name, &call.input, &call.ctx, callback);
            let created = created_slot.lock().ok().and_then(|slot| slot.clone());
            if let Some(child) = created.clone() {
                on_created(index, child);
            }
            ToolExecutionWithAgentLifecycle { result, created }
        })
        .collect()
}

fn execute_tool_call_with_agent_lifecycle(
    name: &str,
    input: &str,
    input_ui: &str,
    ctx: &ToolRuntimeContext,
    emit_ui: bool,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
) -> ToolExecutionWithAgentLifecycle {
    if !is_agent_tool(name) {
        return ToolExecutionWithAgentLifecycle {
            result: tools::execute_tool_call(name, input, ctx),
            created: None,
        };
    }

    #[cfg(target_arch = "wasm32")]
    {
        ToolExecutionWithAgentLifecycle {
            result: tools::execute_tool_call(name, input, ctx),
            created: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let (created_tx, created_rx) = std::sync::mpsc::channel();
        let callback = Arc::new(move |created| {
            let _ = created_tx.send(created);
        });
        std::thread::scope(|scope| {
            let execution =
                scope.spawn(move || tools::execute_agent_tool_call(name, input, ctx, callback));
            let created = created_rx.recv().ok();
            if emit_ui {
                if let Some(created) = &created {
                    on_event(tool_stream_event(
                        stable_tool_call_id(ctx, name, input_ui),
                        name,
                        agent_tool_running_payload(input_ui, created),
                    ));
                }
            }
            ToolExecutionWithAgentLifecycle {
                result: execution.join().expect("AgentTool execution thread panicked"),
                created,
            }
        })
    }
}

fn record_computed_tool(
    session: &mut super::Session,
    computed: BatchComputed,
    emit_ui: bool,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut super::ToolSessionState,
) {
    if emit_ui {
        on_event(tool_stream_event(
            computed.call_id.clone(),
            &computed.tool,
            computed.ui_payload.clone(),
        ));
    }
    let fp = tool_fingerprint(&computed.tool, &computed.input_sanitized, &computed.session_message);
    if tool_state.seen.insert(fp) {
        session.push(super::Role::Tool, computed.session_message);
    }
    if computed.is_non_todo {
        tool_state.non_todo_tool_runs = tool_state.non_todo_tool_runs.saturating_add(1);
    }
}

fn compute_agent_tool_result(
    planned: &AgentToolPlannedCall,
    execution: ToolExecutionWithAgentLifecycle,
) -> BatchComputed {
    let created = execution.created.as_ref();
    let (ui_payload, session_message, success, output_for_model, error_for_model) = match execution
        .result
    {
        Ok(result) => {
            let output_full = maybe_inject_file_link(
                &planned.name,
                &planned.input,
                &planned.ctx,
                &result.model_text(),
            );
            let output_ui = compact_tool_output_for_ui(&planned.name, &output_full);
            let output_for_model = compact_tool_output(&planned.name, &output_full);
            let lifecycle_status = agent_tool_lifecycle_status(&output_full);
            let render_metadata = result.render_metadata();
            let output_session = if preserve_full_output_in_session(&planned.name) {
                output_full
            } else {
                output_for_model.clone()
            };
            let mut ui_payload = completed_tool_payload_for_ui(
                &planned.name,
                &planned.input_ui,
                &result,
                &output_ui,
            );
            if let Some(payload) = ui_payload.as_object_mut() {
                payload.insert("status".to_string(), string_value(lifecycle_status));
                insert_agent_tool_metadata(payload, created);
            }
            (
                ui_payload,
                format!(
                    "tool {}\n{}\n",
                    planned.name,
                    agent_tool_session_payload(
                        lifecycle_status,
                        &output_session,
                        created,
                        render_metadata,
                    )
                ),
                true,
                Some(output_for_model),
                None,
            )
        }
        Err(denied @ ToolCallError::Denied { .. }) => {
            let content = denied.message().to_string();
            (
                denied_tool_payload_with_input(&planned.input_ui, &denied),
                format!("tool {}\n{}\n", planned.name, denied_tool_payload(&denied)),
                false,
                None,
                Some(content),
            )
        }
        Err(ToolCallError::Failed(content)) => (
            error_tool_payload_with_agent(Some(&planned.input_sanitized), &content, created),
            format!(
                "tool {}\n{}\n",
                planned.name,
                error_tool_payload_with_agent(None, &content, created)
            ),
            false,
            None,
            Some(content),
        ),
    };

    BatchComputed {
        call_id: stable_tool_call_id(&planned.ctx, &planned.name, &planned.input_sanitized),
        tool: planned.name.clone(),
        input_sanitized: planned.input_sanitized.clone(),
        ui_payload,
        session_message,
        is_non_todo: is_non_todo_tool(&planned.name),
        success,
        output_for_model,
        error_for_model,
    }
}

pub(crate) fn run_agent_tools_and_record(
    session: &mut super::Session,
    planned: &[AgentToolPlannedCall],
    max_parallel: usize,
    emit_ui: bool,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut super::ToolSessionState,
) -> Vec<String> {
    for call in planned {
        let tool_input_for_trace = super::utils::truncate_string(&call.input_sanitized, 200);
        tracing::info!(
            target: "vw_agent",
            session_id = %call.ctx.session,
            tool_name = %call.name,
            tool_input = %tool_input_for_trace,
            "session processor executing AgentTool concurrently"
        );
    }

    let mut on_created =
        |index: usize,
         created: crate::app::agent::tools::agent_tool::AgentToolChildSessionCreated| {
            track_background_agent_child(tool_state, &planned[index].input, &created);
            if emit_ui {
                let call = &planned[index];
                on_event(tool_stream_event(
                    stable_tool_call_id(&call.ctx, &call.name, &call.input_sanitized),
                    &call.name,
                    agent_tool_running_payload(&call.input_ui, &created),
                ));
            }
        };
    let executions = execute_agent_tools_parallel_with(
        planned,
        max_parallel,
        &mut on_created,
        &tools::execute_agent_tool_call,
    );

    planned
        .iter()
        .zip(executions)
        .map(|(call, execution)| {
            let computed = compute_agent_tool_result(call, execution);
            let content = computed
                .output_for_model
                .clone()
                .or_else(|| computed.error_for_model.clone())
                .unwrap_or_default();
            record_computed_tool(session, computed, emit_ui, on_event, tool_state);
            content
        })
        .collect()
}

/// 执行单个工具并记录到会话
///
/// 此函数执行指定的工具调用，将结果格式化后记录到会话历史，
/// 并通过回调函数向 UI 发送事件通知。
///
/// # 参数
///
/// - `session`: 可变引用的会话对象，用于存储工具执行记录
/// - `name`: 工具名称（如 "bash"、"read"、"write" 等）
/// - `input`: 工具输入参数的 JSON 字符串
/// - `ctx`: 工具执行上下文，包含工作目录等环境信息
/// - `emit_ui`: 是否向 UI 发送事件通知
/// - `on_event`: 事件回调函数，用于处理流式事件
/// - `tool_state`: 工具会话状态，用于去重和统计
///
/// # 返回值
///
/// 返回 `Some(String)` 包含提供给模型的工具输出内容。
/// 对于 batch 工具，会在内部处理并返回其结果。
///
/// # 特殊处理
///
/// - **batch 工具**：内部转发到 `run_batch_tool_and_record` 处理
/// - **todowrite 工具**：当没有其他工具执行时，重写 "completed" 状态
/// - **流式工具**：在执行前发送 "running" 状态的 UI 事件
/// - **去重机制**：通过工具指纹避免重复记录相同的工具调用
///
/// # 工具执行结果
///
/// - **成功**：输出包含标题、元数据和结果
/// - **拒绝**：工具调用被安全策略拒绝
/// - **失败**：工具执行过程中发生错误
pub(crate) fn run_tool_and_record(
    session: &mut super::Session,
    name: &str,
    input: &str,
    ctx: &ToolRuntimeContext,
    emit_ui: bool,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut super::ToolSessionState,
) -> Option<String> {
    // batch 工具特殊处理：转发到批量执行函数
    if name == "batch" {
        let content = run_batch_tool_and_record(session, input, ctx, emit_ui, on_event, tool_state);
        return Some(content);
    }

    // 对 todowrite 工具进行特殊处理：当没有其他工具执行时，重写输入参数
    let input_effective = if should_rewrite_todowrite(name, tool_state.non_todo_tool_runs) {
        super::utils::rewrite_todowrite_completed_when_no_work(input)
    } else {
        input.to_string()
    };

    // 净化工具输入，移除敏感信息或格式化
    let input_sanitized = sanitize_tool_input(name, &input_effective);
    let input_ui = sanitize_tool_input_for_ui(name, &input_effective);
    let call_id = stable_tool_call_id(ctx, name, &input_sanitized);
    let tool_input_for_trace = super::utils::truncate_string(&input_sanitized, 200);
    tracing::info!(
        target: "vw_agent",
        session_id = %ctx.session,
        tool_name = name,
        tool_input = %tool_input_for_trace,
        "session processor executing tool"
    );

    // 对于流式工具，在执行前发送 "running" 状态的 UI 事件
    if should_emit_running_tool(emit_ui, name) {
        on_event(tool_stream_event(
            call_id.clone(),
            name,
            running_tool_payload(&input_ui, name),
        ));
    }

    // 执行工具并格式化结果消息
    let execution = execute_tool_call_with_agent_lifecycle(
        name,
        &input_effective,
        &input_ui,
        ctx,
        emit_ui,
        on_event,
    );
    let agent_child_created = execution.created;
    if let Some(created) = agent_child_created.as_ref() {
        track_background_agent_child(tool_state, &input_effective, created);
    }
    let (ui_payload, session_message, content_for_model) = match execution.result {
        // 工具执行成功的情况
        Ok(v) => {
            // 可能注入文件链接（例如在输出中添加可点击的文件路径）
            let output_full = maybe_inject_file_link(name, input, ctx, &v.model_text());
            // 为 UI 显示压缩输出（更简洁）
            let output_ui = compact_tool_output_for_ui(name, &output_full);
            // 为会话存储压缩输出（保留更多信息）
            let output_for_model = compact_tool_output(name, &output_full);
            let output_session = if preserve_full_output_in_session(name) {
                output_full.clone()
            } else {
                output_for_model.clone()
            };
            if is_agent_tool(name) {
                let lifecycle_status = agent_tool_lifecycle_status(&output_full);
                let mut ui_payload = completed_tool_payload_for_ui(name, &input_ui, &v, &output_ui);
                if let Some(payload) = ui_payload.as_object_mut() {
                    payload.insert("status".to_string(), string_value(lifecycle_status));
                    insert_agent_tool_metadata(payload, agent_child_created.as_ref());
                }
                (
                    ui_payload,
                    format!(
                        "tool {name}\n{}\n",
                        agent_tool_session_payload(
                            lifecycle_status,
                            &output_session,
                            agent_child_created.as_ref(),
                            v.render_metadata(),
                        )
                    ),
                    output_for_model,
                )
            } else {
                (
                    // UI 消息：包含完整信息（输入、标题、元数据和输出）
                    completed_tool_payload_for_ui(name, &input_ui, &v, &output_ui),
                    // 会话消息：仅包含状态和输出（节省上下文空间）
                    format!("tool {}\n{}\n", name, completed_session_payload(&output_session)),
                    output_for_model,
                )
            }
        }
        // 工具调用被拒绝的情况
        Err(denied) => match denied {
            ToolCallError::Denied { .. } => {
                let content = denied.message().to_string();
                (
                    denied_tool_payload_with_input(&input_ui, &denied),
                    format!("tool {}\n{}\n", name, denied_tool_payload(&denied)),
                    content,
                )
            }
            ToolCallError::Failed(content) => (
                error_tool_payload_with_agent(
                    Some(&input_sanitized),
                    &content,
                    agent_child_created.as_ref(),
                ),
                format!(
                    "tool {}\n{}\n",
                    name,
                    error_tool_payload_with_agent(None, &content, agent_child_created.as_ref(),)
                ),
                content,
            ),
        },
    };

    // 向 UI 发送事件通知
    if emit_ui {
        on_event(tool_update_event(call_id, name, ui_payload));
    }

    // 使用工具指纹进行去重：避免重复记录相同的工具调用
    let fp = tool_fingerprint(name, &input_sanitized, &session_message);
    if tool_state.seen.insert(fp) {
        session.push(super::Role::Tool, session_message);
    }

    // 更新非 todo 工具的执行计数（用于 todowrite 的特殊处理逻辑）
    if is_non_todo_tool(name) {
        tool_state.non_todo_tool_runs = tool_state.non_todo_tool_runs.saturating_add(1);
    }

    Some(content_for_model)
}

/// 执行批量工具调用并记录到会话
///
/// 此函数解析批量工具调用请求，并行执行多个工具，
/// 然后将所有结果汇总并记录到会话历史。
///
/// # 参数
///
/// - `input`: 批量工具调用的 JSON 字符串，包含 `tool_calls` 数组
/// - `ctx`: 工具执行上下文，包含工作目录等环境信息
/// - `emit_ui`: 是否向 UI 发送事件通知
/// - `on_event`: 事件回调函数，用于处理流式事件
/// - `tool_state`: 工具会话状态，用于去重和统计
///
/// # 返回值
///
/// 返回格式化的批量执行结果字符串，包含：
/// - 执行摘要（成功/失败数量）
/// - 所有成功工具的输出
/// - 所有失败工具的错误信息
///
/// # 输入格式
///
/// 输入应为 JSON 对象，包含 `tool_calls`、`calls` 或 `toolCalls` 数组：
///
/// ```json
/// {
///   "tool_calls": [
///     { "tool": "read", "parameters": { "filePath": "/path/to/file" } },
///     { "tool": "bash", "parameters": { "command": "ls -la" } }
///   ]
/// }
/// ```
///
/// # 并行执行策略
///
/// - **单个工具**：直接同步执行
/// - **多个工具**：使用 tokio 并行执行，最多同时执行 4 个工具
/// - **WASM 环境**：降级为顺序执行
///
/// # 限制
///
/// - 不支持递归批量调用（即 batch 工具不能调用另一个 batch 工具）
/// - 工具调用必须包含 `tool` 字段指定工具名称
///
/// # 错误处理
///
/// - 输入格式错误：返回错误消息并记录到会话
/// - 递归调用：立即返回错误并终止执行
/// - 单个工具失败：不影响其他工具执行
pub(crate) fn run_batch_tool_and_record(
    session: &mut super::Session,
    input: &str,
    ctx: &ToolRuntimeContext,
    emit_ui: bool,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut super::ToolSessionState,
) -> String {
    /// 在同步上下文中执行异步任务
    ///
    /// 此函数用于在同步代码中执行异步工具调用。
    ///
    /// # 参数
    ///
    /// - `fut`: 要执行的 Future
    ///
    /// # 返回值
    ///
    /// 返回 Future 的执行结果
    ///
    /// # WASM 限制
    ///
    /// 在 WASM 环境下会 panic，因为不支持阻塞式异步执行。
    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        // WASM 环境不支持阻塞式异步执行
        #[cfg(target_arch = "wasm32")]
        panic!("block_on not supported on WASM");

        #[cfg(not(target_arch = "wasm32"))]
        {
            // 尝试获取当前 tokio 运行时句柄
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                // 在已存在的运行时中阻塞执行
                return tokio::task::block_in_place(|| handle.block_on(fut));
            }
            // 创建新的单线程运行时并执行
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime")
                .block_on(fut)
        }
    }

    /// 计算单个工具的执行结果
    ///
    /// 执行指定的工具调用，并格式化为 UI 消息和会话消息。
    ///
    /// # 参数
    ///
    /// - `planned`: 计划执行的工具调用
    /// - `ctx`: 工具执行上下文
    ///
    /// # 返回值
    ///
    /// 返回包含完整执行结果的 `BatchComputed` 对象
    fn compute_single_tool(planned: BatchPlannedCall, ctx: ToolRuntimeContext) -> BatchComputed {
        let BatchPlannedCall {
            call_id,
            tool,
            input_effective,
            input_sanitized,
            input_ui,
            is_non_todo,
        } = planned;

        // 执行工具并格式化结果
        let (ui_payload, session_message, success, output_for_model, error_for_model) =
            match tools::execute_tool_call(&tool, &input_effective, &ctx) {
                // 工具执行成功
                Ok(v) => {
                    // 处理输出：注入文件链接、压缩显示
                    let output_full =
                        maybe_inject_file_link(&tool, &input_effective, &ctx, &v.model_text());
                    let output_ui = compact_tool_output_for_ui(&tool, &output_full);
                    let output_for_model = compact_tool_output(&tool, &output_full);
                    let output_session = if preserve_full_output_in_session(&tool) {
                        output_full.clone()
                    } else {
                        output_for_model.clone()
                    };
                    (
                        completed_tool_payload_for_ui(&tool, &input_ui, &v, &output_ui),
                        format!("tool {}\n{}\n", tool, completed_session_payload(&output_session)),
                        true,
                        Some(output_for_model),
                        None,
                    )
                }
                // 工具调用被拒绝
                Err(denied) => {
                    if matches!(denied, ToolCallError::Denied { .. }) {
                        let error_text = denied.message().to_string();
                        (
                            denied_tool_payload_with_input(&input_ui, &denied),
                            format!("tool {}\n{}\n", tool, denied_tool_payload(&denied)),
                            false,
                            None,
                            Some(error_text),
                        )
                    } else {
                        let error_text = denied.message().to_string();
                        (
                            error_tool_payload_with_input(&input_ui, &error_text),
                            format!("tool {}\n{}\n", tool, error_tool_payload(&error_text)),
                            false,
                            None,
                            Some(error_text),
                        )
                    }
                }
            };

        BatchComputed {
            call_id,
            tool,
            input_sanitized,
            ui_payload,
            session_message,
            is_non_todo,
            success,
            output_for_model,
            error_for_model,
        }
    }

    // ========== 开始解析和执行批量调用 ==========

    // 验证输入格式
    let raw = input.trim();
    if !raw.starts_with('{') {
        let error = "Invalid input format for batch".to_string();
        run_batch_error_and_record(session, input, emit_ui, on_event, tool_state, error.clone());
        return error;
    }

    // 解析 JSON 输入
    let v = match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(v) => v,
        Err(e) => {
            let error = format!(
                "The batch tool was called with invalid arguments: {}.\nPlease rewrite the input so it satisfies the expected schema.",
                e
            );
            run_batch_error_and_record(
                session,
                input,
                emit_ui,
                on_event,
                tool_state,
                error.clone(),
            );
            return error;
        }
    };

    // 提取工具调用数组（支持多种键名以兼容不同格式）
    let calls = v
        .get("tool_calls")
        .or_else(|| v.get("calls"))
        .or_else(|| v.get("toolCalls"))
        .and_then(|vv| vv.as_array())
        .cloned()
        .unwrap_or_default();

    // 初始化执行状态
    let mut ran = 0usize; // 已执行的工具计数
    let mut ran_list: Vec<String> = Vec::new(); // 已执行工具的描述列表（用于 UI 显示）
    let mut planned: Vec<BatchPlannedCall> = Vec::new(); // 计划执行的工具列表
    let mut computed_results: Vec<BatchComputed> = Vec::new(); // 执行结果列表
    let mut non_todo_runs_snapshot = tool_state.non_todo_tool_runs; // 快照当前的非 todo 工具计数

    // 遍历并规划所有工具调用
    for call in calls {
        // 提取工具名称
        let Some(tool) = call.get("tool").and_then(|x| x.as_str()) else {
            continue;
        };

        // 禁止递归批量调用
        if tool == "batch" {
            let error = "Recursive batch calls are not allowed".to_string();
            run_batch_error_and_record(
                session,
                input,
                emit_ui,
                on_event,
                tool_state,
                error.clone(),
            );
            return error;
        }

        // 提取工具参数（默认为空对象）
        let params = call.get("parameters").cloned().unwrap_or_else(empty_object);
        let input_raw = params.to_string();

        // 对 todowrite 工具进行特殊处理
        let input_effective = if should_rewrite_todowrite(tool, non_todo_runs_snapshot) {
            super::utils::rewrite_todowrite_completed_when_no_work(&input_raw)
        } else {
            input_raw
        };

        // 净化输入并标记工具类型
        let input_sanitized = sanitize_tool_input(tool, &input_effective);
        let input_ui = sanitize_tool_input_for_ui(tool, &input_effective);
        let is_non_todo = is_non_todo_tool(tool);
        let call_id = format!(
            "{}:{ran}",
            stable_tool_call_id(ctx, "batch", &sanitize_tool_input("batch", input))
        );

        // 对于流式工具，发送 "running" 状态事件
        if should_emit_running_tool(emit_ui, tool) {
            on_event(tool_stream_event(
                call_id.clone(),
                tool,
                running_tool_payload(&input_ui, tool),
            ));
        }

        // 添加到计划列表
        planned.push(BatchPlannedCall {
            call_id,
            tool: tool.to_string(),
            input_effective,
            input_sanitized,
            input_ui,
            is_non_todo,
        });

        // 生成工具调用的可读描述
        ran_list.push(describe_batch_call(tool, &params));
        ran += 1;

        // 更新非 todo 工具计数快照
        if is_non_todo {
            non_todo_runs_snapshot = non_todo_runs_snapshot.saturating_add(1);
        }
    }

    // ========== 执行工具调用 ==========

    // 单个工具：直接同步执行
    if planned.len() == 1 {
        let call = planned.remove(0);
        let computed = compute_single_tool(call, ctx.clone());
        computed_results.push(computed.clone());
        record_computed_tool(session, computed, emit_ui, on_event, tool_state);
    } else if planned.len() > 1 {
        // 多个工具：并行执行（限制最大并发数）
        const MAX_PARALLEL_BATCH: usize = 4; // 最大并行执行数量
        let planned_count = planned.len();

        // WASM 环境：降级为顺序执行
        #[cfg(target_arch = "wasm32")]
        let computed = {
            let mut results = Vec::with_capacity(planned_count);
            for item in planned {
                results.push(Some(compute_single_tool(item, ctx.clone())));
            }
            results
        };

        // 非 WASM 环境：使用 tokio 并行执行
        #[cfg(not(target_arch = "wasm32"))]
        let computed = block_on(async move {
            use std::sync::Arc;

            // 创建信号量限制并发数
            let sem = Arc::new(tokio::sync::Semaphore::new(MAX_PARALLEL_BATCH));
            let mut out: Vec<Option<BatchComputed>> = vec![None; planned_count];
            let mut set = tokio::task::JoinSet::new();

            // 为每个工具调用创建异步任务
            for (idx, item) in planned.into_iter().enumerate() {
                let sem = sem.clone();
                let ctx = ctx.clone();
                set.spawn(async move {
                    // 获取信号量许可（限制并发）
                    let _permit =
                        sem.acquire_owned().await.expect("batch semaphore should stay open");

                    // 在阻塞线程池中执行工具（避免阻塞异步运行时）
                    let computed =
                        tokio::task::spawn_blocking(move || compute_single_tool(item, ctx))
                            .await
                            .unwrap_or_else(batch_join_error_computed);
                    (idx, computed)
                });
            }

            // 收集所有任务结果
            while let Some(res) = set.join_next().await {
                let (idx, computed) = res.expect("batch join task should not panic");
                out[idx] = Some(computed);
            }
            out
        });

        // 处理并行执行的结果
        for computed in computed.into_iter().flatten() {
            computed_results.push(computed.clone());
            record_computed_tool(session, computed, emit_ui, on_event, tool_state);
        }
    }

    // ========== 汇总并格式化结果 ==========

    let output_for_model = batch_model_output(ran, &computed_results);

    // 生成 batch 工具的 UI 和会话消息
    let output_ui = batch_ui_output(ran, &ran_list);
    let summary_record = completed_batch_summary_record(input, &output_ui);

    // 发送 UI 事件并记录到会话
    if emit_ui {
        on_event(tool_stream_event(
            summary_record.call_id.clone(),
            "batch",
            summary_record.ui_payload.clone(),
        ));
    }
    let fp =
        tool_fingerprint("batch", &summary_record.input_sanitized, &summary_record.session_message);
    if tool_state.seen.insert(fp) {
        session.push(super::Role::Tool, summary_record.session_message);
    }

    output_for_model
}

/// 生成批量工具调用的可读描述
///
/// 此函数根据工具类型和参数生成简洁的人类可读描述，
/// 用于在 UI 中显示已执行的工具调用列表。
///
/// # 参数
///
/// - `tool`: 工具名称
/// - `params`: 工具参数的 JSON 值
///
/// # 返回值
///
/// 返回格式化的工具调用描述字符串，例如：
/// - `- bash ls -la`
/// - `- read /path/to/file`
/// - `- grep pattern=TODO path=src`
///
/// # 工具特定格式
///
/// - **bash**: 显示 command 参数
/// - **read/write/apply_patch**: 显示 filePath/path 参数
/// - **glob**: 显示 pattern 参数
/// - **grep**: 显示 pattern 和 path 参数
/// - **ls**: 显示 path 参数
/// - **其他工具**: 显示截断后的参数 JSON（最多 200 字符）
fn describe_batch_call(tool: &str, params: &serde_json::Value) -> String {
    let mut details: Option<String> = None;

    // 从参数对象中提取字符串值
    if let Some(obj) = params.as_object() {
        // 辅助函数：尝试从多个可能的键中获取字符串值
        let get = |keys: &[&str]| -> Option<String> {
            for k in keys {
                if let Some(v) = obj.get(*k).and_then(|x| x.as_str()).map(|s| s.trim()) {
                    if !v.is_empty() {
                        return Some(v.to_string());
                    }
                }
            }
            None
        };

        // 根据工具类型提取关键参数
        details = match tool {
            "bash" | "shell" => get(&["command"]).map(|v| v),
            "read" | "file_read" => {
                let path = get(&["filePath", "file_path", "path"]);
                let mut parts = Vec::new();
                if let Some(offset) = obj.get("offset").and_then(|x| x.as_i64()) {
                    parts.push(format!("offset={}", offset.max(1)));
                }
                if let Some(limit) = obj.get("limit").and_then(|x| x.as_i64()) {
                    parts.push(format!("limit={limit}"));
                }
                match (path, parts.is_empty()) {
                    (Some(path), true) => Some(path),
                    (Some(path), false) => Some(format!("{path} [{}]", parts.join(", "))),
                    (None, false) => Some(format!("[{}]", parts.join(", "))),
                    (None, true) => None,
                }
            }
            "write" | "file_write" | "file_edit" | "notebook_edit" | "apply_patch" => {
                get(&["file_path", "filePath", "path"])
            }
            "glob" => get(&["pattern"]).map(|v| v),
            "grep" => {
                let pat = get(&["pattern"]);
                let path = get(&["path"]);
                match (pat, path) {
                    (Some(pat), Some(path)) => Some(format!("pattern={} path={}", pat, path)),
                    (Some(pat), None) => Some(format!("pattern={}", pat)),
                    (None, Some(path)) => Some(format!("path={}", path)),
                    _ => None,
                }
            }
            "ls" => get(&["path"]).map(|v| v),
            _ => None,
        };
    }

    // 如果无法提取关键参数，使用截断后的参数 JSON 作为描述
    let detail = details.unwrap_or_else(|| {
        let raw = params.to_string();
        let sanitized = sanitize_tool_input(tool, &raw);
        super::utils::truncate_string(&sanitized, 200)
    });

    format!("- {} {}", tool, detail)
}

/// 记录批量工具执行错误
///
/// 此函数用于在批量工具执行失败时，记录错误信息到会话并向 UI 发送错误事件。
///
/// # 参数
///
/// - `session`: 可变引用的会话对象，用于存储错误记录
/// - `input`: 导致错误的原始输入字符串
/// - `emit_ui`: 是否向 UI 发送错误事件
/// - `on_event`: 事件回调函数，用于处理流式事件
/// - `tool_state`: 工具会话状态，用于去重
/// - `error`: 错误消息内容
///
/// # 生成的消息
///
/// - **UI 消息**: 包含状态 "error"、净化后的输入和错误详情
/// - **会话消息**: 包含状态 "error" 和错误详情（不含输入以节省空间）
fn run_batch_error_and_record(
    session: &mut super::Session,
    input: &str,
    emit_ui: bool,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut super::ToolSessionState,
    error: String,
) {
    // 净化输入并生成错误消息
    let input_sanitized = sanitize_tool_input("batch", input);
    let input_ui = sanitize_tool_input_for_ui("batch", input);
    let ui_payload = error_tool_payload_with_input(&input_ui, &error);
    let session_message = format!("tool {}\n{}\n", "batch", error_tool_payload(&error));

    // 发送 UI 错误事件
    if emit_ui {
        on_event(tool_stream_event(
            fallback_tool_call_id("batch", &input_sanitized),
            "batch",
            ui_payload,
        ));
    }

    // 使用工具指纹去重并记录到会话
    let fp = tool_fingerprint("batch", &input_sanitized, &session_message);
    if tool_state.seen.insert(fp) {
        session.push(super::Role::Tool, session_message);
    }
}
#[cfg(test)]
#[path = "tools_exec_tests.rs"]
mod tools_exec_tests;

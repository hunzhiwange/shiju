//! 桌面聊天流式接口。
//!
//! 本模块把 gateway chat 请求转换为 session processor 请求，并把处理器事件编码为
//! SSE 帧返回桌面端。它还负责合并 agent/model/options 覆盖项、把 tool 消息转换为
//! session 可理解的文本，以及在流结束后补写桌面会话消息。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::extract::Query;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::{Map, Value};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use vw_api_types::chat::{GatewayChatStreamRequest, ReasoningEffortDto};
use vw_api_types::id::TenantScope;

pub(super) use crate::session::runtime::is_successful_finish_reason;

use super::raw_commit;
use super::shared::with_tenant_instance;
use crate::app::agent::gateway::api::handlers::resumable_stream::{
    STREAM_REQUEST_ID_HEADER, StreamKind, StreamOpenError, global_stream_store, last_event_id,
    request_signature, sse_response, stream_request_id,
};

use crate::app::agent::agent;
use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::approval_state;
use crate::app::agent::gateway::instance::InstanceQuery;
use crate::app::agent::gateway::instance::resolve_directory;
use crate::app::agent::project::instance;
use crate::app::agent::provider::provider;
use crate::app::agent::session as agent_session;
use crate::app::agent::snapshot;
use crate::id;
use crate::session::ui_types as ui_models;

/// 单轮流式对话预分配的消息 id。
///
/// 预分配 id 让前端可以在流完成事件中得到稳定的 assistant/user 消息关联。
#[derive(Debug, Clone)]
pub(super) struct StreamTurnMessageIds {
    assistant_id: String,
    user_id: String,
}

impl StreamTurnMessageIds {
    /// 创建一组流式对话消息 id。
    ///
    /// # 参数
    ///
    /// * `assistant_id` - assistant 消息 id。
    /// * `user_id` - user 消息 id。
    ///
    /// # 返回值
    ///
    /// 返回用于持久化流式轮次的 id 容器。
    pub(super) fn new(assistant_id: impl Into<String>, user_id: impl Into<String>) -> Self {
        Self { assistant_id: assistant_id.into(), user_id: user_id.into() }
    }
}

#[derive(Debug)]
struct StreamStepSnapshot {
    step_index: u32,
    snapshot: Option<String>,
}

#[derive(Debug)]
struct StreamTrajectoryRecorder {
    turn_id: String,
    steps: Vec<ui_models::ChatSessionStep>,
    tool_results: Vec<agent_session::processor::ToolEvent>,
    response_text: String,
    final_step_has_text: bool,
}

impl StreamTrajectoryRecorder {
    fn new(turn_id: String) -> Self {
        Self {
            turn_id,
            steps: Vec::new(),
            tool_results: Vec::new(),
            response_text: String::new(),
            final_step_has_text: false,
        }
    }

    fn step_mut(&mut self, step_index: u32, occurred_ms: u64) -> &mut ui_models::ChatSessionStep {
        if let Some(position) = self.steps.iter().position(|step| step.index == step_index) {
            return &mut self.steps[position];
        }
        self.steps.push(ui_models::ChatSessionStep {
            index: step_index,
            turn_id: Some(self.turn_id.clone()),
            started_ms: occurred_ms,
            finished_ms: None,
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage: ui_models::TokenUsage::default(),
            cost_usd: None,
            finish_reason: None,
            model: None,
            tool_timings: Vec::new(),
        });
        self.steps.last_mut().expect("stream trajectory step was just inserted")
    }

    fn record(&mut self, event: &agent_session::processor::StreamEvent) {
        match event {
            agent_session::processor::StreamEvent::StepStart { step_index, created_ms, model } => {
                self.final_step_has_text = false;
                let step = self.step_mut(*step_index, *created_ms);
                step.started_ms = *created_ms;
                if model.is_some() {
                    step.model.clone_from(model);
                }
            }
            agent_session::processor::StreamEvent::Delta(text) => {
                self.response_text.push_str(text);
                self.final_step_has_text |= !text.trim().is_empty();
            }
            agent_session::processor::StreamEvent::Tool(event) => {
                let step_index = event_step_index(self.steps.as_slice());
                let step = self.step_mut(step_index, event.occurred_ms);
                let timing = if let Some(position) =
                    step.tool_timings.iter().position(|timing| timing.call_id == event.call_id)
                {
                    &mut step.tool_timings[position]
                } else {
                    step.tool_timings.push(ui_models::ToolTiming {
                        call_id: event.call_id.clone(),
                        name: event.name.clone(),
                        started_ms: None,
                        finished_ms: None,
                    });
                    step.tool_timings.last_mut().expect("stream tool timing was just inserted")
                };
                match event.kind {
                    agent_session::processor::ToolEventKind::Start => {
                        timing.started_ms.get_or_insert(event.occurred_ms);
                    }
                    agent_session::processor::ToolEventKind::Update => {}
                    agent_session::processor::ToolEventKind::Complete => {
                        timing.started_ms.get_or_insert(event.occurred_ms);
                        if timing.finished_ms.replace(event.occurred_ms).is_none() {
                            self.tool_results.push(event.clone());
                        }
                    }
                }
            }
            agent_session::processor::StreamEvent::StepFinish {
                step_index,
                finished_ms,
                usage,
                finish_reason,
                model,
            } => {
                let step = self.step_mut(*step_index, *finished_ms);
                step.finished_ms = Some(*finished_ms);
                step.usage = usage.clone();
                step.finish_reason.clone_from(finish_reason);
                if model.is_some() {
                    step.model.clone_from(model);
                }
            }
            _ => {}
        }
    }

    fn append_tool_parts(
        &self,
        session_id: &str,
        parts: &mut Vec<agent_session::message::Part>,
    ) -> anyhow::Result<()> {
        use agent_session::message::{
            Part, PartTime, ToolPart, ToolState, ToolStateCompleted, ToolStateCompletedTime,
            ToolStateError,
        };
        use agent_session::processor::ToolCallStatus;

        for event in &self.tool_results {
            let payload = &event.payload;
            let title = payload["title"].as_str().filter(|title| !title.trim().is_empty());
            let input = payload["input"]
                .as_object()
                .cloned()
                .or_else(|| {
                    payload["input"].as_str().and_then(|input| serde_json::from_str(input).ok())
                })
                .unwrap_or_default();
            let output = payload["output"]
                .as_str()
                .or_else(|| payload["error"].as_str())
                .unwrap_or_default()
                .to_owned();
            let state = if event.status == ToolCallStatus::Completed {
                ToolState::Completed(ToolStateCompleted {
                    input,
                    output,
                    title: title.unwrap_or(&event.name).to_owned(),
                    metadata: Map::new(),
                    time: ToolStateCompletedTime {
                        start: event.occurred_ms,
                        end: event.occurred_ms,
                        compacted: None,
                    },
                    attachments: None,
                })
            } else {
                ToolState::Error(ToolStateError {
                    input,
                    error: output,
                    metadata: None,
                    time: PartTime { start: event.occurred_ms, end: Some(event.occurred_ms) },
                })
            };
            let part_id = id::ascending(id::Prefix::Part, None)?;
            parts.push(Part::Tool(ToolPart {
                base: stream_part_base(session_id, &self.turn_id, part_id),
                call_id: event.call_id.clone(),
                tool: event.name.clone(),
                state,
                metadata: None,
            }));
        }
        Ok(())
    }
}

fn event_step_index(steps: &[ui_models::ChatSessionStep]) -> u32 {
    steps.last().map(|step| step.index).unwrap_or_default()
}

#[derive(Debug, Default)]
struct StreamPatchRecorder {
    active_step: Option<StreamStepSnapshot>,
    parts: Vec<agent_session::message::Part>,
}

impl StreamPatchRecorder {
    fn begin_step(&mut self, session_id: &str, assistant_id: &str, root: &str, step_index: u32) {
        let snapshot = snapshot::track(root).ok().flatten();
        self.active_step = Some(StreamStepSnapshot { step_index, snapshot: snapshot.clone() });

        let Some(part) = new_stream_step_start_part(session_id, assistant_id, snapshot) else {
            return;
        };
        self.parts.push(part);
    }

    fn finish_tool_round(
        &mut self,
        session_id: &str,
        assistant_id: &str,
        root: &str,
        step_index: u32,
    ) {
        let Some(active_step) = self.active_step.take() else {
            return;
        };
        if active_step.step_index != step_index {
            return;
        }
        let Some(start_snapshot) = active_step.snapshot else {
            return;
        };
        let Ok(patch) = snapshot::patch(root, &start_snapshot) else {
            return;
        };
        if patch.files.is_empty() {
            return;
        }

        let line_stats = if let Ok(Some(snapshot_after)) = snapshot::track(root) {
            let line_stats = snapshot::diff_full(root, &start_snapshot, &snapshot_after)
                .ok()
                .map(|diffs| patch_line_stats(&diffs));
            if let Some(part) =
                new_stream_step_finish_part(session_id, assistant_id, snapshot_after)
            {
                self.parts.push(part);
            }
            line_stats
        } else {
            None
        };

        let Some(part) =
            new_stream_patch_part(session_id, assistant_id, patch.hash, patch.files, line_stats)
        else {
            return;
        };
        self.parts.push(part);
    }

    fn take_parts(&mut self) -> Vec<agent_session::message::Part> {
        std::mem::take(&mut self.parts)
    }

    fn finish_active_step(&mut self, session_id: &str, assistant_id: &str, root: &str) {
        if let Some(step) = &self.active_step {
            self.finish_tool_round(session_id, assistant_id, root, step.step_index);
        }
    }
}

fn tool_event_kind_name(kind: agent_session::processor::ToolEventKind) -> &'static str {
    match kind {
        agent_session::processor::ToolEventKind::Start => "start",
        agent_session::processor::ToolEventKind::Update => "update",
        agent_session::processor::ToolEventKind::Complete => "complete",
    }
}

fn tool_call_status_name(status: &agent_session::processor::ToolCallStatus) -> &str {
    match status {
        agent_session::processor::ToolCallStatus::Running => "running",
        agent_session::processor::ToolCallStatus::Waiting => "waiting",
        agent_session::processor::ToolCallStatus::Completed => "completed",
        agent_session::processor::ToolCallStatus::Failed => "failed",
        agent_session::processor::ToolCallStatus::Denied => "denied",
        agent_session::processor::ToolCallStatus::Cancelled => "cancelled",
        agent_session::processor::ToolCallStatus::Other(status) => status,
    }
}

fn patch_line_stats(diffs: &[snapshot::FileDiff]) -> (i64, i64) {
    diffs.iter().fold((0_i64, 0_i64), |(additions, deletions), diff| {
        (additions.saturating_add(diff.additions), deletions.saturating_add(diff.deletions))
    })
}

fn stream_empty_token_info() -> agent_session::message::TokenInfo {
    agent_session::message::TokenInfo {
        total: Some(0),
        input: 0,
        output: 0,
        reasoning: 0,
        cache: agent_session::message::TokenCacheInfo { read: 0, write: 0 },
    }
}

fn stream_part_base(
    session_id: &str,
    assistant_id: &str,
    part_id: String,
) -> agent_session::message::PartBase {
    agent_session::message::PartBase {
        id: part_id,
        session_id: session_id.to_string(),
        message_id: assistant_id.to_string(),
    }
}

fn build_stream_step_start_part(
    session_id: &str,
    assistant_id: &str,
    part_id: String,
    snapshot: Option<String>,
) -> agent_session::message::Part {
    agent_session::message::Part::StepStart(agent_session::message::StepStartPart {
        base: stream_part_base(session_id, assistant_id, part_id),
        snapshot,
    })
}

fn new_stream_step_start_part(
    session_id: &str,
    assistant_id: &str,
    snapshot: Option<String>,
) -> Option<agent_session::message::Part> {
    let part_id = id::ascending(id::Prefix::Part, None).ok()?;
    Some(build_stream_step_start_part(session_id, assistant_id, part_id, snapshot))
}

fn build_stream_step_finish_part(
    session_id: &str,
    assistant_id: &str,
    part_id: String,
    snapshot: String,
) -> agent_session::message::Part {
    agent_session::message::Part::StepFinish(agent_session::message::StepFinishPart {
        base: stream_part_base(session_id, assistant_id, part_id),
        reason: "tool_round".to_string(),
        snapshot: Some(snapshot),
        cost: 0.0,
        tokens: stream_empty_token_info(),
    })
}

fn new_stream_step_finish_part(
    session_id: &str,
    assistant_id: &str,
    snapshot: String,
) -> Option<agent_session::message::Part> {
    let part_id = id::ascending(id::Prefix::Part, None).ok()?;
    Some(build_stream_step_finish_part(session_id, assistant_id, part_id, snapshot))
}

fn build_stream_patch_part(
    session_id: &str,
    assistant_id: &str,
    part_id: String,
    hash: String,
    files: Vec<String>,
    line_stats: Option<(i64, i64)>,
) -> agent_session::message::Part {
    let (additions, deletions) = line_stats
        .map_or((None, None), |(additions, deletions)| (Some(additions), Some(deletions)));
    agent_session::message::Part::Patch(agent_session::message::PatchPart {
        base: stream_part_base(session_id, assistant_id, part_id),
        hash,
        files,
        additions,
        deletions,
    })
}

fn new_stream_patch_part(
    session_id: &str,
    assistant_id: &str,
    hash: String,
    files: Vec<String>,
    line_stats: Option<(i64, i64)>,
) -> Option<agent_session::message::Part> {
    let part_id = id::ascending(id::Prefix::Part, None).ok()?;
    Some(build_stream_patch_part(session_id, assistant_id, part_id, hash, files, line_stats))
}

fn tool_content_to_session_text(content: &str) -> String {
    let parsed = serde_json::from_str::<Value>(content).ok();
    let tool_call_id =
        parsed.as_ref().and_then(|value| value.get("tool_call_id")).and_then(Value::as_str);
    let tool_output = parsed
        .as_ref()
        .and_then(|value| value.get("content"))
        .and_then(Value::as_str)
        .unwrap_or(content);

    match tool_call_id {
        Some(tool_call_id) => {
            format!(
                "[Tool results]\n<tool_result id=\"{tool_call_id}\">\n{tool_output}\n</tool_result>"
            )
        }
        None => format!("[Tool results]\n<tool_result>\n{tool_output}\n</tool_result>"),
    }
}

fn merge_gateway_request_options(
    options: Option<Value>,
    agent: Option<String>,
    allowed_tools: Option<Vec<String>>,
    chat_system_prompt: Option<String>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    acp_agent: Option<String>,
    acp_allowed_tools: Option<Vec<String>>,
    reasoning_effort: Option<ReasoningEffortDto>,
) -> Value {
    let mut merged = match options {
        Some(Value::Object(map)) => map,
        Some(_) | None => Map::new(),
    };

    if let Some(agent) = agent {
        merged.insert("agent".to_string(), Value::String(agent));
    }

    let allowed_tools_for_acp = allowed_tools.clone();
    if let Some(allowed_tools) = allowed_tools {
        merged.insert(
            "allowed_tools".to_string(),
            Value::Array(allowed_tools.into_iter().map(Value::String).collect()),
        );
    }

    if let Some(chat_system_prompt) = chat_system_prompt
        && !merged.contains_key("chat_system_prompt")
    {
        // 用户显式 options 优先；agent 默认值只填补缺失项，避免覆盖调用方意图。
        merged.insert("chat_system_prompt".to_string(), Value::String(chat_system_prompt));
    }

    if let Some(temperature) = temperature
        && !merged.contains_key("temperature")
    {
        merged.insert("temperature".to_string(), Value::from(temperature));
    }

    if let Some(top_p) = top_p
        && !merged.contains_key("top_p")
    {
        merged.insert("top_p".to_string(), Value::from(top_p));
    }

    if let Some(acp_agent) = acp_agent {
        merged.insert("acp_test".to_string(), Value::Bool(true));
        merged.insert("acp_agent".to_string(), Value::String(acp_agent));
    }

    let acp_requested = merged.get("acp_test").and_then(Value::as_bool).unwrap_or(false)
        || merged
            .get("acp_agent")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty());
    let acp_allowed_tools =
        acp_allowed_tools.or_else(|| if acp_requested { allowed_tools_for_acp } else { None });
    if let Some(acp_allowed_tools) = acp_allowed_tools {
        merged.insert("acp_test".to_string(), Value::Bool(true));
        merged.insert(
            "acp_allowed_tools".to_string(),
            Value::Array(acp_allowed_tools.into_iter().map(Value::String).collect()),
        );
    }

    merged.remove("reasoning_effort");
    merged.remove("thought_level");
    if let Some(reasoning_effort) = reasoning_effort {
        merged.insert(
            "reasoning_effort".to_string(),
            serde_json::to_value(reasoning_effort)
                .expect("ReasoningEffortDto always serializes to a string"),
        );
    }

    Value::Object(merged)
}

#[derive(Debug, Clone, Default)]
struct DelegateRequestOverrides {
    agent: Option<String>,
    model: Option<String>,
    allowed_tools: Option<Vec<String>>,
    chat_system_prompt: Option<String>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    default_allowed_skills: Option<Vec<String>>,
    skill_policy: Option<vw_config_types::agent::DelegateCapabilityPolicy>,
    trusted_user_selected_skills: Option<Vec<String>>,
}

#[derive(Debug)]
struct PreparedStreamHistory {
    history: Vec<ui_models::ChatMessage>,
    query: String,
    preview: String,
}

#[derive(Debug, Clone)]
pub(crate) struct GatewayChatStreamCollected {
    pub(crate) output: String,
    pub(crate) message_id: Option<String>,
    pub(crate) parent_message_id: Option<String>,
}

fn normalize_tool_ids(tools: Vec<String>) -> Option<Vec<String>> {
    let mut normalized = Vec::new();
    for tool in tools {
        let trimmed = tool.trim();
        if trimmed.is_empty() || normalized.iter().any(|existing| existing == trimmed) {
            continue;
        }
        normalized.push(trimmed.to_string());
    }
    (!normalized.is_empty()).then_some(normalized)
}

fn prepare_stream_history(
    history_messages: Vec<Value>,
    resume_history_only: bool,
    preserve_structured_prompt: bool,
) -> PreparedStreamHistory {
    let total_messages = history_messages.len();
    let mut history = Vec::new();
    let mut query = String::new();
    let mut preview = String::new();
    let mut prior_user_content = None;

    for (idx, message) in history_messages.iter().enumerate() {
        let role = message.get("role").and_then(Value::as_str).unwrap_or("user");
        let source = message.get("content").unwrap_or(&Value::Null);
        let content = gateway_content_to_prompt_text(source, preserve_structured_prompt);

        let normalized_content =
            if role == "tool" { tool_content_to_session_text(&content) } else { content };

        if idx + 1 == total_messages && role == "user" {
            query = normalized_content;
            preview = gateway_content_to_title_source(source);
            continue;
        }

        let mapped_role = match role {
            "assistant" => ui_models::ChatRole::Assistant,
            "system" => ui_models::ChatRole::System,
            "developer" => ui_models::ChatRole::Developer,
            "tool" | "native" => ui_models::ChatRole::Tool,
            _ => ui_models::ChatRole::User,
        };

        if mapped_role == ui_models::ChatRole::User {
            prior_user_content = message.get("content");
        }
        history.push(ui_models::ChatMessage {
            role: mapped_role,
            content: normalized_content,
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        });
    }

    if !resume_history_only
        && query.is_empty()
        && let Some(last_user_idx) =
            history.iter().rposition(|message| matches!(message.role, ui_models::ChatRole::User))
    {
        query = history.remove(last_user_idx).content;
        if let Some(content) = prior_user_content {
            preview = gateway_content_to_title_source(content);
        }
    }

    PreparedStreamHistory { history, query, preview }
}

fn gateway_content_to_title_source(content: &Value) -> String {
    let text = match content {
        Value::String(text) => std::borrow::Cow::Borrowed(text.as_str()),
        Value::Array(blocks) => std::borrow::Cow::Owned(
            blocks
                .iter()
                .filter(|block| block["type"] == "text")
                .filter_map(|block| block["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        _ => return String::new(),
    };
    agent_session::title::first_response_source(&text)
}

fn gateway_content_to_prompt_text(content: &Value, preserve_structured: bool) -> String {
    if let Some(text) = content.as_str() {
        return text.to_string();
    }
    let Some(blocks) = content.as_array() else {
        return String::new();
    };
    if preserve_structured {
        return serde_json::to_string(blocks).unwrap_or_default();
    }

    blocks
        .iter()
        .filter_map(|block| match block.get("type").and_then(Value::as_str) {
            Some("text") => block.get("text").and_then(Value::as_str).map(ToOwned::to_owned),
            Some("image") => {
                let mime_type = block.get("mimeType").and_then(Value::as_str)?;
                let data = block.get("data").and_then(Value::as_str)?;
                Some(format!("[IMAGE:data:{mime_type};base64,{data}]"))
            }
            Some("resource_link") => {
                block.get("uri").and_then(Value::as_str).map(|uri| format!("[Resource: {uri}]"))
            }
            Some("resource") => {
                let resource = block.get("resource")?;
                let uri = resource.get("uri").and_then(Value::as_str).unwrap_or_default();
                let text = resource.get("text").and_then(Value::as_str).unwrap_or_default();
                Some(if text.is_empty() {
                    format!("[Resource: {uri}]")
                } else {
                    format!("[Resource: {uri}]\n{text}")
                })
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn stream_payload_for_non_done_event(
    ev: agent_session::processor::StreamEvent,
    assistant_text: &mut String,
    assistant_finish_reason: &mut Option<String>,
    assistant_model: &mut Option<String>,
    current_step_index: &mut Option<u32>,
    stream_session_id: &str,
) -> Option<Value> {
    match ev {
        agent_session::processor::StreamEvent::LlmRawArtifact(_) => None,
        agent_session::processor::StreamEvent::ModelToolCalls(calls) => Some(serde_json::json!({
            "type": "chat.tool_calls", "tool_calls": calls,
        })),
        agent_session::processor::StreamEvent::ModelRecovery(state) => {
            let phase = match state.phase {
                agent_session::processor::RecoveryPhase::Waiting => "waiting",
                agent_session::processor::RecoveryPhase::Recovered => "recovered",
                agent_session::processor::RecoveryPhase::Stopped => "stopped",
                agent_session::processor::RecoveryPhase::Cancelled => "cancelled",
            };
            Some(serde_json::json!({
                "type": "chat.model_recovery", "session_id": stream_session_id,
                "attempt": state.attempt, "max_attempts": state.max_attempts,
                "wait_ms": state.wait_ms, "phase": phase,
                "reason": state.reason, "next_action": state.next_action,
            }))
        }
        agent_session::processor::StreamEvent::ContextLoaded { paths } => Some(serde_json::json!({
            "type": "chat.context_loaded",
            "paths": paths
        })),
        agent_session::processor::StreamEvent::Delta(delta) => {
            assistant_text.push_str(&delta);
            Some(serde_json::json!({ "type": "chat.delta", "delta": delta }))
        }
        agent_session::processor::StreamEvent::Tool(event) => {
            let separator =
                if assistant_text.is_empty() || assistant_text.ends_with('\n') { "" } else { "\n" };
            let delta = format!("{separator}tool {}\n{}\n", event.name, event.payload);
            assistant_text.push_str(&delta);
            let Some(step_index) = *current_step_index else {
                return Some(serde_json::json!({ "type": "chat.delta", "delta": delta }));
            };
            Some(serde_json::json!({
                "type": "chat.tool",
                "step_index": step_index,
                "call_id": event.call_id,
                "name": event.name,
                "kind": tool_event_kind_name(event.kind),
                "status": tool_call_status_name(&event.status),
                "occurred_ms": event.occurred_ms,
                "payload": event.payload,
            }))
        }
        agent_session::processor::StreamEvent::Compaction { phase } => {
            let phase = match phase {
                agent_session::processor::CompactionPhase::Started => "started",
                agent_session::processor::CompactionPhase::Completed => "completed",
            };
            Some(serde_json::json!({
                "type": "chat.compaction",
                "phase": phase,
            }))
        }
        agent_session::processor::StreamEvent::StepStart { step_index, created_ms, model } => {
            *current_step_index = Some(step_index);
            *assistant_finish_reason = None;
            Some(serde_json::json!({
                "type": "chat.step_start",
                "step_index": step_index,
                "created_ms": created_ms,
                "model": model
            }))
        }
        agent_session::processor::StreamEvent::StepFinish {
            step_index,
            finished_ms,
            usage,
            finish_reason,
            model,
        } => {
            if finish_reason.is_some() {
                *assistant_finish_reason = finish_reason.clone();
            }
            if model.is_some() {
                *assistant_model = model.clone();
            }
            Some(serde_json::json!({
                "type": "chat.step_finish",
                "step_index": step_index,
                "finished_ms": finished_ms,
                "usage": usage,
                "finish_reason": finish_reason,
                "model": model
            }))
        }
        agent_session::processor::StreamEvent::PostToolRound { step_index } => {
            Some(serde_json::json!({
                "type": "chat.post_tool_round",
                "step_index": step_index
            }))
        }
        agent_session::processor::StreamEvent::Error(err) => {
            Some(serde_json::json!({ "type": "chat.error", "error": err }))
        }
        agent_session::processor::StreamEvent::Done(_) => None,
    }
}

fn processor_event_from_prompt_event(
    event: agent_session::runtime::PromptEvent,
) -> agent_session::processor::StreamEvent {
    match event {
        agent_session::runtime::PromptEvent::LlmRawArtifact(artifact) => {
            agent_session::processor::StreamEvent::LlmRawArtifact(artifact)
        }
        agent_session::runtime::PromptEvent::ContextLoaded { paths } => {
            agent_session::processor::StreamEvent::ContextLoaded { paths }
        }
        agent_session::runtime::PromptEvent::ModelRecovery(state) => {
            agent_session::processor::StreamEvent::ModelRecovery(state)
        }
        agent_session::runtime::PromptEvent::Text(text) => {
            agent_session::processor::StreamEvent::Delta(text)
        }
        agent_session::runtime::PromptEvent::Tool(event) => {
            agent_session::processor::StreamEvent::Tool(event)
        }
        agent_session::runtime::PromptEvent::Compaction { phase } => {
            agent_session::processor::StreamEvent::Compaction { phase }
        }
        agent_session::runtime::PromptEvent::StepStart { step_index, created_ms, model } => {
            agent_session::processor::StreamEvent::StepStart { step_index, created_ms, model }
        }
        agent_session::runtime::PromptEvent::StepFinish {
            step_index,
            finished_ms,
            usage,
            finish_reason,
            model,
        } => agent_session::processor::StreamEvent::StepFinish {
            step_index,
            finished_ms,
            usage,
            finish_reason,
            model,
        },
        agent_session::runtime::PromptEvent::PostToolRound { step_index } => {
            agent_session::processor::StreamEvent::PostToolRound { step_index }
        }
        agent_session::runtime::PromptEvent::Terminated(termination) => match termination {
            agent_session::runtime::PromptTermination::Completed(usage) => {
                agent_session::processor::StreamEvent::Done(usage)
            }
            agent_session::runtime::PromptTermination::Cancelled => {
                agent_session::processor::StreamEvent::Error("cancelled".to_string())
            }
            agent_session::runtime::PromptTermination::Failed(error) => {
                agent_session::processor::StreamEvent::Error(error)
            }
        },
    }
}

#[cfg(test)]
async fn resolve_delegate_request_overrides(
    agent_name: Option<String>,
    allowed_tools: Option<Vec<String>>,
    explicit_model: Option<String>,
) -> Result<DelegateRequestOverrides, ApiError> {
    let agent_name = agent_name.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    });
    let explicit_allowed_tools =
        allowed_tools.map(|tools| normalize_tool_ids(tools).unwrap_or_default());

    let Some(agent_name) = agent_name else {
        return Ok(DelegateRequestOverrides {
            model: explicit_model,
            allowed_tools: explicit_allowed_tools,
            ..DelegateRequestOverrides::default()
        });
    };

    let Some(config) = agent::get(&agent_name).await else {
        return Err(ApiError::bad_request(format!("unknown agent: {}", agent_name)));
    };

    let runtime_config = crate::config::get_current()
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    resolve_delegate_overrides_from_config(
        agent_name,
        config,
        &runtime_config,
        explicit_allowed_tools,
        explicit_model,
    )
    .await
}

async fn resolve_delegate_overrides_from_config(
    agent_name: String,
    config: vw_config_types::agent::AgentDefinitionConfig,
    runtime_config: &crate::config::Config,
    explicit_allowed_tools: Option<Vec<String>>,
    explicit_model: Option<String>,
) -> Result<DelegateRequestOverrides, ApiError> {
    // Auto/blank delegates to agent defaults before the prompt's environment fallback.
    let explicit_model = explicit_model
        .filter(|model| !model.trim().is_empty() && !model.trim().eq_ignore_ascii_case("auto"));
    let workspace = instance::directory();
    let defaults = crate::session::capability_policy::native_agent_defaults(
        runtime_config,
        &agent_name,
        explicit_model.as_deref(),
        Path::new(&workspace),
    )
    .await
    .map_err(|error| ApiError::bad_request(error.to_string()))?;
    if config.force_model_selection
        && explicit_model.as_ref().is_some_and(|model| {
            !model.trim().eq_ignore_ascii_case("auto")
                && model != &format!("{}/{}", config.provider, config.model)
        })
    {
        return Err(ApiError::bad_request(
            "explicit model conflicts with forced agent model binding",
        ));
    }
    let fixed_model = match &config.defaults.model {
        vw_config_types::agent::AgentDefault::Fixed(model) => {
            Some(format!("{}/{}", model.provider, model.model))
        }
        vw_config_types::agent::AgentDefault::Inherit => None,
    };
    let model = defaults
        .as_ref()
        .and_then(|defaults| defaults.effective.model.as_ref())
        .map(|model| format!("{}/{}", model.provider, model.model))
        .or_else(|| delegated_model_override(&config, explicit_model.or(fixed_model)));
    let chat_system_prompt = config
        .system_prompt
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    // `main` 是交互式主代理，不是委托代理。它的工具清单已经由桌面根据当前
    // 注册目录和用户选择生成，不能再套用子代理的静态默认能力上限。
    if agent_name == "main" {
        return Ok(DelegateRequestOverrides {
            agent: Some(agent_name),
            model,
            allowed_tools: explicit_allowed_tools,
            default_allowed_skills: normalize_tool_ids(config.allowed_skills),
            skill_policy: Some(config.skill_policy),
            chat_system_prompt,
            temperature: config.temperature,
            top_p: config.top_p,
            ..DelegateRequestOverrides::default()
        });
    }

    let mut registered_tools = explicit_allowed_tools
        .unwrap_or_else(|| normalize_tool_ids(config.allowed_tools).unwrap_or_default());
    registered_tools.retain(|tool| {
        !matches!(tool.trim().to_ascii_lowercase().as_str(), "agenttool" | "agent_tool" | "agent")
    });

    Ok(DelegateRequestOverrides {
        agent: Some(agent_name),
        model,
        allowed_tools: Some(registered_tools),
        default_allowed_skills: normalize_tool_ids(config.allowed_skills),
        skill_policy: Some(config.skill_policy),
        trusted_user_selected_skills: None,
        chat_system_prompt,
        temperature: config.temperature,
        top_p: config.top_p,
    })
}

fn merge_delegate_capability_policy_options(
    mut options: Value,
    default_allowed_skills: Option<Vec<String>>,
    skill_policy: Option<vw_config_types::agent::DelegateCapabilityPolicy>,
    trusted_user_selected_skills: Option<Vec<String>>,
) -> Value {
    if let Some(options) = options.as_object_mut() {
        let inherited_strict =
            options.get("delegate_skill_policy").and_then(Value::as_str) == Some("strict");
        let inherited_skills = options
            .get("delegate_default_allowed_skills")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let default_allowed_skills = if inherited_strict {
            Some(match skill_policy {
                Some(vw_config_types::agent::DelegateCapabilityPolicy::Strict) => {
                    default_allowed_skills
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|name| inherited_skills.contains(&Value::String(name.clone())))
                        .collect()
                }
                _ => {
                    inherited_skills.iter().filter_map(Value::as_str).map(str::to_string).collect()
                }
            })
        } else {
            default_allowed_skills
        };
        let skill_policy = if inherited_strict {
            Some(vw_config_types::agent::DelegateCapabilityPolicy::Strict)
        } else {
            skill_policy
        };
        if default_allowed_skills.is_some() || skill_policy.is_some() {
            options.insert(
                "delegate_default_allowed_skills".to_string(),
                Value::Array(
                    default_allowed_skills
                        .unwrap_or_default()
                        .into_iter()
                        .map(Value::String)
                        .collect(),
                ),
            );
        }
        if let Some(skill_policy) = skill_policy {
            options.insert(
                "delegate_skill_policy".to_string(),
                serde_json::to_value(skill_policy)
                    .expect("DelegateCapabilityPolicy always serializes to a string"),
            );
        }
        if let Some(trusted_user_selected_skills) = trusted_user_selected_skills
            && !options.contains_key("delegate_trusted_user_selected_skills")
        {
            options.insert(
                "delegate_trusted_user_selected_skills".to_string(),
                Value::Array(trusted_user_selected_skills.into_iter().map(Value::String).collect()),
            );
        }
    }
    options
}

/// 将完整强制绑定置于显式父模型之前；普通 agent 绑定只作为 Auto 软偏好。
fn delegated_model_override(
    config: &crate::config::DelegateAgentConfig,
    explicit_model: Option<String>,
) -> Option<String> {
    if config.force_model_selection && config.has_complete_model_binding() {
        return agent::resolve_model_ref(config)
            .map(|model_ref| format!("{}/{}", model_ref.provider_id, model_ref.model_id));
    }
    explicit_model
}

async fn start_gateway_chat_stream(
    tenant: TenantScope,
    dir: String,
    body: GatewayChatStreamRequest,
    background_full_access: Option<bool>,
    abort_token: Option<tokio_util::sync::CancellationToken>,
) -> Result<UnboundedReceiver<Value>, ApiError> {
    let (tx, rx) = mpsc::unbounded_channel::<Value>();
    start_gateway_chat_producer(
        tenant,
        dir,
        body,
        background_full_access,
        abort_token,
        None,
        move |payload, _terminal| tx.send(payload).is_ok(),
    )
    .await?;
    Ok(rx)
}

fn validate_public_acp_chat_options(options: Option<&Value>) -> Result<(), ApiError> {
    let Some(options) = options else { return Ok(()) };
    if options.get("acp_session_mode").is_some() {
        return Err(ApiError::bad_request("acp_session_mode is private; use full_access"));
    }
    // On the first request even a custom permission config ID may not be known.
    // Only the session options endpoint can validate an ID against live capabilities.
    if options
        .get("acp_session_config")
        .is_some_and(|value| !value.as_object().is_some_and(Map::is_empty))
    {
        return Err(ApiError::bad_request(
            "acp_session_config is private; use the session ACP options endpoint for non-permission settings",
        ));
    }
    Ok(())
}

/// Carry the public model source into the processor's ACP runtime options.
/// Older clients supplied the source only in options; keep that explicit choice.
fn merge_acp_model_source(body: &mut GatewayChatStreamRequest) -> Result<(), ApiError> {
    use vw_api_types::acp_model_source::AcpModelSource;

    let mut options = match body.options.take() {
        None => Map::new(),
        Some(Value::Object(options)) => options,
        Some(_) => return Err(ApiError::bad_request("chat options must be an object")),
    };
    let source = match options.get("acp_model_source") {
        Some(value) => {
            let source: AcpModelSource = serde_json::from_value(value.clone())
                .map_err(|_| ApiError::bad_request("invalid acp_model_source"))?;
            if body.acp_model_source != AcpModelSource::SessionModel
                && body.acp_model_source != source
            {
                return Err(ApiError::bad_request("conflicting ACP model sources"));
            }
            source
        }
        None => body.acp_model_source,
    };
    body.acp_model_source = source;
    options.insert("acp_model_source".into(), serde_json::json!(source));
    body.options = Some(Value::Object(options));
    Ok(())
}

async fn prepare_gateway_agent_request(
    body: &mut GatewayChatStreamRequest,
    background_full_access: Option<bool>,
) -> Result<DelegateRequestOverrides, ApiError> {
    merge_acp_model_source(body)?;
    let options = body.options.as_ref();
    if options.is_some_and(|options| options.get("agent_binding").is_some()) {
        return Err(ApiError::bad_request("agent_binding is server-owned"));
    }
    let option_backend =
        options.and_then(|options| options.get("acp_agent")).and_then(Value::as_str);
    if body.acp_agent.as_deref().zip(option_backend).is_some_and(|(a, b)| a != b) {
        return Err(ApiError::bad_request("conflicting ACP backend selections"));
    }
    let backend = body.acp_agent.as_deref().or(option_backend).map(ToOwned::to_owned);
    let ephemeral =
        options.and_then(|options| options.get("goal_evaluation")).and_then(Value::as_bool)
            == Some(true)
            && body.session_id.as_ref().is_none_or(|id| id.0.trim().is_empty());
    if !ephemeral && body.session_id.as_ref().is_none_or(|id| id.0.trim().is_empty()) {
        body.session_id = Some(
            id::ascending(id::Prefix::Session, None)
                .map_err(|error| ApiError::internal(error.to_string()))?
                .into(),
        );
    }
    let session = body.session_id.as_ref().map(|id| id.0.as_str()).unwrap_or_default();
    let (binding, config) = crate::session::agent_binding::prepare(
        session,
        body.agent.as_deref(),
        backend.as_deref(),
        body.model.as_deref(),
        Path::new(&instance::directory()),
    )
    .await
    .map_err(|error| ApiError::bad_request(error.to_string()))?;
    match &binding.runtime {
        vw_config_types::agent::AgentRuntime::Native => {
            if options.and_then(|options| options.get("acp_test")).and_then(Value::as_bool)
                == Some(true)
            {
                return Err(ApiError::bad_request(
                    "native agent conflicts with ACP runtime options",
                ));
            }
            body.acp_agent = None;
        }
        vw_config_types::agent::AgentRuntime::Acp { backend } => {
            body.acp_agent = Some(backend.clone())
        }
    }
    let model = session_model(&body, &binding, session).await?;
    let explicit_full_access = options
        .and_then(|options| options.get("full_access"))
        .map(|value| {
            value.as_bool().ok_or_else(|| ApiError::bad_request("full_access must be a boolean"))
        })
        .transpose()?;
    let background_acp_access = background_full_access.filter(|_| body.acp_agent.is_some());
    let full_access = match background_acp_access {
        // The automation grant is for this turn only. Never persist it as a chat preference.
        Some(granted) => granted,
        None => crate::session::agent_binding::resolve_full_access(
            session,
            explicit_full_access,
            binding.defaults.full_access,
        )
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?,
    };
    let mut options = body.options.take().unwrap_or_else(|| serde_json::json!({}));
    let values = options
        .as_object_mut()
        .ok_or_else(|| ApiError::bad_request("options must be an object"))?;
    values.insert("full_access".into(), Value::Bool(full_access));
    if background_acp_access.is_some() {
        values
            .entry("acp_non_interactive_permissions")
            .or_insert_with(|| serde_json::json!("deny"));
    }
    body.options = Some(options);
    body.agent = Some(binding.agent.clone());
    let definition = config
        .agents
        .get(&binding.agent)
        .cloned()
        .ok_or_else(|| ApiError::internal("bound agent definition missing"))?;
    resolve_delegate_overrides_from_config(
        binding.agent,
        definition,
        &config,
        body.allowed_tools.clone().map(|tools| normalize_tool_ids(tools).unwrap_or_default()),
        model,
    )
    .await
}

async fn session_model(
    body: &GatewayChatStreamRequest,
    binding: &agent_session::agent_binding::AgentBinding,
    session: &str,
) -> Result<Option<String>, ApiError> {
    if session.is_empty() {
        return Ok(body.model.clone());
    }
    let fork_execution = if let Some(project) = instance::project() {
        agent_session::fork_execution::load_scoped(&project.id, session)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
    } else {
        None
    };
    let fork_selectable = fork_execution.as_ref().is_some_and(|state| !state.started);
    let acp = body.acp_agent.is_some();
    let locked = if !fork_selectable
        && acp
        && let Some(locked) = agent_session::llm::locked_acp_model(session, &instance::directory())
            .map_err(ApiError::bad_request)?
    {
        validate_session_model_source(body.acp_model_source, locked.source)?;
        locked.provider_id.zip(locked.model).map(|(provider, model)| format!("{provider}/{model}"))
    } else {
        let history = if fork_selectable {
            Vec::new()
        } else {
            agent_session::message::messages(session, Some(2))
                .await
                .map_err(|error| ApiError::internal(error.to_string()))?
                .into_iter()
                .filter(|entry| {
                    fork_execution.as_ref().is_none_or(|state| {
                        !state.inherited_message_ids.iter().any(|id| id == entry.info.id())
                    })
                })
                .collect()
        };
        let previous = history.iter().find_map(|entry| {
            let (provider, model) = match &entry.info {
                agent_session::message::Info::Assistant(assistant) => {
                    (&assistant.provider_id, &assistant.model_id)
                }
                agent_session::message::Info::User(user) => {
                    (&user.model.provider_id, &user.model.model_id)
                }
            };
            (!provider.is_empty() && !model.is_empty()).then_some((provider, model))
        });
        match previous {
            Some((provider, _)) if acp && provider == "acp" => {
                validate_session_model_source(
                    body.acp_model_source,
                    vw_api_types::acp_model_source::AcpModelSource::CliLocal,
                )?;
                None
            }
            Some((provider, model)) => {
                if acp {
                    validate_session_model_source(
                        body.acp_model_source,
                        vw_api_types::acp_model_source::AcpModelSource::SessionModel,
                    )?;
                }
                Some(format!("{provider}/{model}"))
            }
            None if acp => return Ok(body.model.clone()),
            None => match &binding.defaults.model {
                vw_config_types::agent::AgentDefault::Fixed(model) => {
                    Some(format!("{}/{}", model.provider, model.model))
                }
                vw_config_types::agent::AgentDefault::Inherit => None,
            },
        }
    };
    if let Some(locked) = &locked
        && body.model.as_deref().map(str::trim).is_some_and(|model| {
            !model.is_empty() && !model.eq_ignore_ascii_case("auto") && model != locked
        })
    {
        return Err(ApiError::bad_request("changing a session model requires a new session"));
    }
    Ok(locked)
}

fn validate_session_model_source(
    requested: vw_api_types::acp_model_source::AcpModelSource,
    locked: vw_api_types::acp_model_source::AcpModelSource,
) -> Result<(), ApiError> {
    if requested != locked {
        return Err(ApiError::bad_request(
            "changing a session model source requires a new session",
        ));
    }
    Ok(())
}

type SharedPromptCancel = Arc<Mutex<Option<agent_session::runtime::PromptCancelHandle>>>;

async fn start_gateway_chat_producer(
    tenant: TenantScope,
    dir: String,
    mut body: GatewayChatStreamRequest,
    background_full_access: Option<bool>,
    abort_token: Option<tokio_util::sync::CancellationToken>,
    prompt_cancel: Option<SharedPromptCancel>,
    send: impl FnMut(Value, bool) -> bool + Send + 'static,
) -> Result<(), ApiError> {
    let fork = match body.session_id.as_ref().map(|id| id.0.trim()).filter(|id| !id.is_empty()) {
        Some(id) => agent_session::session::forked_session(id)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?,
        None => None,
    };
    let project = instance::project()
        .ok_or_else(|| ApiError::internal("chat stream is missing project context"))?;
    let prompt_guard = match body.session_id.as_ref().map(|id| id.0.trim()) {
        Some(session_id) if !session_id.is_empty() => {
            Some(agent_session::message::lock_stream_prompt(&tenant, &project.id, session_id).await)
        }
        _ => None,
    };
    let (dir, fork_history) = match fork {
        Some(info) => {
            let history = agent_session::session::load_fork_history(&info)
                .await
                .map_err(|error| ApiError::bad_request(error.to_string()))?;
            (info.directory, Some(history))
        }
        None => (dir, None),
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        let metadata = tokio::fs::metadata(&dir).await.map_err(|error| {
            ApiError::bad_request(format!("无法访问会话工作目录 {dir}：{error}"))
        })?;
        if !metadata.is_dir() {
            return Err(ApiError::bad_request(format!("会话工作目录不是目录：{dir}")));
        }
    }
    let (session_project, _) = crate::project::from_directory(&tenant, &dir)
        .await
        .map_err(|error| ApiError::bad_request(format!("无法解析会话项目：{error}")))?;
    let delegate_overrides =
        prepare_gateway_agent_request(&mut body, background_full_access).await?;
    let GatewayChatStreamRequest {
        acp_model_source: _,
        session_id,
        messages,
        system: _,
        model: _,
        reasoning_effort,
        agent: _,
        allowed_tools: _,
        acp_agent,
        acp_allowed_tools,
        options,
    } = body;

    let model = delegate_overrides.model.clone();
    let message_agent = delegate_overrides.agent.clone().unwrap_or_else(|| "gateway".to_string());
    let preserve_structured_prompt = acp_agent.is_some();
    let background_acp = background_full_access.is_some() && preserve_structured_prompt;
    let options = merge_gateway_request_options(
        options,
        delegate_overrides.agent,
        delegate_overrides.allowed_tools,
        delegate_overrides.chat_system_prompt,
        delegate_overrides.temperature,
        delegate_overrides.top_p,
        acp_agent,
        acp_allowed_tools,
        reasoning_effort,
    );
    let options = merge_delegate_capability_policy_options(
        options,
        delegate_overrides.default_allowed_skills,
        delegate_overrides.skill_policy,
        delegate_overrides.trusted_user_selected_skills,
    );
    let resume_history_only =
        options.get("desktop_resume_history_only").and_then(Value::as_bool).unwrap_or(false);
    let approval_manager = approval_state::approval_manager_for_current_instance().await;
    let reliability = crate::config::get_project_for_tenant(&tenant, Path::new(&dir))
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?
        .reliability;

    let root_dir = dir;
    let project_context = instance::Context {
        directory: instance::directory(),
        worktree: instance::worktree(),
        project,
    };
    let goal_evaluation = options.get("goal_evaluation").and_then(Value::as_bool).unwrap_or(false);
    let stream_session_id =
        match session_id.as_ref().map(|value| value.0.trim()).filter(|value| !value.is_empty()) {
            Some(session_id) => session_id.to_string(),
            None if goal_evaluation => String::new(),
            None => id::ascending(id::Prefix::Session, None)
                .map_err(|error| ApiError::internal(error.to_string()))?,
        };
    let should_persist_app_session_artifacts = !goal_evaluation;
    let app_session_scope = Some(session_project.id);
    let stream_message_ids = if !should_persist_app_session_artifacts {
        None
    } else {
        Some(preallocate_stream_turn_message_ids()?)
    };
    let (history, query, preview, persisted_history) = match fork_history {
        Some((history, persisted)) => {
            let (query, preview) = if resume_history_only {
                (String::new(), String::new())
            } else {
                let message =
                    messages.last().filter(|msg| msg["role"] == "user").ok_or_else(|| {
                        ApiError::bad_request("fork continuation requires a new user message")
                    })?;
                (
                    gateway_content_to_prompt_text(&message["content"], preserve_structured_prompt),
                    gateway_content_to_title_source(&message["content"]),
                )
            };
            (history, query, preview, Some(persisted))
        }
        None => {
            let PreparedStreamHistory { history, query, preview } =
                prepare_stream_history(messages, resume_history_only, preserve_structured_prompt);
            (history, query, preview, None)
        }
    };
    if should_persist_app_session_artifacts {
        let reservation_tenant = tenant.clone();
        let project_id = app_session_scope.clone().expect("persistent stream has project scope");
        let session_id = stream_session_id.clone();
        let directory = root_dir.clone();
        crate::global_database::run_isolated(move || {
            reserve_stream_agent_session(
                &reservation_tenant,
                &project_id,
                &session_id,
                &directory,
                agent_session::session::now_ms(),
            )
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    }
    // The same prompt guard also serializes model-source changes. Rejections above
    // leave forks selectable; once execution is accepted, errors cannot unlock them.
    agent_session::fork_execution::start(&project_context.project.id, &stream_session_id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let runtime_handle = tokio::runtime::Handle::current();
    let delegated_context = crate::tools::current_tool_use_context();

    std::thread::spawn(move || {
        let _prompt_guard = prompt_guard;
        struct Sink<F> {
            send: F,
            sent_terminal: bool,
        }

        let sink = Arc::new(Mutex::new(Sink { send, sent_terminal: false }));
        let sink_for_run = Arc::clone(&sink);
        let outcome = catch_unwind(AssertUnwindSafe(move || {
            let processing_started_ms = agent_session::session::now_ms();
            let mut processing_timer = vw_shared::time::ActiveTimer::default();
            processing_timer.start_at(web_time::Instant::now());
            let mut assistant_text = String::new();
            let mut assistant_finish_reason: Option<String> = None;
            let mut assistant_model = model.clone();
            let patch_recorder = Arc::new(Mutex::new(StreamPatchRecorder::default()));
            let mut current_step_index = None;
            let mut trajectory_recorder = StreamTrajectoryRecorder::new(
                stream_message_ids.as_ref().map(|ids| ids.assistant_id.clone()).unwrap_or_default(),
            );
            let mut llm_timer = vw_shared::time::ActiveTimer::default();
            let approval_reply_target = if stream_session_id.trim().is_empty() {
                "desktop".to_string()
            } else {
                stream_session_id.clone()
            };
            let (approval_prompt_tx, _approval_prompt_rx) = tokio::sync::mpsc::unbounded_channel();

            let publication_cancellation = abort_token.clone();
            let turn = agent_session::ui_store::with_tenant_scope_sync(tenant.clone(), || {
                let start = || {
                    let recorder = Arc::clone(&patch_recorder);
                    let request = agent_session::processor::Request {
                        stream: 0,
                        session: stream_session_id.clone(),
                        query: query.clone(),
                        root: Some(root_dir.clone()),
                        model: model.clone(),
                        reliability: reliability.clone(),
                        options: options.clone(),
                        approval: Some(approval_manager.clone()),
                        channel_name: Some("desktop".to_string()),
                        non_cli_approval_context: (!background_acp).then(|| {
                            crate::agent::loop_::NonCliApprovalContext {
                                sender: "desktop".to_string(),
                                reply_target: approval_reply_target,
                                prompt_tx: approval_prompt_tx,
                            }
                        }),
                        assistant_message_id: stream_message_ids
                            .as_ref()
                            .map(|ids| ids.assistant_id.clone()),
                        history,
                        persist_app_session_artifacts: should_persist_app_session_artifacts,
                        app_session_scope: app_session_scope.clone(),
                        abort_token,
                        session_tools: crate::session::runtime::SessionToolset::default(),
                        session_skills: Default::default(),
                    };
                    agent_session::runtime::start_prompt_with_runner(
                        request,
                        move |request, mut emit| {
                            let session_id = request.session.clone();
                            let root = request.root.clone().expect("gateway prompt has a root");
                            let assistant_id = request.assistant_message_id.clone();
                            agent_session::processor::run_with_persisted_history(
                                request,
                                persisted_history,
                                move |event| {
                                    if let Some(assistant_id) = &assistant_id {
                                        // Capture Git state before the processor can advance to the next step.
                                        match &event {
                                        agent_session::processor::StreamEvent::StepStart {
                                            step_index,
                                            ..
                                        } => {
                                            recorder
                                                .lock()
                                                .unwrap_or_else(std::sync::PoisonError::into_inner)
                                                .begin_step(
                                                    &session_id,
                                                    assistant_id,
                                                    &root,
                                                    *step_index,
                                                );
                                        }
                                        agent_session::processor::StreamEvent::PostToolRound {
                                            step_index,
                                        } => {
                                            recorder
                                                .lock()
                                                .unwrap_or_else(std::sync::PoisonError::into_inner)
                                                .finish_tool_round(
                                                    &session_id,
                                                    assistant_id,
                                                    &root,
                                                    *step_index,
                                                );
                                        }
                                        // ACP executes its tools inside the model request.
                                        agent_session::processor::StreamEvent::StepFinish {
                                            step_index,
                                            ..
                                        } if preserve_structured_prompt => {
                                            recorder
                                                .lock()
                                                .unwrap_or_else(std::sync::PoisonError::into_inner)
                                                .finish_tool_round(
                                                    &session_id,
                                                    assistant_id,
                                                    &root,
                                                    *step_index,
                                                );
                                        }
                                        _ => {}
                                    }
                                    }
                                    emit(event)
                                },
                            )
                        },
                    )
                };
                match delegated_context {
                    Some(context) => crate::tools::scope_tool_use_context_sync(context, start),
                    None => start(),
                }
            });
            let cancel = turn.cancel.clone();
            if let Some(handle) = prompt_cancel {
                *handle.lock().unwrap_or_else(std::sync::PoisonError::into_inner) =
                    Some(cancel.clone());
            }
            while let Ok(prompt_event) = turn.events.recv() {
                let mut ev = processor_event_from_prompt_event(prompt_event);
                if let agent_session::processor::StreamEvent::LlmRawArtifact(artifact) = ev {
                    match raw_commit::register(&tenant, app_session_scope.as_deref(), artifact) {
                        Ok(()) => continue,
                        Err(error) => {
                            tracing::error!(
                                target: "vw_agent",
                                session_id = %stream_session_id,
                                error = %error,
                                "failed to buffer raw LLM artifact"
                            );
                            ev = agent_session::processor::StreamEvent::Error(
                                "failed to buffer raw LLM artifact".to_string(),
                            );
                        }
                    }
                }
                trajectory_recorder.record(&ev);
                let timer_now = web_time::Instant::now();
                match &ev {
                    agent_session::processor::StreamEvent::StepStart { .. } => {
                        llm_timer.start_at(timer_now);
                    }
                    agent_session::processor::StreamEvent::StepFinish { .. } => {
                        llm_timer.pause_at(timer_now);
                    }
                    _ => {}
                }
                let terminal = matches!(
                    ev,
                    agent_session::processor::StreamEvent::Done(_)
                        | agent_session::processor::StreamEvent::Error(_)
                );
                let patch_parts = if terminal {
                    processing_timer.pause_at(timer_now);
                    llm_timer.pause_at(timer_now);
                    let mut recorder =
                        patch_recorder.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    if matches!(ev, agent_session::processor::StreamEvent::Error(_))
                        && let Some(ids) = &stream_message_ids
                    {
                        recorder.finish_active_step(
                            &stream_session_id,
                            &ids.assistant_id,
                            &root_dir,
                        );
                    }
                    recorder.take_parts()
                } else {
                    Vec::new()
                };
                let mut summary_update = None;
                if matches!(ev, agent_session::processor::StreamEvent::PostToolRound { .. })
                    || (preserve_structured_prompt
                        && matches!(ev, agent_session::processor::StreamEvent::StepFinish { .. }))
                {
                    let parts = patch_recorder
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .parts
                        .clone();
                    match runtime_handle.block_on(crate::session::ui_store::with_tenant_scope(
                        tenant.clone(),
                        instance::provide_context(
                            project_context.clone(),
                            agent_session::summary::refresh_stream_diff_summary(
                                &stream_session_id,
                                &parts,
                            ),
                        ),
                    )) {
                        Ok(summary) => summary_update = summary,
                        Err(error) => tracing::warn!(
                            session_id = %stream_session_id, %error,
                            "failed to update running session diff summary"
                        ),
                    }
                }
                let payload = match ev {
                    agent_session::processor::StreamEvent::Done(usage) => {
                        let processing_completed_ms = agent_session::session::now_ms();
                        let persisted_turn = if should_persist_app_session_artifacts
                            && !(resume_history_only && query.trim().is_empty())
                        {
                            let stream_message_ids = stream_message_ids.clone();
                            let mut patch_parts = patch_parts;
                            if let Err(error) = trajectory_recorder
                                .append_tool_parts(&stream_session_id, &mut patch_parts)
                            {
                                let mut sink = sink_for_run
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                                sink.sent_terminal = (sink.send)(
                                    serde_json::json!({"type": "chat.error", "error": error.to_string()}),
                                    true,
                                );
                                break;
                            }
                            let native_text = if trajectory_recorder.tool_results.is_empty() {
                                &assistant_text
                            } else {
                                &trajectory_recorder.response_text
                            };
                            let persist = persist_stream_chat_turn_for_agent(
                                &stream_session_id,
                                &query,
                                native_text,
                                assistant_model.as_deref(),
                                &usage,
                                assistant_finish_reason.as_deref(),
                                stream_message_ids.as_ref(),
                                &patch_parts,
                                &message_agent,
                                None,
                                Some(agent_session::message::AssistantTime {
                                    created: processing_started_ms,
                                    completed: Some(processing_completed_ms),
                                    processing_elapsed_ms: Some(
                                        processing_timer.elapsed_ms_at(timer_now),
                                    ),
                                    llm_elapsed_ms: Some(llm_timer.elapsed_ms_at(timer_now)),
                                }),
                            );
                            match runtime_handle.block_on(
                                crate::session::ui_store::with_tenant_scope(
                                    tenant.clone(),
                                    instance::provide_context(project_context.clone(), persist),
                                ),
                            ) {
                                Ok((assistant_id, user_id, summary)) => {
                                    summary_update = summary;
                                    let ids = stream_message_ids
                                        .as_ref()
                                        .expect("persistent streams preallocate message ids");
                                    let eligible = !resume_history_only
                                        && !query.trim().is_empty()
                                        && publication_cancellation
                                            .as_ref()
                                            .is_none_or(|token| !token.is_cancelled())
                                        && trajectory_recorder.final_step_has_text
                                        && trajectory_recorder.steps.iter().all(|step| {
                                            step.tool_timings
                                                .iter()
                                                .all(|tool| tool.finished_ms.is_some())
                                        })
                                        && is_successful_finish_reason(
                                            assistant_finish_reason.as_deref(),
                                        );
                                    match commit_stream_session_trajectory(
                                        &tenant,
                                        app_session_scope.as_deref(),
                                        &stream_session_id,
                                        &query,
                                        &assistant_text,
                                        assistant_model.as_deref(),
                                        ids,
                                        &trajectory_recorder,
                                        processing_started_ms,
                                        processing_completed_ms,
                                        eligible.then_some(preview.as_str()),
                                    ) {
                                        Ok(first_preview) => {
                                            if let Some(preview) = first_preview {
                                                let title_model = if preserve_structured_prompt {
                                                    None
                                                } else {
                                                    assistant_model.clone()
                                                };
                                                agent_session::title::spawn_first_response_title(
                                                    &runtime_handle,
                                                    tenant.clone(),
                                                    project_context.clone(),
                                                    stream_session_id.clone(),
                                                    preview,
                                                    title_model,
                                                );
                                            }
                                            Ok((Some(assistant_id), Some(user_id)))
                                        }
                                        Err(error) => {
                                            tracing::error!(
                                                target: "vw_agent",
                                                session_id = %stream_session_id,
                                                error = %error,
                                                "failed to commit streamed session trajectory"
                                            );
                                            Err(error.to_string())
                                        }
                                    }
                                }
                                Err(error) => {
                                    tracing::error!(
                                        target: "vw_agent",
                                        session_id = %stream_session_id,
                                        error = %error.message,
                                        "failed to persist streamed chat turn"
                                    );
                                    Err(error.message)
                                }
                            }
                        } else {
                            Ok((None, None))
                        };
                        match persisted_turn {
                            Ok((message_id, parent_message_id)) => serde_json::json!({
                                "type": "chat.done",
                                "session_id": stream_session_id,
                                "usage": usage,
                                "message_id": message_id,
                                "parent_message_id": parent_message_id
                            }),
                            Err(error) => {
                                serde_json::json!({ "type": "chat.error", "session_id": stream_session_id, "error": error })
                            }
                        }
                    }
                    agent_session::processor::StreamEvent::Error(error) => {
                        let processing_completed_ms = agent_session::session::now_ms();
                        if should_persist_app_session_artifacts
                            && !(resume_history_only && query.trim().is_empty())
                        {
                            let stream_message_ids = stream_message_ids.clone();
                            let assistant_text = if assistant_text.trim().is_empty() {
                                error.as_str()
                            } else {
                                assistant_text.as_str()
                            };
                            let assistant_error = agent_session::message::AssistantError::Unknown {
                                message: error.clone(),
                            };
                            let empty_usage = ui_models::TokenUsage {
                                input_tokens: 0,
                                output_tokens: 0,
                                cached_tokens: 0,
                                reasoning_tokens: 0,
                            };
                            let persist = persist_stream_chat_turn_for_agent(
                                &stream_session_id,
                                &query,
                                assistant_text,
                                assistant_model.as_deref(),
                                &empty_usage,
                                Some("error"),
                                stream_message_ids.as_ref(),
                                &patch_parts,
                                &message_agent,
                                Some(assistant_error),
                                Some(agent_session::message::AssistantTime {
                                    created: processing_started_ms,
                                    completed: Some(processing_completed_ms),
                                    processing_elapsed_ms: Some(
                                        processing_timer.elapsed_ms_at(timer_now),
                                    ),
                                    llm_elapsed_ms: Some(llm_timer.elapsed_ms_at(timer_now)),
                                }),
                            );
                            if let Ok((_, _, summary)) = runtime_handle.block_on(
                                crate::session::ui_store::with_tenant_scope(
                                    tenant.clone(),
                                    instance::provide_context(project_context.clone(), persist),
                                ),
                            ) {
                                summary_update = summary;
                            }
                        }
                        serde_json::json!({ "type": "chat.error", "session_id": stream_session_id, "error": error })
                    }
                    other => match stream_payload_for_non_done_event(
                        other,
                        &mut assistant_text,
                        &mut assistant_finish_reason,
                        &mut assistant_model,
                        &mut current_step_index,
                        &stream_session_id,
                    ) {
                        Some(payload) => payload,
                        None => continue,
                    },
                };
                let mut sink =
                    sink_for_run.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Some(summary) = summary_update {
                    let _ = (sink.send)(
                        serde_json::json!({
                            "type": "chat.session_updated",
                            "session_id": stream_session_id,
                            "session": { "id": stream_session_id, "summary": summary },
                        }),
                        false,
                    );
                }
                let accepted = (sink.send)(payload, terminal);
                if accepted && terminal {
                    sink.sent_terminal = true;
                }
                drop(sink);
                if !accepted {
                    cancel.interrupt();
                    break;
                }
                if terminal {
                    break;
                }
            }
        }));

        let mut sink = sink.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if !sink.sent_terminal {
            let error = match outcome {
                Ok(()) => "gateway chat stream 异常结束：未发出终态事件".to_string(),
                Err(panic) => format!(
                    "gateway chat stream 内部异常: {}",
                    panic_payload_message(panic.as_ref())
                ),
            };
            let _ = (sink.send)(serde_json::json!({ "type": "chat.error", "error": error }), true);
        }
    });

    Ok(())
}

fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic".to_string()
    }
}

pub(crate) async fn collect_gateway_chat_stream(
    dir: String,
    body: GatewayChatStreamRequest,
) -> Result<GatewayChatStreamCollected, ApiError> {
    collect_gateway_chat_stream_with_observer(dir, body, |_| {}).await
}

pub(crate) async fn collect_gateway_chat_stream_with_observer(
    dir: String,
    body: GatewayChatStreamRequest,
    observe_payload: impl FnMut(&Value),
) -> Result<GatewayChatStreamCollected, ApiError> {
    collect_gateway_chat_stream_for_turn(dir, body, None, observe_payload).await
}

pub(crate) async fn collect_background_gateway_chat_stream(
    dir: String,
    body: GatewayChatStreamRequest,
    acp_full_access: bool,
    observe_payload: impl FnMut(&Value),
) -> Result<GatewayChatStreamCollected, ApiError> {
    collect_gateway_chat_stream_for_turn(dir, body, Some(acp_full_access), observe_payload).await
}

async fn collect_gateway_chat_stream_for_turn(
    dir: String,
    body: GatewayChatStreamRequest,
    background_full_access: Option<bool>,
    observe_payload: impl FnMut(&Value),
) -> Result<GatewayChatStreamCollected, ApiError> {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let cancellation = tokio_util::sync::CancellationToken::new();
    // AgentTool can drop the collector on stop while the gateway producer is still active.
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let rx = with_tenant_instance(tenant.clone(), dir.clone(), move || {
        Box::pin(async move {
            start_gateway_chat_stream(tenant, dir, body, background_full_access, Some(cancellation))
                .await
        })
    })
    .await?;
    collect_gateway_chat_stream_payloads(rx, observe_payload).await
}

async fn collect_gateway_chat_stream_payloads(
    mut rx: UnboundedReceiver<Value>,
    mut observe_payload: impl FnMut(&Value),
) -> Result<GatewayChatStreamCollected, ApiError> {
    let mut output = String::new();

    while let Some(payload) = rx.recv().await {
        observe_payload(&payload);
        match payload.get("type").and_then(Value::as_str) {
            Some("chat.delta") => {
                if let Some(delta) = payload.get("delta").and_then(Value::as_str) {
                    output.push_str(delta);
                }
            }
            Some("chat.done") => {
                let message_id =
                    payload.get("message_id").and_then(Value::as_str).map(str::to_string);
                let parent_message_id =
                    payload.get("parent_message_id").and_then(Value::as_str).map(str::to_string);
                return Ok(GatewayChatStreamCollected { output, message_id, parent_message_id });
            }
            Some("chat.error") => {
                let error = payload
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("gateway chat stream failed");
                return Err(ApiError::internal(error.to_string()));
            }
            _ => {}
        }
    }

    Err(ApiError::internal("gateway chat stream closed before completion"))
}

/// 处理桌面聊天流式请求。
///
/// # 参数
///
/// * `query` - 实例目录查询。
/// * `headers` - 可携带实例目录 header。
/// * `body` - 聊天历史、模型、agent 与 options。
///
/// # 返回值
///
/// 返回 `text/event-stream` 响应，事件类型包括 delta、step_start、step_finish、
/// post_tool_round、done 和 error。
///
/// # 错误处理
///
/// 实例解析、agent 解析或 processor 初始化失败时返回 [`ApiError`]；流内错误会编码为
/// `chat.error` 事件发送给前端。
pub(super) async fn chat_stream(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(mut body): Json<GatewayChatStreamRequest>,
) -> Result<impl IntoResponse, ApiError> {
    validate_public_acp_chat_options(body.options.as_ref())?;
    // Public background requests can only opt into the restricted ACP path.
    let background_full_access = (body
        .options
        .as_ref()
        .and_then(|options| options.get("acp_background"))
        .and_then(Value::as_bool)
        == Some(true))
    .then_some(false);
    let tenant = crate::session::ui_store::current_tenant_scope();
    let dir = resolve_directory(&query, &headers);
    #[cfg(not(target_arch = "wasm32"))]
    let dir =
        tokio::fs::canonicalize(&dir).await.map_or(dir, |path| path.to_string_lossy().into_owned());
    let request_id = stream_request_id(&headers)?;
    let after_event_id = last_event_id(&headers)?;
    let resume_after = headers.contains_key("last-event-id").then_some(after_event_id);
    if resume_after.is_some() && !headers.contains_key(STREAM_REQUEST_ID_HEADER) {
        return Err(ApiError::bad_request("Last-Event-ID requires x-vw-stream-request-id"));
    }
    let tenant_for_stream = tenant.clone();
    let sse = with_tenant_instance(tenant, dir.clone(), move || {
        Box::pin(async move {
            let project = instance::project()
                .ok_or_else(|| ApiError::internal("chat stream is missing project context"))?;
            let serialized = serde_json::to_vec(&(&project.id, &dir, &body))
                .map_err(|error| ApiError::internal(format!("chat stream 请求签名失败: {error}")))?;
            let signature = request_signature(&serialized);
            let session_id = body.session_id.as_ref().map(|id| id.0.trim())
                .filter(|id| !id.is_empty()).map(str::to_owned);
            let goal_evaluation = body.options.as_ref().and_then(|options| {
                options.get("goal_evaluation").and_then(Value::as_bool)
            }) == Some(true);
            let session_id = match session_id {
                None if !goal_evaluation => Some(id::ascending(id::Prefix::Session, None)
                    .map_err(|error| ApiError::internal(error.to_string()))?),
                session_id => session_id,
            };
            if let Some(session_id) = &session_id {
                axum::http::HeaderValue::from_str(session_id)
                    .map_err(|_| ApiError::bad_request("invalid session id"))?;
            }
            let mut opened = global_stream_store()
                .open_chat(
                    tenant_for_stream.clone(),
                    request_id.clone(),
                    signature,
                    resume_after,
                    session_id,
                )
                .map_err(|error| match error {
                    StreamOpenError::RequestConflict => ApiError {
                        status: StatusCode::CONFLICT,
                        message: "相同流请求标识对应了不同 chat 请求".to_string(),
                    },
                    StreamOpenError::ResumeUnavailable => ApiError {
                        status: StatusCode::GONE,
                        message: "chat replay is unavailable; read the original session before sending again".to_string(),
                    },
                })?;
            body.session_id = opened.chat_session_id.clone().map(Into::into);
            if let Some(producer) = opened.producer.take() {
                let cancellation = tokio_util::sync::CancellationToken::new();
                let cancellation_for_stop = cancellation.clone();
                let prompt_cancel: SharedPromptCancel = Arc::new(Mutex::new(None));
                let handle_for_stop = prompt_cancel.clone();
                producer.install_cancellation(move || {
                    cancellation_for_stop.cancel();
                    let handle = handle_for_stop
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .clone();
                    if let Some(handle) = handle {
                        handle.cancel();
                    }
                });
                start_gateway_chat_producer(
                    tenant_for_stream,
                    dir,
                    body,
                    background_full_access,
                    Some(cancellation),
                    Some(prompt_cancel),
                    move |payload, terminal| producer.publish(payload.to_string(), terminal),
                )
                .await?;
            }
            let mut response = sse_response(opened.subscription);
            response.headers_mut().insert(
                STREAM_REQUEST_ID_HEADER,
                axum::http::HeaderValue::from_str(&request_id)
                    .map_err(|_| ApiError::bad_request("invalid stream request id"))?,
            );
            if let Some(session_id) = opened.chat_session_id {
                response.headers_mut().insert(
                    "x-vw-session-id",
                    axum::http::HeaderValue::from_str(&session_id)
                        .map_err(|_| ApiError::internal("invalid replay session id"))?,
                );
            }
            Ok(response)
        })
    })
    .await?;

    Ok(sse)
}

pub(super) async fn chat_stream_stop(
    Extension(tenant): Extension<TenantScope>,
    axum::extract::Path(request_id): axum::extract::Path<String>,
) -> Json<Value> {
    let cancelled = global_stream_store().cancel(tenant, StreamKind::Chat, &request_id);
    Json(serde_json::json!({ "cancelled": cancelled }))
}

/// 拆分流式请求中的模型引用。
///
/// # 参数
///
/// * `model` - 可选模型引用，通常形如 `provider/model`。
///
/// # 返回值
///
/// 返回 session 消息可保存的 provider/model 结构；空值返回空引用。
pub(super) fn split_stream_model_ref(model: Option<&str>) -> agent_session::message::ModelRef {
    let Some(model) = model.map(str::trim).filter(|value| !value.is_empty()) else {
        return agent_session::message::ModelRef {
            provider_id: String::new(),
            model_id: String::new(),
        };
    };

    let parsed = provider::parse_model(model);
    if parsed.model_id.is_empty() {
        agent_session::message::ModelRef {
            provider_id: String::new(),
            model_id: parsed.provider_id,
        }
    } else {
        agent_session::message::ModelRef {
            provider_id: parsed.provider_id,
            model_id: parsed.model_id,
        }
    }
}

fn token_info_from_usage(usage: &ui_models::TokenUsage) -> agent_session::message::TokenInfo {
    agent_session::message::TokenInfo {
        total: Some(usage.input_tokens + usage.output_tokens + usage.cached_tokens),
        input: usage.input_tokens,
        output: usage.output_tokens,
        reasoning: usage.reasoning_tokens,
        cache: agent_session::message::TokenCacheInfo { read: usage.cached_tokens, write: 0 },
    }
}

/// 持久化一轮流式聊天的 user/assistant 消息。
///
/// # 参数
///
/// * `session_id` - 目标会话 id。
/// * `query` - 本轮用户输入。
/// * `assistant_text` - assistant 最终文本。
/// * `model` - 可选模型引用。
/// * `usage` - token 使用量。
/// * `finish_reason` - 可选完成原因。
/// * `preallocated_ids` - 可选预分配消息 id。
/// * `extra_assistant_parts` - 本轮附加到 assistant 消息的回滚追踪片段。
///
/// # 返回值
///
/// 返回 `(assistant_id, user_id)`，用于 done 事件回传。
///
/// # 错误处理
///
/// id 生成或消息/片段写入失败时返回 [`ApiError`]。
pub(super) async fn persist_stream_chat_turn(
    session_id: &str,
    query: &str,
    assistant_text: &str,
    model: Option<&str>,
    usage: &ui_models::TokenUsage,
    finish_reason: Option<&str>,
    preallocated_ids: Option<&StreamTurnMessageIds>,
    extra_assistant_parts: &[agent_session::message::Part],
) -> Result<(String, String), ApiError> {
    persist_stream_chat_turn_for_agent(
        session_id,
        query,
        assistant_text,
        model,
        usage,
        finish_reason,
        preallocated_ids,
        extra_assistant_parts,
        "gateway",
        None,
        None,
    )
    .await
    .map(|(assistant_id, user_id, _)| (assistant_id, user_id))
}

pub(crate) async fn persist_child_agent_turn(
    session_id: &str,
    query: &str,
    assistant_text: &str,
    agent_key: &str,
    error: Option<&str>,
) -> Result<(String, String), ApiError> {
    let usage = ui_models::TokenUsage {
        input_tokens: 0,
        output_tokens: 0,
        cached_tokens: 0,
        reasoning_tokens: 0,
    };
    let assistant_error = error.map(|message| agent_session::message::AssistantError::Unknown {
        message: message.to_string(),
    });
    persist_stream_chat_turn_for_agent(
        session_id,
        query,
        assistant_text,
        None,
        &usage,
        error.map(|_| "error"),
        None,
        &[],
        agent_key,
        assistant_error,
        None,
    )
    .await
    .map(|(assistant_id, user_id, _)| (assistant_id, user_id))
}

/// Commit a turn returned by an executor that does not write a gateway trajectory.
pub(crate) fn commit_collected_child_agent_turn(
    session_id: &str,
    prompt: &str,
    output: &str,
    (assistant_id, user_id): (String, String),
) -> anyhow::Result<()> {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let project = instance::project().ok_or_else(|| anyhow::anyhow!("child project is missing"))?;
    let recorder = StreamTrajectoryRecorder::new(assistant_id.clone());
    let ids = StreamTurnMessageIds::new(assistant_id, user_id);
    let now = agent_session::session::now_ms();
    let preview = agent_session::title::first_response_source(prompt);
    commit_stream_session_trajectory(
        &tenant,
        Some(&project.id),
        session_id,
        prompt,
        output,
        None,
        &ids,
        &recorder,
        now,
        now,
        Some(&preview),
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn persist_stream_chat_turn_for_agent(
    session_id: &str,
    query: &str,
    assistant_text: &str,
    model: Option<&str>,
    usage: &ui_models::TokenUsage,
    finish_reason: Option<&str>,
    preallocated_ids: Option<&StreamTurnMessageIds>,
    extra_assistant_parts: &[agent_session::message::Part],
    message_agent: &str,
    assistant_error: Option<agent_session::message::AssistantError>,
    processing_time: Option<agent_session::message::AssistantTime>,
) -> Result<(String, String, Option<agent_session::session::Summary>), ApiError> {
    let model_ref = split_stream_model_ref(model);
    let now = agent_session::session::now_ms();
    let assistant_time = processing_time.unwrap_or(agent_session::message::AssistantTime {
        created: now,
        completed: Some(now),
        processing_elapsed_ms: None,
        llm_elapsed_ms: None,
    });
    let request_started_ms = assistant_time.created;

    let user_id = match preallocated_ids {
        Some(ids) => ids.user_id.clone(),
        None => id::ascending(id::Prefix::Message, None)
            .map_err(|err| ApiError::internal(err.to_string()))?,
    };
    let user_part_id =
        id::ascending(id::Prefix::Part, None).map_err(|err| ApiError::internal(err.to_string()))?;
    let user_info =
        agent_session::message::Info::User(Box::new(agent_session::message::UserInfo {
            id: user_id.clone(),
            session_id: session_id.to_string(),
            time: agent_session::message::UserTime { created: request_started_ms },
            summary: None,
            agent: message_agent.to_string(),
            model: model_ref.clone(),
            system: None,
            tools: None,
            variant: None,
        }));
    let user_part = agent_session::message::Part::Text(agent_session::message::TextPart {
        base: agent_session::message::PartBase {
            id: user_part_id,
            session_id: session_id.to_string(),
            message_id: user_id.clone(),
        },
        text: query.to_string(),
        synthetic: None,
        ignored: None,
        time: Some(agent_session::message::PartTime {
            start: request_started_ms,
            end: Some(request_started_ms),
        }),
        metadata: None,
    });

    let assistant_id = match preallocated_ids {
        Some(ids) => ids.assistant_id.clone(),
        None => id::ascending(id::Prefix::Message, None)
            .map_err(|err| ApiError::internal(err.to_string()))?,
    };
    let assistant_part_id =
        id::ascending(id::Prefix::Part, None).map_err(|err| ApiError::internal(err.to_string()))?;
    let assistant_info =
        agent_session::message::Info::Assistant(Box::new(agent_session::message::AssistantInfo {
            id: assistant_id.clone(),
            session_id: session_id.to_string(),
            time: assistant_time,
            error: assistant_error,
            parent_id: user_id.clone(),
            model_id: model_ref.model_id,
            provider_id: model_ref.provider_id,
            mode: "chat".to_string(),
            agent: message_agent.to_string(),
            path: agent_session::message::PathInfo {
                cwd: instance::directory(),
                root: instance::worktree(),
            },
            summary: None,
            cost: 0.0,
            tokens: token_info_from_usage(usage),
            variant: None,
            finish: finish_reason.map(str::to_string),
        }));
    let assistant_part = agent_session::message::Part::Text(agent_session::message::TextPart {
        base: agent_session::message::PartBase {
            id: assistant_part_id,
            session_id: session_id.to_string(),
            message_id: assistant_id.clone(),
        },
        text: assistant_text.to_string(),
        synthetic: None,
        ignored: None,
        time: Some(agent_session::message::PartTime { start: now, end: Some(now) }),
        metadata: None,
    });

    let _history = agent_session::message::lock_history(session_id).await;
    agent_session::message::update_message(&user_info)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;
    agent_session::message::update_part(&user_part)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;
    agent_session::message::update_message(&assistant_info)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;
    agent_session::message::update_part(&assistant_part)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;
    for part in extra_assistant_parts {
        agent_session::message::update_part(part)
            .await
            .map_err(|err| ApiError::internal(err.to_string()))?;
    }
    let summary = if !extra_assistant_parts
        .iter()
        .any(|part| !matches!(part, agent_session::message::Part::Tool(_)))
    {
        None
    } else {
        match agent_session::summary::refresh_session_diff_summary(session_id).await {
            Ok(summary) => Some(summary),
            Err(error) => {
                tracing::warn!(session_id, %error, "failed to persist session diff summary");
                None
            }
        }
    };

    Ok((assistant_id, user_id, summary))
}

fn preallocate_stream_turn_message_ids() -> Result<StreamTurnMessageIds, ApiError> {
    let assistant_id = id::ascending(id::Prefix::Message, None)
        .map_err(|err| ApiError::internal(err.to_string()))?;
    let user_id = id::ascending(id::Prefix::Message, None)
        .map_err(|err| ApiError::internal(err.to_string()))?;
    Ok(StreamTurnMessageIds { assistant_id, user_id })
}

fn upsert_stream_ui_message(
    session: &mut ui_models::ChatSession,
    message_id: &str,
    parent_message_id: Option<&str>,
    message: ui_models::ChatMessage,
) {
    if session.message_ids.len() < session.messages.len() {
        session.message_ids.resize(session.messages.len(), None);
    }
    if let Some(position) =
        session.message_ids.iter().position(|existing| existing.as_deref() == Some(message_id))
    {
        session.messages[position] = message;
        return;
    }
    // Desktop snapshots can contain tool markers or only a partial response, so
    // content equality cannot identify the pending assistant. Bind it to this
    // turn's user message, without consuming entries from another turn.
    let pending_position = if let Some(parent_id) = parent_message_id {
        session.message_ids.iter().position(|id| id.as_deref() == Some(parent_id)).and_then(
            |parent_position| {
                let mut candidates = session
                    .messages
                    .iter()
                    .enumerate()
                    .skip(parent_position + 1)
                    .take_while(|(_, existing)| existing.role != ui_models::ChatRole::User)
                    .filter(|(position, existing)| {
                        session.message_ids.get(*position).is_none_or(Option::is_none)
                            && existing.role == message.role
                    })
                    .map(|(position, _)| position);
                let position = candidates.next()?;
                candidates.next().is_none().then_some(position)
            },
        )
    } else {
        session
            .messages
            .iter()
            .enumerate()
            .rev()
            .find(|(position, existing)| {
                session.message_ids.get(*position).is_none_or(Option::is_none)
                    && existing.role == message.role
                    && existing.content == message.content
            })
            .map(|(position, _)| position)
    };
    if let Some(position) = pending_position {
        session.messages[position] = message;
        session.message_ids[position] = Some(message_id.to_string());
        return;
    }
    session.messages.push(message);
    session.message_ids.push(Some(message_id.to_string()));
}

#[allow(clippy::too_many_arguments)]
fn commit_stream_session_trajectory(
    tenant: &TenantScope,
    scope: Option<&str>,
    session_id: &str,
    query: &str,
    assistant_text: &str,
    model: Option<&str>,
    ids: &StreamTurnMessageIds,
    recorder: &StreamTrajectoryRecorder,
    created_ms: u64,
    completed_ms: u64,
    first_response_preview: Option<&str>,
) -> anyhow::Result<Option<String>> {
    let finished_steps = recorder
        .steps
        .iter()
        .filter(|step| {
            step.finished_ms.is_some()
                && !(step.model.is_none() && step.finish_reason.as_deref() == Some("workflow"))
        })
        .map(|step| (recorder.turn_id.clone(), step.index))
        .collect::<Vec<_>>();
    let claims = raw_commit::claim(tenant, scope, session_id, &finished_steps)?;
    if claims.expected_steps().len() != finished_steps.len() {
        anyhow::bail!(
            "stream trajectory is missing raw LLM artifacts: expected {}, found {}",
            finished_steps.len(),
            claims.expected_steps().len()
        );
    }

    let mut session = crate::session::ui_store::load_session_scoped_for_tenant_fallible(
        tenant, session_id, scope,
    )?
    .unwrap_or_else(|| ui_models::ChatSession {
        id: session_id.to_string(),
        title: String::new(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms,
        updated_ms: completed_ms,
    });

    upsert_stream_ui_message(
        &mut session,
        &ids.user_id,
        None,
        ui_models::ChatMessage {
            role: ui_models::ChatRole::User,
            content: query.to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    );
    upsert_stream_ui_message(
        &mut session,
        &ids.assistant_id,
        Some(&ids.user_id),
        ui_models::ChatMessage {
            role: ui_models::ChatRole::Assistant,
            content: assistant_text.to_string(),
            model: model.map(str::to_string),
            think_timing: Vec::new(),
            processing_timing: Some(ui_models::ProcessingTiming {
                started_ms: created_ms,
                completed_ms: Some(completed_ms),
                processing_elapsed_ms: Some(completed_ms.saturating_sub(created_ms)),
                llm_elapsed_ms: None,
            }),
        },
    );
    for step in &recorder.steps {
        session.steps.retain(|existing| {
            !(existing.index == step.index
                && (existing.turn_id.is_none()
                    || existing.turn_id.as_deref() == Some(recorder.turn_id.as_str())))
        });
        session.steps.push(step.clone());
    }
    session.steps.sort_by_key(|step| (step.started_ms, step.index));
    if session.created_ms == 0 {
        session.created_ms = created_ms;
    } else {
        session.created_ms = session.created_ms.min(created_ms);
    }
    session.updated_ms = session.updated_ms.max(completed_ms);

    let project_id =
        scope.ok_or_else(|| anyhow::anyhow!("stream session is missing project scope"))?;
    let mut info =
        crate::session::ui_store::load_agent_session_scoped_for_tenant(tenant, session_id, scope)
            .ok_or_else(|| anyhow::anyhow!("stream session reservation is missing"))?;
    if info.is_first_response_pending()
        && let Some(preview) = first_response_preview
        && (!info.is_agent_tool()
            || !agent_session::child_agent::child_agent_handoff_from_output(assistant_text)
                .is_empty())
    {
        let task_title = if info.is_visible_task() {
            crate::session::ui_store::load_visible_task_states(tenant, project_id)?
                .remove(session_id)
                .filter(|task| Some(task.parent_session_id.as_str()) == info.parent_id.as_deref())
                .and_then(|task| task.title)
        } else if info.is_agent_tool() && !preview.is_empty() {
            Some(agent_session::child_agent::task_summary(preview))
        } else {
            None
        };
        let published = crate::session::ui_store::commit_first_response(
            tenant,
            project_id,
            crate::session::store::FirstResponseCommit {
                snapshot: &session,
                raw_artifacts: claims.raw_artifacts(),
                expected_finished_steps: claims.expected_steps(),
                preview: Some(preview),
                task_title: task_title.as_deref(),
            },
        )?;
        if !published {
            anyhow::bail!("session changed while committing its first response");
        }
        raw_commit::acknowledge(&claims);
        return Ok((!preview.is_empty() && task_title.is_none()).then(|| preview.to_owned()));
    }

    info.time.updated = info.time.updated.max(session.updated_ms);
    session.title.clone_from(&info.title);
    crate::session::ui_store::save_agent_session_scoped_for_tenant(tenant, &info, scope)
        .ok_or_else(|| anyhow::anyhow!("failed to update stream session metadata"))?;
    if claims.is_empty() {
        crate::session::ui_store::save_session_ui_scoped_for_tenant(tenant, &session, scope, true)?;
    } else {
        crate::session::ui_store::commit_session_ui_snapshot_scoped_for_tenant(
            tenant,
            &session,
            scope,
            claims.raw_artifacts(),
            claims.expected_steps(),
        )?;
    }
    raw_commit::acknowledge(&claims);
    Ok(None)
}

pub(super) fn reserve_stream_agent_session(
    tenant: &TenantScope,
    project_id: &str,
    session_id: &str,
    directory: &str,
    created_ms: u64,
) -> anyhow::Result<()> {
    let info = agent_session::session::Info {
        id: session_id.to_string(),
        slug: agent_session::session::create_slug(),
        project_id: project_id.to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: String::new(),
        version: crate::app::agent::installation::version(),
        time: agent_session::session::TimeInfo {
            created: created_ms,
            updated: created_ms,
            compacting: None,
            archived: None,
        },
        permission: None,
        fork_execution_started: None,
        revert: None,
    };
    crate::session::ui_store::reserve_pending_session(tenant, &info)?;
    Ok(())
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod stream_tests;

#[cfg(test)]
pub(crate) use stream_tests::collect_video_edit_test_stream;

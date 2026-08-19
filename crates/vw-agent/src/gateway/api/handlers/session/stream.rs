//! 桌面聊天流式接口。
//!
//! 本模块把 gateway chat 请求转换为 session processor 请求，并把处理器事件编码为
//! SSE 帧返回桌面端。它还负责合并 agent/model/options 覆盖项、把 tool 消息转换为
//! session 可理解的文本，以及在流结束后补写桌面会话消息。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::{Extension, Json};
use axum::extract::Query;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::{Map, Value};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use vw_api_types::chat::{GatewayChatStreamRequest, ReasoningEffortDto};
use vw_api_types::id::TenantScope;

use super::raw_commit;
use super::shared::with_tenant_instance;
use crate::app::agent::gateway::api::handlers::resumable_stream::{
    StreamKind, StreamOpenError, global_stream_store, last_event_id, request_signature,
    sse_response, stream_request_id,
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
}

impl StreamTrajectoryRecorder {
    fn new(turn_id: String) -> Self {
        Self { turn_id, steps: Vec::new() }
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
                let step = self.step_mut(*step_index, *created_ms);
                step.started_ms = *created_ms;
                if model.is_some() {
                    step.model.clone_from(model);
                }
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
                        timing.finished_ms.get_or_insert(event.occurred_ms);
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
}

fn event_step_index(steps: &[ui_models::ChatSessionStep]) -> u32 {
    steps.last().map(|step| step.index).unwrap_or_default()
}

#[derive(Debug, Default)]
struct StreamPatchRecorder {
    current_step_index: Option<u32>,
    active_step: Option<StreamStepSnapshot>,
    parts: Vec<agent_session::message::Part>,
}

impl StreamPatchRecorder {
    fn note_step_start(&mut self, step_index: u32) {
        self.current_step_index = Some(step_index);
    }

    fn begin_step(&mut self, session_id: &str, assistant_id: &str, root: &str, step_index: u32) {
        self.note_step_start(step_index);
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
    if acp_requested && let Some(reasoning_effort) = reasoning_effort {
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
) -> PreparedStreamHistory {
    let total_messages = history_messages.len();
    let mut history = Vec::new();
    let mut query = String::new();

    for (idx, message) in history_messages.into_iter().enumerate() {
        let role = message.get("role").and_then(Value::as_str).unwrap_or("user");
        let content =
            message.get("content").and_then(Value::as_str).unwrap_or_default().to_string();

        let normalized_content =
            if role == "tool" { tool_content_to_session_text(&content) } else { content };

        if idx + 1 == total_messages && role == "user" {
            query = normalized_content;
            continue;
        }

        let mapped_role = match role {
            "assistant" => ui_models::ChatRole::Assistant,
            "system" => ui_models::ChatRole::System,
            "tool" => ui_models::ChatRole::Tool,
            _ => ui_models::ChatRole::User,
        };

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
    }

    PreparedStreamHistory { history, query }
}

fn stream_payload_for_non_done_event(
    ev: agent_session::processor::StreamEvent,
    assistant_text: &mut String,
    assistant_finish_reason: &mut Option<String>,
    assistant_model: &mut Option<String>,
    patch_recorder: &mut StreamPatchRecorder,
    stream_session_id: &str,
    stream_message_ids: Option<&StreamTurnMessageIds>,
    root_dir: &str,
) -> Option<Value> {
    match ev {
        agent_session::processor::StreamEvent::LlmRawArtifact(_) => None,
        agent_session::processor::StreamEvent::ContextLoaded { paths } => Some(serde_json::json!({
            "type": "chat.context_loaded",
            "paths": paths
        })),
        agent_session::processor::StreamEvent::Delta(delta) => {
            assistant_text.push_str(&delta);
            Some(serde_json::json!({ "type": "chat.delta", "delta": delta }))
        }
        agent_session::processor::StreamEvent::Tool(event) => {
            let delta = format!("tool {}\n{}\n", event.name, event.payload);
            assistant_text.push_str(&delta);
            let Some(step_index) = patch_recorder.current_step_index else {
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
        agent_session::processor::StreamEvent::StepStart { step_index, created_ms, model } => {
            patch_recorder.note_step_start(step_index);
            if let Some(assistant_id) =
                stream_message_ids.as_ref().map(|ids| ids.assistant_id.as_str())
            {
                patch_recorder.begin_step(stream_session_id, assistant_id, root_dir, step_index);
            }
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
            if let Some(assistant_id) =
                stream_message_ids.as_ref().map(|ids| ids.assistant_id.as_str())
            {
                patch_recorder.finish_tool_round(
                    stream_session_id,
                    assistant_id,
                    root_dir,
                    step_index,
                );
            }
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
        agent_session::runtime::PromptEvent::Text(text) => {
            agent_session::processor::StreamEvent::Delta(text)
        }
        agent_session::runtime::PromptEvent::Tool(event) => {
            agent_session::processor::StreamEvent::Tool(event)
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

async fn resolve_delegate_request_overrides(
    agent_name: Option<String>,
    allowed_tools: Option<Vec<String>>,
    explicit_model: Option<String>,
) -> Result<DelegateRequestOverrides, ApiError> {
    let agent_name = agent_name.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    });
    let explicit_allowed_tools = allowed_tools.and_then(normalize_tool_ids);

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

    let model = delegated_model_override(&config, explicit_model);
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
            chat_system_prompt,
            temperature: config.temperature,
            top_p: config.top_p,
            ..DelegateRequestOverrides::default()
        });
    }

    let mut registered_tools = explicit_allowed_tools
        .unwrap_or_else(|| normalize_tool_ids(config.allowed_tools).unwrap_or_default());
    registered_tools.retain(|tool| {
        !matches!(
            tool.trim().to_ascii_lowercase().as_str(),
            "agenttool" | "agent_tool" | "agent"
        )
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
        if let Some(default_allowed_skills) = default_allowed_skills {
            options.insert(
                "delegate_default_allowed_skills".to_string(),
                Value::Array(default_allowed_skills.into_iter().map(Value::String).collect()),
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
) -> Result<UnboundedReceiver<Value>, ApiError> {
    let (tx, rx) = mpsc::unbounded_channel::<Value>();
    start_gateway_chat_producer(tenant, dir, body, None, None, move |payload, _terminal| {
        tx.send(payload).is_ok()
    })
    .await?;
    Ok(rx)
}

async fn start_gateway_chat_producer(
    tenant: TenantScope,
    dir: String,
    body: GatewayChatStreamRequest,
    delegate_overrides: Option<DelegateRequestOverrides>,
    abort_token: Option<tokio_util::sync::CancellationToken>,
    send: impl FnMut(Value, bool) -> bool + Send + 'static,
) -> Result<(), ApiError> {
    let GatewayChatStreamRequest {
        session_id,
        messages,
        system: _,
        model,
        reasoning_effort,
        agent,
        allowed_tools,
        acp_agent,
        acp_allowed_tools,
        options,
    } = body;

    let delegate_overrides = match delegate_overrides {
        Some(delegate_overrides) => delegate_overrides,
        None => resolve_delegate_request_overrides(agent, allowed_tools, model).await?,
    };
    let model = delegate_overrides.model.clone();
    let message_agent = delegate_overrides.agent.clone().unwrap_or_else(|| "gateway".to_string());
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

    let history_messages = messages;
    let root_dir = dir;
    let goal_evaluation = options.get("goal_evaluation").and_then(Value::as_bool).unwrap_or(false);
    let stream_session_id =
        match session_id.as_ref().map(|value| value.0.trim()).filter(|value| !value.is_empty()) {
            Some(session_id) => session_id.to_string(),
            None if goal_evaluation => String::new(),
            None => id::ascending(id::Prefix::Session, None)
                .map_err(|error| ApiError::internal(error.to_string()))?,
        };
    let should_persist_app_session_artifacts = !goal_evaluation;
    let app_session_scope = if should_persist_app_session_artifacts || goal_evaluation {
        crate::app::agent::session::processor::app_session_scope_from_root_async(Some(
            root_dir.as_str(),
        ))
        .await
    } else {
        None
    };
    let stream_message_ids = if !should_persist_app_session_artifacts {
        None
    } else {
        Some(preallocate_stream_turn_message_ids()?)
    };
    let runtime_handle = tokio::runtime::Handle::current();

    std::thread::spawn(move || {
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
            let PreparedStreamHistory { history, query } =
                prepare_stream_history(history_messages, resume_history_only);
            let mut assistant_text = String::new();
            let mut assistant_finish_reason: Option<String> = None;
            let mut assistant_model = model.clone();
            let mut patch_recorder = StreamPatchRecorder::default();
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

            let turn = agent_session::runtime::start_prompt(agent_session::processor::Request {
                stream: 0,
                session: stream_session_id.clone(),
                query: query.clone(),
                root: Some(root_dir.clone()),
                model: model.clone(),
                reliability: reliability.clone(),
                options: options.clone(),
                approval: Some(approval_manager.clone()),
                channel_name: Some("desktop".to_string()),
                non_cli_approval_context: Some(crate::agent::loop_::NonCliApprovalContext {
                    sender: "desktop".to_string(),
                    reply_target: approval_reply_target,
                    prompt_tx: approval_prompt_tx,
                }),
                assistant_message_id: stream_message_ids
                    .as_ref()
                    .map(|ids| ids.assistant_id.clone()),
                history,
                persist_app_session_artifacts: should_persist_app_session_artifacts,
                app_session_scope: app_session_scope.clone(),
                abort_token,
                session_tools: crate::session::runtime::SessionToolset::default(),
            });
            let cancel = turn.cancel.clone();
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
                if terminal {
                    processing_timer.pause_at(timer_now);
                    llm_timer.pause_at(timer_now);
                }
                let payload = match ev {
                    agent_session::processor::StreamEvent::Done(usage) => {
                        let processing_completed_ms = agent_session::session::now_ms();
                        let persisted_turn = if should_persist_app_session_artifacts
                            && !(resume_history_only && query.trim().is_empty())
                        {
                            let stream_message_ids = stream_message_ids.clone();
                            let patch_parts = patch_recorder.take_parts();
                            match runtime_handle.block_on(
                                crate::session::ui_store::with_tenant_scope(
                                    tenant.clone(),
                                    persist_stream_chat_turn_for_agent(
                                        &stream_session_id,
                                        &query,
                                        &assistant_text,
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
                                            llm_elapsed_ms: Some(
                                                llm_timer.elapsed_ms_at(timer_now),
                                            ),
                                        }),
                                    ),
                                ),
                            ) {
                                Ok((assistant_id, user_id)) => {
                                    let ids = stream_message_ids
                                        .as_ref()
                                        .expect("persistent streams preallocate message ids");
                                    if let Err(error) = commit_stream_session_trajectory(
                                        &tenant,
                                        app_session_scope.as_deref(),
                                        &stream_session_id,
                                        &root_dir,
                                        &query,
                                        &assistant_text,
                                        assistant_model.as_deref(),
                                        ids,
                                        &trajectory_recorder,
                                        processing_started_ms,
                                        processing_completed_ms,
                                    ) {
                                        tracing::error!(
                                            target: "vw_agent",
                                            session_id = %stream_session_id,
                                            error = %error,
                                            "failed to commit streamed session trajectory"
                                        );
                                        Err(error.to_string())
                                    } else {
                                        Ok((Some(assistant_id), Some(user_id)))
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
                                serde_json::json!({ "type": "chat.error", "error": error })
                            }
                        }
                    }
                    agent_session::processor::StreamEvent::Error(error) => {
                        let processing_completed_ms = agent_session::session::now_ms();
                        if should_persist_app_session_artifacts
                            && !(resume_history_only && query.trim().is_empty())
                        {
                            let stream_message_ids = stream_message_ids.clone();
                            let patch_parts = patch_recorder.take_parts();
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
                            let _ = runtime_handle.block_on(
                                crate::session::ui_store::with_tenant_scope(
                                    tenant.clone(),
                                    persist_stream_chat_turn_for_agent(
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
                                            llm_elapsed_ms: Some(
                                                llm_timer.elapsed_ms_at(timer_now),
                                            ),
                                        }),
                                    ),
                                ),
                            );
                        }
                        serde_json::json!({ "type": "chat.error", "error": error })
                    }
                    other => match stream_payload_for_non_done_event(
                        other,
                        &mut assistant_text,
                        &mut assistant_finish_reason,
                        &mut assistant_model,
                        &mut patch_recorder,
                        &stream_session_id,
                        stream_message_ids.as_ref(),
                        &root_dir,
                    ) {
                        Some(payload) => payload,
                        None => continue,
                    },
                };
                let mut sink =
                    sink_for_run.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let accepted = (sink.send)(payload, terminal);
                if accepted && terminal {
                    sink.sent_terminal = true;
                }
                drop(sink);
                if !accepted {
                    cancel.cancel();
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
    let tenant = TenantScope::default();
    let rx = with_tenant_instance(tenant.clone(), dir.clone(), move || {
        Box::pin(async move { start_gateway_chat_stream(tenant, dir, body).await })
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
    Json(body): Json<GatewayChatStreamRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let dir = resolve_directory(&query, &headers);
    let request_id = stream_request_id(&headers)?;
    let after_event_id = last_event_id(&headers)?;
    let serialized_request = serde_json::to_vec(&body)
        .map_err(|error| ApiError::internal(format!("chat stream 请求签名失败: {error}")))?;
    let signature = request_signature(&serialized_request);
    let tenant_for_stream = tenant.clone();
    let sse = with_tenant_instance(tenant, dir.clone(), move || {
        Box::pin(async move {
            let delegate_overrides = resolve_delegate_request_overrides(
                body.agent.clone(),
                body.allowed_tools.clone(),
                body.model.clone(),
            )
            .await?;
            let mut opened = global_stream_store()
                .open(
                    tenant_for_stream.clone(),
                    StreamKind::Chat,
                    request_id,
                    signature,
                    after_event_id,
                )
                .map_err(|error| match error {
                    StreamOpenError::RequestConflict => ApiError {
                        status: StatusCode::CONFLICT,
                        message: "相同流请求标识对应了不同 chat 请求".to_string(),
                    },
                })?;
            if let Some(producer) = opened.producer.take() {
                let cancellation = tokio_util::sync::CancellationToken::new();
                let cancellation_for_stop = cancellation.clone();
                producer.install_cancellation(move || cancellation_for_stop.cancel());
                start_gateway_chat_producer(
                    tenant_for_stream,
                    dir,
                    body,
                    Some(delegate_overrides),
                    Some(cancellation),
                    move |payload, terminal| producer.publish(payload.to_string(), terminal),
                )
                .await?;
            }
            Ok(sse_response(opened.subscription))
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
) -> Result<(String, String), ApiError> {
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
    if !extra_assistant_parts.is_empty() {
        let _ = agent_session::summary::refresh_session_diff_summary(session_id).await;
    }

    Ok((assistant_id, user_id))
}

fn preallocate_stream_turn_message_ids() -> Result<StreamTurnMessageIds, ApiError> {
    let assistant_id = id::ascending(id::Prefix::Message, None)
        .map_err(|err| ApiError::internal(err.to_string()))?;
    let user_id = id::ascending(id::Prefix::Message, None)
        .map_err(|err| ApiError::internal(err.to_string()))?;
    Ok(StreamTurnMessageIds { assistant_id, user_id })
}

fn initial_stream_session_title(query: &str) -> String {
    query
        .trim()
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .unwrap_or("新会话")
        .to_string()
}

fn upsert_stream_ui_message(
    session: &mut ui_models::ChatSession,
    message_id: &str,
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
    if let Some(position) = session
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
    {
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
    directory: &str,
    query: &str,
    assistant_text: &str,
    model: Option<&str>,
    ids: &StreamTurnMessageIds,
    recorder: &StreamTrajectoryRecorder,
    created_ms: u64,
    completed_ms: u64,
) -> anyhow::Result<()> {
    let finished_steps = recorder
        .steps
        .iter()
        .filter(|step| step.finished_ms.is_some())
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
        title: initial_stream_session_title(query),
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

    ensure_stream_agent_session(
        tenant,
        scope,
        session_id,
        directory,
        &session.title,
        session.created_ms,
        session.updated_ms,
    )?;

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
    Ok(())
}

fn ensure_stream_agent_session(
    tenant: &TenantScope,
    scope: Option<&str>,
    session_id: &str,
    directory: &str,
    title: &str,
    created_ms: u64,
    updated_ms: u64,
) -> anyhow::Result<()> {
    let project_id = scope
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("stream session is missing project scope"))?;
    let existing = crate::session::ui_store::load_agent_session_scoped_for_tenant(
        tenant,
        session_id,
        Some(project_id),
    );
    let info = if let Some(mut info) = existing {
        info.time.created = info.time.created.min(created_ms);
        info.time.updated = info.time.updated.max(updated_ms);
        info
    } else {
        agent_session::session::Info {
            id: session_id.to_string(),
            slug: agent_session::session::create_slug(),
            project_id: project_id.to_string(),
            directory: directory.to_string(),
            parent_id: None,
            summary: None,
            share: None,
            title: title.to_string(),
            version: crate::app::agent::installation::version(),
            time: agent_session::session::TimeInfo {
                created: created_ms,
                updated: updated_ms,
                compacting: None,
                archived: None,
            },
            permission: None,
            revert: None,
        }
    };
    crate::session::ui_store::save_agent_session_scoped_for_tenant(tenant, &info, Some(project_id))
        .ok_or_else(|| anyhow::anyhow!("failed to persist stream session metadata"))?;
    Ok(())
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod stream_tests;

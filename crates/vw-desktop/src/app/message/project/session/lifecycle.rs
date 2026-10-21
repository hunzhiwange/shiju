//! 处理项目会话生命周期，负责加载历史、重置视图和切换活动会话。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use super::common::save_config_field_task;
use crate::app::message::project::PreparedSessionOpenSource;
use crate::app::message::project::ProjectMessage;
use crate::app::message::project::helpers::{
    find_session_project_path, load_prepared_session_open_from_gateway_task, now_ms,
    prepare_session_ui_chunks_task, prepare_session_ui_task,
};
use crate::app::{App, Message, models, state::ActiveSessionViewState};
use serde_json::{Map, Value};
use vw_shared::message::types as agent_message;
use web_time::Instant;

/// 幂等登记外部创建的会话，不触碰当前活动会话及聊天运行态。
pub(crate) fn cache_external_session(app: &mut App, info: vw_shared::session::info::Info) {
    if let Some(existing) = app.sessions.iter_mut().find(|session| session.id == info.id) {
        *existing = info.clone();
    } else {
        app.sessions.insert(0, info.clone());
    }

    let sessions = app.project_sessions.entry(info.directory.clone()).or_default();
    if let Some(existing) = sessions.iter_mut().find(|session| session.id == info.id) {
        *existing = info;
    } else {
        sessions.insert(0, info);
    }
}

fn push_loaded_chat_message(
    chat: &mut Vec<models::ChatMessage>,
    message_ids: &mut Vec<Option<String>>,
    role: models::ChatRole,
    content: String,
    model: Option<String>,
    think_timing: Vec<models::ThinkTiming>,
    message_id: Option<String>,
    processing_timing: &mut Option<vw_shared::session::ui_types::ProcessingTiming>,
) {
    if content.trim().is_empty() && think_timing.is_empty() {
        return;
    }

    chat.push(models::ChatMessage {
        role,
        content,
        model,
        think_timing,
        processing_timing: processing_timing.take(),
    });
    message_ids.push(message_id);
}

fn flush_loaded_text_message(
    chat: &mut Vec<models::ChatMessage>,
    message_ids: &mut Vec<Option<String>>,
    role: models::ChatRole,
    content: &mut String,
    model: Option<String>,
    think_timing: &mut Vec<models::ThinkTiming>,
    message_id: &Option<String>,
    processing_timing: &mut Option<vw_shared::session::ui_types::ProcessingTiming>,
) {
    push_loaded_chat_message(
        chat,
        message_ids,
        role,
        std::mem::take(content),
        model,
        std::mem::take(think_timing),
        message_id.clone(),
        processing_timing,
    );
}

fn tool_input_text(input: &Map<String, Value>) -> Option<String> {
    if input.is_empty() {
        return None;
    }

    serde_json::to_string(input).ok().filter(|value| !value.trim().is_empty())
}

fn tool_error_is_denied(error: &str, metadata: Option<&Map<String, Value>>) -> bool {
    if metadata.is_some_and(|value| {
        value.contains_key("permission_request") || value.contains_key("permissionRequest")
    }) {
        return true;
    }

    let lowered = error.to_ascii_lowercase();
    lowered.contains("approval required")
        || lowered.contains("requires approval")
        || lowered.contains("denied")
        || lowered.contains("forbidden")
        || lowered.contains("not allowed")
        || lowered.contains("permission")
}

fn merged_tool_metadata(
    metadata: Option<&Map<String, Value>>,
    call_id: &str,
) -> Option<Map<String, Value>> {
    let mut merged = metadata.cloned().unwrap_or_default();
    if !merged.contains_key("callID")
        && !merged.contains_key("toolCallId")
        && !merged.contains_key("call_id")
        && !merged.contains_key("callId")
    {
        merged.insert("callID".to_string(), Value::String(call_id.to_string()));
    }

    if merged.is_empty() { None } else { Some(merged) }
}

fn lift_tool_metadata(payload: &mut Map<String, Value>, metadata: &Map<String, Value>) {
    if let Some(value) = metadata
        .get("permission_request")
        .cloned()
        .or_else(|| metadata.get("permissionRequest").cloned())
    {
        payload.entry("permission_request".to_string()).or_insert(value);
    }

    if let Some(value) = metadata.get("summary").cloned() {
        payload.entry("summary".to_string()).or_insert(value);
    }

    if let Some(value) =
        metadata.get("renderHint").cloned().or_else(|| metadata.get("render_hint").cloned())
    {
        payload.entry("renderHint".to_string()).or_insert(value);
    }
}

fn insert_string_field(payload: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        payload.insert(key.to_string(), Value::String(value));
    }
}

fn insert_value_field(payload: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        payload.insert(key.to_string(), value);
    }
}

fn tool_part_payload(part: &agent_message::ToolPart) -> Value {
    let mut payload = Map::new();
    payload.insert("callID".to_string(), Value::String(part.call_id.clone()));
    payload.insert("toolCallId".to_string(), Value::String(part.call_id.clone()));

    match &part.state {
        agent_message::ToolState::Pending(state) => {
            payload.insert("status".to_string(), Value::String("pending".to_string()));
            insert_string_field(
                &mut payload,
                "input",
                if state.raw.trim().is_empty() {
                    tool_input_text(&state.input)
                } else {
                    Some(state.raw.clone())
                },
            );
        }
        agent_message::ToolState::Running(state) => {
            payload.insert("status".to_string(), Value::String("running".to_string()));
            insert_string_field(&mut payload, "input", tool_input_text(&state.input));
            insert_string_field(&mut payload, "title", state.title.clone());
            insert_value_field(&mut payload, "time", serde_json::to_value(&state.time).ok());

            if let Some(metadata) = merged_tool_metadata(state.metadata.as_ref(), &part.call_id) {
                lift_tool_metadata(&mut payload, &metadata);
                payload.insert("metadata".to_string(), Value::Object(metadata));
            }
        }
        agent_message::ToolState::Completed(state) => {
            payload.insert("status".to_string(), Value::String("completed".to_string()));
            insert_string_field(&mut payload, "input", tool_input_text(&state.input));
            insert_string_field(&mut payload, "title", Some(state.title.clone()));
            insert_string_field(&mut payload, "output", Some(state.output.clone()));
            insert_value_field(&mut payload, "time", serde_json::to_value(&state.time).ok());
            insert_value_field(
                &mut payload,
                "attachments",
                state
                    .attachments
                    .as_ref()
                    .filter(|attachments| !attachments.is_empty())
                    .and_then(|attachments| serde_json::to_value(attachments).ok()),
            );

            if let Some(metadata) = merged_tool_metadata(Some(&state.metadata), &part.call_id) {
                lift_tool_metadata(&mut payload, &metadata);
                payload.insert("metadata".to_string(), Value::Object(metadata));
            }
        }
        agent_message::ToolState::Error(state) => {
            let status = if tool_error_is_denied(&state.error, state.metadata.as_ref()) {
                "denied"
            } else {
                "error"
            };
            payload.insert("status".to_string(), Value::String(status.to_string()));
            insert_string_field(&mut payload, "input", tool_input_text(&state.input));
            insert_string_field(&mut payload, "error", Some(state.error.clone()));
            insert_value_field(&mut payload, "time", serde_json::to_value(&state.time).ok());

            if let Some(metadata) = merged_tool_metadata(state.metadata.as_ref(), &part.call_id) {
                lift_tool_metadata(&mut payload, &metadata);
                payload.insert("metadata".to_string(), Value::Object(metadata));
            }
        }
    }

    Value::Object(payload)
}

fn tool_part_message_content(part: &agent_message::ToolPart) -> String {
    format!("tool {}\n{}\n", part.tool, tool_part_payload(part))
}

fn assistant_info(message: &agent_message::WithParts) -> Option<&agent_message::AssistantInfo> {
    match &message.info {
        agent_message::Info::Assistant(info) => Some(info.as_ref()),
        agent_message::Info::User(_) => None,
    }
}

fn token_usage_from_info(tokens: &agent_message::TokenInfo) -> models::TokenUsage {
    models::TokenUsage {
        input_tokens: tokens.input,
        output_tokens: tokens.output,
        cached_tokens: tokens.cache.read + tokens.cache.write,
        reasoning_tokens: tokens.reasoning,
    }
}

fn token_usage_has_value(usage: &models::TokenUsage) -> bool {
    usage.input_tokens != 0
        || usage.output_tokens != 0
        || usage.cached_tokens != 0
        || usage.reasoning_tokens != 0
}

fn assistant_model_label(info: &agent_message::AssistantInfo) -> Option<String> {
    if info.model_id.trim().is_empty() {
        None
    } else if info.provider_id.trim().is_empty() {
        Some(info.model_id.clone())
    } else {
        Some(format!("{}/{}", info.provider_id, info.model_id))
    }
}

fn message_model_label(info: &agent_message::Info) -> Option<String> {
    match info {
        agent_message::Info::User(info) => {
            let provider_id = info.model.provider_id.trim();
            let model_id = info.model.model_id.trim();
            if model_id.is_empty() {
                None
            } else if provider_id.is_empty() {
                Some(model_id.to_string())
            } else {
                Some(format!("{provider_id}/{model_id}"))
            }
        }
        agent_message::Info::Assistant(info) => assistant_model_label(info),
    }
}

fn message_processing_timing(
    info: &agent_message::Info,
) -> Option<vw_shared::session::ui_types::ProcessingTiming> {
    let agent_message::Info::Assistant(info) = info else {
        return None;
    };
    let completed_ms = info.time.completed?;
    Some(vw_shared::session::ui_types::ProcessingTiming {
        started_ms: info.time.created,
        completed_ms: Some(completed_ms),
        processing_elapsed_ms: info.time.processing_elapsed_ms,
        llm_elapsed_ms: info.time.llm_elapsed_ms,
    })
}

pub(crate) fn latest_assistant_model_label(msgs: &[agent_message::WithParts]) -> Option<String> {
    msgs.iter()
        .filter_map(assistant_info)
        .filter_map(|info| {
            assistant_model_label(info).map(|model| {
                let timestamp = info.time.completed.unwrap_or(info.time.created);
                (timestamp, model)
            })
        })
        .max_by_key(|(timestamp, _)| *timestamp)
        .map(|(_, model)| model)
}

fn upsert_loaded_step_start(
    steps: &mut Vec<models::ChatSessionStep>,
    index: u32,
    started_ms: u64,
    model: Option<String>,
    snapshot: Option<String>,
) {
    if let Some(step) = steps.iter_mut().find(|step| step.index == index) {
        step.started_ms = started_ms;
        if model.is_some() {
            step.model = model;
        }
        if snapshot.is_some() {
            step.start_snapshot_path = snapshot;
        }
    } else {
        steps.push(models::ChatSessionStep {
            index,
            turn_id: None,
            started_ms,
            finished_ms: None,
            start_snapshot_path: snapshot,
            finish_snapshot_path: None,
            usage: models::TokenUsage::default(),
            cost_usd: None,
            finish_reason: None,
            model,
            tool_timings: Vec::new(),
        });
    }
}

fn upsert_loaded_step_finish(
    steps: &mut Vec<models::ChatSessionStep>,
    index: u32,
    finished_ms: u64,
    usage: models::TokenUsage,
    finish_reason: Option<String>,
    model: Option<String>,
    cost_usd: Option<f64>,
    snapshot: Option<String>,
) {
    if let Some(step) = steps.iter_mut().find(|step| step.index == index) {
        step.finished_ms = Some(finished_ms);
        step.usage = usage;
        if finish_reason.is_some() {
            step.finish_reason = finish_reason;
        }
        if model.is_some() {
            step.model = model;
        }
        if cost_usd.is_some() {
            step.cost_usd = cost_usd;
        }
        if snapshot.is_some() {
            step.finish_snapshot_path = snapshot;
        }
    } else {
        steps.push(models::ChatSessionStep {
            index,
            turn_id: None,
            started_ms: finished_ms,
            finished_ms: Some(finished_ms),
            start_snapshot_path: None,
            finish_snapshot_path: snapshot,
            usage,
            cost_usd,
            finish_reason,
            model,
            tool_timings: Vec::new(),
        });
    }
}

pub(crate) fn steps_from_gateway_messages(
    msgs: &[agent_message::WithParts],
) -> Vec<models::ChatSessionStep> {
    let mut steps = Vec::new();
    let mut next_index = 1u32;
    let mut active_step_index = None;

    for message in msgs {
        let assistant = assistant_info(message);
        let started_ms = assistant.map(|info| info.time.created).unwrap_or_default();
        let finished_ms = assistant
            .and_then(|info| info.time.completed)
            .or_else(|| assistant.map(|info| info.time.created))
            .unwrap_or_default();
        let model = assistant.and_then(assistant_model_label);
        let assistant_usage =
            assistant.map(|info| token_usage_from_info(&info.tokens)).unwrap_or_default();
        let assistant_finish = assistant.and_then(|info| info.finish.clone());
        let assistant_cost = assistant.and_then(|info| (info.cost > 0.0).then_some(info.cost));

        for part in &message.parts {
            match part {
                agent_message::Part::StepStart(part) => {
                    let index = next_index;
                    next_index = next_index.saturating_add(1);
                    active_step_index = Some(index);
                    upsert_loaded_step_start(
                        &mut steps,
                        index,
                        started_ms,
                        model.clone(),
                        part.snapshot.clone(),
                    );
                }
                agent_message::Part::StepFinish(part) => {
                    let index = active_step_index.unwrap_or_else(|| {
                        let index = next_index;
                        next_index = next_index.saturating_add(1);
                        index
                    });
                    let part_usage = token_usage_from_info(&part.tokens);
                    let usage = if token_usage_has_value(&part_usage) {
                        part_usage
                    } else {
                        assistant_usage.clone()
                    };
                    let finish_reason = assistant_finish
                        .clone()
                        .or_else(|| (!part.reason.trim().is_empty()).then(|| part.reason.clone()));
                    let cost_usd = (part.cost > 0.0).then_some(part.cost).or(assistant_cost);
                    upsert_loaded_step_finish(
                        &mut steps,
                        index,
                        finished_ms,
                        usage,
                        finish_reason,
                        model.clone(),
                        cost_usd,
                        part.snapshot.clone(),
                    );
                    active_step_index = None;
                }
                _ => {}
            }
        }
    }

    if steps.is_empty() {
        for message in msgs {
            if let Some(info) = assistant_info(message) {
                let usage = token_usage_from_info(&info.tokens);
                if !token_usage_has_value(&usage) {
                    continue;
                }
                let index = next_index;
                next_index = next_index.saturating_add(1);
                steps.push(models::ChatSessionStep {
                    index,
                    turn_id: None,
                    started_ms: info.time.created,
                    finished_ms: info.time.completed,
                    start_snapshot_path: None,
                    finish_snapshot_path: None,
                    usage,
                    cost_usd: (info.cost > 0.0).then_some(info.cost),
                    finish_reason: info.finish.clone(),
                    model: assistant_model_label(info),
                    tool_timings: Vec::new(),
                });
            }
        }
    }

    steps.sort_by_key(|step| step.index);
    steps
}

/// loaded_chat_from_gateway_messages 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn loaded_chat_from_gateway_messages(
    mut msgs: Vec<agent_message::WithParts>,
) -> (Vec<models::ChatMessage>, Vec<Option<String>>) {
    let mut chat = Vec::new();
    let mut message_ids = Vec::new();

    let user_created_by_id = msgs
        .iter()
        .filter_map(|message| match &message.info {
            agent_message::Info::User(info) => Some((info.id.clone(), info.time.created)),
            agent_message::Info::Assistant(_) => None,
        })
        .collect::<std::collections::HashMap<_, _>>();
    let mut assistant_created_by_parent = std::collections::HashMap::<String, u64>::new();
    for message in &msgs {
        let agent_message::Info::Assistant(info) = &message.info else {
            continue;
        };
        if info.parent_id.trim().is_empty() {
            continue;
        }
        assistant_created_by_parent
            .entry(info.parent_id.clone())
            .and_modify(|created| *created = (*created).min(info.time.created))
            .or_insert(info.time.created);
    }
    msgs.sort_by_key(|message| match &message.info {
        agent_message::Info::User(info) => {
            let anchor = assistant_created_by_parent
                .get(&info.id)
                .copied()
                .map_or(info.time.created, |created| created.min(info.time.created));
            (anchor, info.id.clone(), 0_u8, info.time.created)
        }
        agent_message::Info::Assistant(info) => {
            let has_parent = !info.parent_id.trim().is_empty();
            let parent_created =
                has_parent.then(|| user_created_by_id.get(&info.parent_id).copied()).flatten();
            let anchor =
                parent_created.map_or(info.time.created, |created| created.min(info.time.created));
            let turn_id = if has_parent { info.parent_id.clone() } else { info.id.clone() };
            (anchor, turn_id, 1_u8, info.time.created)
        }
    });

    for message in msgs {
        let role = match &message.info {
            agent_message::Info::User(_) => models::ChatRole::User,
            agent_message::Info::Assistant(_) => models::ChatRole::Assistant,
        };
        let message_id = Some(message.info.id().to_string());
        let model = message_model_label(&message.info);
        let mut processing_timing = message_processing_timing(&message.info);
        let mut content = String::new();
        let mut think_timing = Vec::new();
        let has_compaction =
            message.parts.iter().any(|part| matches!(part, agent_message::Part::Compaction(_)));

        // 网关消息按片段到达，这里保持原顺序合并，避免工具卡片和正文错位。

        for part in message.parts {
            match part {
                agent_message::Part::Text(part) if !has_compaction => content.push_str(&part.text),
                agent_message::Part::Compaction(_) => {
                    flush_loaded_text_message(
                        &mut chat,
                        &mut message_ids,
                        role.clone(),
                        &mut content,
                        model.clone(),
                        &mut think_timing,
                        &message_id,
                        &mut processing_timing,
                    );
                    push_loaded_chat_message(
                        &mut chat,
                        &mut message_ids,
                        models::ChatRole::System,
                        crate::app::components::chat_panel::message_view::COMPACTION_COMPLETED_TEXT
                            .to_string(),
                        None,
                        Vec::new(),
                        message_id.clone(),
                        &mut processing_timing,
                    );
                }
                agent_message::Part::Text(_) => {}
                agent_message::Part::Reasoning(part) => {
                    content.push_str("<think>");
                    content.push_str(&part.text);
                    content.push_str("</think>");
                    think_timing.push(models::ThinkTiming {
                        start_ms: part.time.start,
                        end_ms: part.time.end,
                        last_update_ms: part.time.end.unwrap_or(part.time.start),
                    });
                }
                agent_message::Part::Tool(part) => {
                    flush_loaded_text_message(
                        &mut chat,
                        &mut message_ids,
                        role.clone(),
                        &mut content,
                        model.clone(),
                        &mut think_timing,
                        &message_id,
                        &mut processing_timing,
                    );
                    push_loaded_chat_message(
                        &mut chat,
                        &mut message_ids,
                        models::ChatRole::Tool,
                        tool_part_message_content(&part),
                        None,
                        Vec::new(),
                        message_id.clone(),
                        &mut processing_timing,
                    );
                }
                agent_message::Part::File(part) => {
                    content.push_str(&format!("\n[File: {}]\n", part.url));
                }
                _ => {}
            }
        }

        flush_loaded_text_message(
            &mut chat,
            &mut message_ids,
            role,
            &mut content,
            model,
            &mut think_timing,
            &message_id,
            &mut processing_timing,
        );
    }

    (chat, message_ids)
}

pub(super) fn reset_boundary_loaded_end(
    loaded_ids: &[Option<String>],
    reset_boundary: &[Option<String>],
) -> usize {
    if let Some(boundary_id) = reset_boundary.iter().rev().find_map(Option::as_ref)
        && let Some(idx) = loaded_ids.iter().position(|id| id.as_ref() == Some(boundary_id))
    {
        return idx.saturating_add(1);
    }

    reset_boundary.len().min(loaded_ids.len())
}

fn filter_loaded_chat_after_reset_boundary(
    loaded_chat: Vec<models::ChatMessage>,
    loaded_ids: Vec<Option<String>>,
    reset_boundary: &[Option<String>],
    local_ids: &[Option<String>],
) -> (Vec<models::ChatMessage>, Vec<Option<String>>) {
    if reset_boundary.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let boundary_end = reset_boundary_loaded_end(&loaded_ids, reset_boundary);
    let retain_end = boundary_end.min(loaded_chat.len()).min(loaded_ids.len());
    let allowed_after_boundary: std::collections::HashSet<&str> =
        local_ids.iter().skip(reset_boundary.len()).filter_map(Option::as_deref).collect();

    let mut filtered_chat = Vec::with_capacity(loaded_chat.len().min(local_ids.len()));
    let mut filtered_ids = Vec::with_capacity(loaded_ids.len().min(local_ids.len()));

    for idx in 0..retain_end {
        filtered_chat.push(loaded_chat[idx].clone());
        filtered_ids.push(loaded_ids[idx].clone());
    }

    for idx in retain_end..loaded_chat.len().min(loaded_ids.len()) {
        let Some(message_id) = loaded_ids[idx].as_deref() else {
            continue;
        };
        if allowed_after_boundary.contains(message_id) {
            filtered_chat.push(loaded_chat[idx].clone());
            filtered_ids.push(loaded_ids[idx].clone());
        }
    }

    (filtered_chat, filtered_ids)
}

fn apply_prepared_session_open(
    app: &mut App,
    prepared: crate::app::session::PreparedSessionOpen,
    open_generation: u64,
) {
    let crate::app::session::PreparedSessionOpen {
        session_id,
        messages,
        message_ids,
        usage,
        steps,
        calls,
        step_llm_raw,
        llm_raw_step_indices,
        updated_ms,
        content_signature: _,
        session_model,
        base_ui_chunk,
        timeline_page,
    } = prepared;

    if let Some(page) = timeline_page {
        let mut timeline = crate::app::session::chat_timeline::PagedChatTimeline::new(
            session_id.clone(),
            open_generation,
        );
        timeline.merge_initial_page(page);
        app.paged_chat_timeline = Some(timeline);
    }

    app.chat_scroll_id = iced::widget::Id::unique();
    app.chat = messages;
    app.chat_message_ids = message_ids;
    app.usage = usage;
    app.active_session_view_state.updated_ms = updated_ms;
    let restored_session_model_from_steps =
        app.restore_session_model_from_steps(&session_id, &steps);
    app.replace_active_session_steps(steps);
    app.replace_active_session_calls(calls);
    app.replace_active_session_step_llm_raw(step_llm_raw);
    app.replace_active_session_llm_raw_step_indices(llm_raw_step_indices);
    let restored_session_model_from_history = session_model
        .as_deref()
        .is_some_and(|model| app.restore_session_model_from_history(&session_id, model));
    if restored_session_model_from_steps || restored_session_model_from_history {
        app.sync_active_session_preferences();
    }

    app.active_session_view_state.ui_preparing = true;
    app.active_session_view_state.base_ready = false;
    app.invalidate_chat_ui_state();
    if let Some(base_ui_chunk) = base_ui_chunk {
        let chunk_start_idx = base_ui_chunk.chunk_start_idx;
        app.pin_chat_ui_chunk(Some(chunk_start_idx));
        app.apply_prepared_chat_ui_phase(crate::app::session::PreparedChatUiPhase::Base(
            base_ui_chunk,
        ));
        app.active_session_view_state.base_ready = true;
        app.rebuild_active_session_message_meta();
    } else {
        app.pin_chat_ui_chunk(None);
    }
    app.active_session_view_state.ui_preparing = false;
}

fn remote_history_is_incomplete(
    current_ids: &[Option<String>],
    remote_ids: &[Option<String>],
) -> bool {
    if current_ids.is_empty() {
        return false;
    }

    let current_known = current_ids.iter().flatten().collect::<std::collections::HashSet<_>>();
    let remote_known = remote_ids.iter().flatten().collect::<std::collections::HashSet<_>>();
    let shared_known_count = current_known.intersection(&remote_known).count();

    (shared_known_count > 0 && shared_known_count < current_known.len())
        || (shared_known_count == 0 && remote_ids.len() < current_ids.len())
}

/// handle 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        ProjectMessage::SessionRightClicked(id, x, y) => {
            app.session_menu_id = Some(id);
            app.session_menu_anchor = Some(iced::Point::new(x, y));
            Some(iced::Task::none())
        }
        ProjectMessage::SessionMenuClose => {
            app.session_menu_id = None;
            app.session_menu_anchor = None;
            Some(iced::Task::none())
        }
        ProjectMessage::SessionTreeToggled(id) => {
            if !app.session_tree_expanded.remove(&id) {
                app.session_tree_expanded.insert(id);
            }
            Some(iced::Task::none())
        }
        ProjectMessage::SessionTitleClicked(id) => {
            let now = web_time::Instant::now();
            let is_double_click = if let Some(last) = app.session_title_last_click {
                now.duration_since(last).as_millis() < 400
            } else {
                false
            };
            app.session_title_last_click = Some(now);
            if is_double_click {
                app.session_menu_id = None;
                app.session_menu_anchor = None;
                app.show_session_actions_popover = false;
                app.session_rename_id = Some(id.clone());
                if let Some(s) = app.sessions.iter().find(|s| s.id == id) {
                    app.session_rename_value = s.title.clone();
                }
            }
            Some(iced::Task::none())
        }
        ProjectMessage::SessionRenamePressed(id) => {
            app.session_menu_id = None;
            app.session_menu_anchor = None;
            app.show_session_actions_popover = false;
            app.session_rename_id = Some(id.clone());
            if let Some(s) = app.sessions.iter().find(|s| s.id == id) {
                app.session_rename_value = s.title.clone();
            } else if let Some(s) = app
                .project_sessions
                .values()
                .find_map(|sessions| sessions.iter().find(|s| s.id == id))
            {
                app.session_rename_value = s.title.clone();
            }
            Some(iced::Task::none())
        }
        ProjectMessage::SessionRenameChanged(v) => {
            app.session_rename_value = v;
            Some(iced::Task::none())
        }
        ProjectMessage::SessionRenameCancel => {
            app.session_rename_id = None;
            app.session_rename_value.clear();
            Some(iced::Task::none())
        }
        ProjectMessage::SessionRenameSave => {
            let Some(id) = app.session_rename_id.clone() else {
                return Some(iced::Task::none());
            };
            let title = app.session_rename_value.trim().to_string();
            if title.is_empty() {
                return Some(iced::Task::none());
            }

            app.session_rename_id = None;
            app.session_rename_value.clear();

            if let Some(s) = app.sessions.iter_mut().find(|s| s.id == id) {
                s.title = title.clone();
            }
            for sessions in app.project_sessions.values_mut() {
                if let Some(s) = sessions.iter_mut().find(|s| s.id == id) {
                    s.title = title.clone();
                }
            }
            app.refresh_task_pet_session_title(&id, &title);

            let project_path = find_session_project_path(app, &id);
            Some(iced::Task::perform(
                async move {
                    let client = crate::app::gateway_client().map_err(|err| err.to_string())?;
                    client
                        .session_update::<vw_shared::session::info::Info>(
                            &id,
                            project_path.as_deref(),
                            &vw_gateway_client::GatewaySessionPatchBody {
                                title: Some(title),
                                time: None,
                            },
                        )
                        .await
                },
                |_| Message::None,
            ))
        }
        ProjectMessage::SessionArchivePressed(id) => {
            app.session_menu_id = None;
            app.session_menu_anchor = None;
            app.show_session_actions_popover = false;

            if let Some(s) = app.sessions.iter_mut().find(|s| s.id == id) {
                s.time.archived = Some(now_ms());
            }
            for sessions in app.project_sessions.values_mut() {
                if let Some(s) = sessions.iter_mut().find(|s| s.id == id) {
                    s.time.archived = Some(now_ms());
                }
            }

            let mut switch_task = iced::Task::none();
            if app.active_session_id.as_ref() == Some(&id) {
                let next = app
                    .sessions
                    .iter()
                    .find(|s| s.time.archived.is_none() && s.id != id)
                    .map(|s| s.id.clone());

                if let Some(next_id) = next {
                    app.prepare_historical_session_model(&next_id);
                    app.active_session_id = Some(next_id.clone());
                    app.mark_active_session_viewed();
                    app.sync_active_session_preferences();
                    switch_task = iced::Task::perform(
                        async move {
                            let client = match crate::app::gateway_client() {
                                Ok(client) => client,
                                Err(err) => return Err(err),
                            };
                            let msgs = client
                                .session_messages::<Vec<agent_message::WithParts>>(&next_id, None)
                                .await;
                            let info = client
                                .session_get::<vw_shared::session::info::Info>(&next_id, None)
                                .await;
                            match (msgs, info) {
                                (Ok(msgs), Ok(_info)) => {
                                    let mut usage = models::TokenUsage::default();
                                    for m in &msgs {
                                        if let agent_message::Info::Assistant(a) = &m.info {
                                            usage.input_tokens += a.tokens.input;
                                            usage.output_tokens += a.tokens.output;
                                            usage.cached_tokens +=
                                                a.tokens.cache.read + a.tokens.cache.write;
                                            usage.reasoning_tokens += a.tokens.reasoning;
                                        }
                                    }
                                    Ok((next_id, msgs, usage))
                                }
                                (Err(e), _) => Err(e.to_string()),
                                (_, Err(e)) => Err(e.to_string()),
                            }
                        },
                        |res| Message::Project(ProjectMessage::SessionMessagesLoaded(res)),
                    );
                } else {
                    app.active_session_id = None;
                    app.chat.clear();
                    app.usage = models::TokenUsage::default();
                    app.active_session_view_state = ActiveSessionViewState::default();
                    app.invalidate_chat_ui_state();
                }
            }

            let project_path = find_session_project_path(app, &id);
            Some(iced::Task::batch(vec![
                iced::Task::perform(
                    async move {
                        let client = match crate::app::gateway_client() {
                            Ok(client) => client,
                            Err(err) => return Err(err.to_string()),
                        };
                        client
                            .session_update::<vw_shared::session::info::Info>(
                                &id,
                                project_path.as_deref(),
                                &vw_gateway_client::GatewaySessionPatchBody {
                                    title: None,
                                    time: Some(vw_gateway_client::GatewaySessionPatchTime {
                                        archived: Some(vw_shared::time::now_ms()),
                                    }),
                                },
                            )
                            .await
                            .map_err(|err| err.to_string())
                    },
                    |_| Message::None,
                ),
                switch_task,
            ]))
        }
        ProjectMessage::SessionDeletePressed(id) => {
            app.session_menu_id = None;
            app.session_menu_anchor = None;
            app.show_session_actions_popover = false;
            app.session_runtime_states.remove(&id);
            app.session_view_cache.remove(&id);
            app.session_chat_cache.remove(&id);
            app.session_chat_message_id_cache.remove(&id);
            app.session_history_reset_boundaries.remove(&id);

            let project_path = find_session_project_path(app, &id);

            if let Some(pos) = app.sessions.iter().position(|s| s.id == id) {
                app.sessions.remove(pos);
            }
            for sessions in app.project_sessions.values_mut() {
                if let Some(pos) = sessions.iter().position(|s| s.id == id) {
                    sessions.remove(pos);
                }
            }

            let mut switch_task = iced::Task::none();

            if app.active_session_id.as_ref() == Some(&id) {
                let next =
                    app.sessions.iter().find(|s| s.time.archived.is_none()).map(|s| s.id.clone());

                if let Some(next_id) = next {
                    app.prepare_historical_session_model(&next_id);
                    app.active_session_id = Some(next_id.clone());
                    app.mark_active_session_viewed();
                    app.sync_active_session_preferences();
                    switch_task = iced::Task::perform(
                        async move {
                            let client = match crate::app::gateway_client() {
                                Ok(client) => client,
                                Err(err) => return Err(err),
                            };
                            let msgs = client
                                .session_messages::<Vec<agent_message::WithParts>>(&next_id, None)
                                .await;
                            match msgs {
                                Ok(msgs) => {
                                    let mut usage = models::TokenUsage::default();
                                    for m in &msgs {
                                        if let agent_message::Info::Assistant(a) = &m.info {
                                            usage.input_tokens += a.tokens.input;
                                            usage.output_tokens += a.tokens.output;
                                            usage.cached_tokens +=
                                                a.tokens.cache.read + a.tokens.cache.write;
                                            usage.reasoning_tokens += a.tokens.reasoning;
                                        }
                                    }
                                    Ok((next_id, msgs, usage))
                                }
                                Err(e) => Err(e.to_string()),
                            }
                        },
                        |res| Message::Project(ProjectMessage::SessionMessagesLoaded(res)),
                    );
                } else {
                    app.active_session_id = None;
                    app.chat.clear();
                    app.usage = models::TokenUsage::default();
                    app.active_session_view_state = ActiveSessionViewState::default();
                    app.invalidate_chat_ui_state();
                }
            }

            Some(iced::Task::batch(vec![
                iced::Task::perform(
                    async move {
                        let client = crate::app::gateway_client().map_err(|err| err.to_string())?;
                        client.session_delete(&id, project_path.as_deref()).await
                    },
                    |_| Message::None,
                ),
                switch_task,
            ]))
        }
        ProjectMessage::SessionCopyPressed(id) => {
            let project_path = find_session_project_path(app, &id);
            Some(iced::Task::perform(
                async move {
                    let client = crate::app::gateway_client().map_err(|err| err.to_string())?;
                    client
                        .session_fork::<vw_shared::session::info::Info>(
                            &id,
                            project_path.as_deref(),
                            &None,
                        )
                        .await
                },
                |res| match res {
                    Ok(info) => Message::Project(ProjectMessage::SessionCopied(info)),
                    Err(e) => {
                        eprintln!("Fork failed: {}", e);
                        Message::None
                    }
                },
            ))
        }
        ProjectMessage::SessionCopied(info) => {
            if app.project_path.as_ref() == Some(&info.directory)
                && !app.sessions.iter().any(|s| s.id == info.id)
            {
                app.sessions.insert(0, info.clone());
            }
            let sessions_key = app.project_path.clone().unwrap_or_else(|| info.directory.clone());
            if let Some(list) = app.project_sessions.get_mut(&sessions_key) {
                if !list.iter().any(|s| s.id == info.id) {
                    list.insert(0, info.clone());
                }
            } else {
                app.project_sessions.insert(sessions_key, vec![info.clone()]);
            }
            Some(iced::Task::none())
        }
        ProjectMessage::SessionCreated(info) => {
            let is_discovered_child =
                info.parent_id.as_deref().is_some_and(|parent_id| !parent_id.trim().is_empty());
            app.cache_active_session_chat();
            if !app.sessions.iter().any(|s| s.id == info.id) {
                app.sessions.insert(0, info.clone());
            }
            let sessions_key = app.project_path.clone().unwrap_or_else(|| info.directory.clone());
            if let Some(list) = app.project_sessions.get_mut(&sessions_key) {
                if !list.iter().any(|s| s.id == info.id) {
                    list.insert(0, info.clone());
                }
            } else {
                app.project_sessions.insert(sessions_key, vec![info.clone()]);
            }
            if is_discovered_child {
                return Some(iced::Task::none());
            }

            app.new_session_creating = false;
            app.new_session_last_directory = Some(info.directory.clone());
            let save_last_directory_task = save_config_field_task(
                "new_session_last_directory",
                serde_json::Value::String(info.directory.clone()),
            );
            app.active_session_id = Some(info.id.clone());
            app.chat_scroll_id = iced::widget::Id::unique();
            app.mark_active_session_viewed();
            app.chat.clear();
            app.chat_message_ids.clear();
            app.store_session_chat_snapshot(
                info.id.clone(),
                crate::app::session::shared_chat_messages(app.chat.clone()),
                app.chat_message_ids.clone(),
            );
            app.usage = models::TokenUsage::default();
            app.active_session_view_state = ActiveSessionViewState::default();
            app.invalidate_chat_ui_state();
            app.sync_active_session_preferences();
            Some(save_last_directory_task)
        }
        ProjectMessage::SessionOpenPrepared {
            target_session_id,
            project_path,
            open_generation,
            source,
            result,
        } => {
            let is_active_target = app.active_session_id.as_deref() == Some(&target_session_id)
                && app.session_view_cache.is_current_open(&target_session_id, open_generation);
            let prepared = match result {
                Ok(prepared) if is_active_target && prepared.session_id == target_session_id => {
                    prepared
                }
                Ok(_) => {
                    tracing::info!(
                        target: "vw_desktop",
                        session_id = %target_session_id,
                        open_generation,
                        branch = "stale_open_result",
                        "discarded prepared session open because the active request changed"
                    );
                    return Some(iced::Task::none());
                }
                Err(error) if source == PreparedSessionOpenSource::UiStore && is_active_target => {
                    tracing::warn!(
                        target: "vw_desktop",
                        session_id = %target_session_id,
                        error = %error,
                        "failed to prepare session UI store history; falling back to remote history"
                    );
                    return Some(load_prepared_session_open_from_gateway_task(
                        project_path,
                        target_session_id,
                        open_generation,
                        app.dialogue_flow_show_reasoning_summary,
                    ));
                }
                Err(error) if is_active_target => {
                    tracing::warn!(
                        target: "vw_desktop",
                        session_id = %target_session_id,
                        error = %error,
                        "failed to prepare remote session history"
                    );
                    if app.chat.is_empty() {
                        app.error_message =
                            Some(format!("Failed to load session messages: {error}"));
                    }
                    app.active_session_view_state.ui_preparing = false;
                    return Some(iced::Task::none());
                }
                Err(_) => return Some(iced::Task::none()),
            };

            let incomplete_remote_history = source == PreparedSessionOpenSource::RemoteHistory
                && (remote_history_is_incomplete(&app.chat_message_ids, &prepared.message_ids)
                    || app.session_history_reset_boundary(&target_session_id).is_some_and(
                        |boundary| {
                            boundary == app.chat_message_ids
                                && prepared.message_ids != app.chat_message_ids
                        },
                    ));
            if incomplete_remote_history {
                tracing::warn!(
                    target: "vw_desktop",
                    session_id = %target_session_id,
                    remote_messages = prepared.message_ids.len(),
                    retained_messages = app.chat_message_ids.len(),
                    "preserved prepared session history because remote history was incomplete"
                );
                app.active_session_view_state.ui_preparing = false;
                return Some(iced::Task::none());
            }

            let same_remote_history = source == PreparedSessionOpenSource::RemoteHistory
                && crate::app::session::session_content_signature(&app.chat, &app.chat_message_ids)
                    == prepared.content_signature;
            if same_remote_history {
                app.active_session_view_state.ui_preparing = false;
                app.usage = prepared.usage;
                if !prepared.steps.is_empty() {
                    app.replace_active_session_steps(prepared.steps);
                }
                if !prepared.step_llm_raw.is_empty() {
                    app.replace_active_session_step_llm_raw(prepared.step_llm_raw);
                }
                if !prepared.llm_raw_step_indices.is_empty() {
                    app.replace_active_session_llm_raw_step_indices(prepared.llm_raw_step_indices);
                }
                if let Some(summary) = app.session_view_cache.record_open_commit(
                    &target_session_id,
                    open_generation,
                    crate::app::session::view_cache::SessionOpenUiCommit::SameContentRefresh,
                    app.chat.len(),
                    web_time::Duration::ZERO,
                    true,
                ) {
                    summary.emit();
                }
                return Some(crate::app::message::chat::refresh_active_trajectory_raw_task(
                    app,
                    &target_session_id,
                ));
            }

            let commit_started_at = Instant::now();
            let message_count = prepared.messages.len();
            apply_prepared_session_open(app, prepared, open_generation);
            let trajectory_refresh_task =
                crate::app::message::chat::refresh_active_trajectory_raw_task(
                    app,
                    &target_session_id,
                );
            let finalize = source == PreparedSessionOpenSource::RemoteHistory;
            if let Some(summary) = app.session_view_cache.record_open_commit(
                &target_session_id,
                open_generation,
                crate::app::session::view_cache::SessionOpenUiCommit::PreparedHistory,
                message_count,
                commit_started_at.elapsed(),
                finalize,
            ) {
                summary.emit();
            }
            if source == PreparedSessionOpenSource::UiStore {
                return Some(iced::Task::batch([
                    load_prepared_session_open_from_gateway_task(
                        project_path,
                        target_session_id,
                        open_generation,
                        app.dialogue_flow_show_reasoning_summary,
                    ),
                    trajectory_refresh_task,
                ]));
            }
            Some(trajectory_refresh_task)
        }
        ProjectMessage::SessionMessagesLoaded(res) => {
            match res {
                Ok((id, msgs, usage)) => {
                    let loaded_session_model = latest_assistant_model_label(&msgs);
                    let loaded_steps = steps_from_gateway_messages(&msgs);
                    tracing::info!(
                        target: "vw_desktop",
                        session_id = %id,
                        message_count = msgs.len(),
                        active_session_matches = app.active_session_id.as_ref() == Some(&id),
                        "received session messages load result"
                    );
                    if app.active_session_id.as_ref() == Some(&id) {
                        let restored_session_model =
                            loaded_session_model.as_deref().is_some_and(|model| {
                                app.restore_session_model_from_history(&id, model)
                            });
                        let local_messages = app
                            .cached_chat_messages(&id)
                            .map(|chat| chat.iter().cloned().collect::<Vec<_>>());
                        let local_message_ids = app.cached_chat_message_ids(&id);
                        let reset_boundary =
                            app.session_history_reset_boundary(&id).map(|ids| ids.to_vec());
                        let (next_chat, next_ids) = if msgs.is_empty()
                            && let Some(local) = local_messages.clone()
                            && !local.is_empty()
                        {
                            let ids = local_message_ids.unwrap_or_else(|| vec![None; local.len()]);
                            (local, ids)
                        } else {
                            let (converted, converted_ids) =
                                loaded_chat_from_gateway_messages(msgs);
                            let (converted, converted_ids) =
                                if let Some(reset_boundary) = reset_boundary.as_deref() {
                                    filter_loaded_chat_after_reset_boundary(
                                        converted,
                                        converted_ids,
                                        reset_boundary,
                                        local_message_ids.as_deref().unwrap_or(&[]),
                                    )
                                } else {
                                    (converted, converted_ids)
                                };

                            if let Some(local) = local_messages.clone()
                                && converted.len() < local.len()
                            {
                                tracing::warn!(
                                    target: "vw_desktop",
                                    session_id = %id,
                                    gateway_messages = converted.len(),
                                    cached_messages = local.len(),
                                    "preserved cached session history because gateway history was shorter"
                                );
                                let ids =
                                    local_message_ids.unwrap_or_else(|| vec![None; local.len()]);
                                (local, ids)
                            } else if !converted.is_empty() {
                                (converted, converted_ids)
                            } else if let Some(local) = local_messages
                                && !local.is_empty()
                            {
                                let ids =
                                    local_message_ids.unwrap_or_else(|| vec![None; local.len()]);
                                (local, ids)
                            } else {
                                (Vec::new(), Vec::new())
                            }
                        };

                        let next_ids = if next_chat.len() == next_ids.len() {
                            next_ids
                        } else {
                            vec![None; next_chat.len()]
                        };
                        let same_visible_history = app.chat.len() == next_chat.len()
                            && app.chat.iter().zip(&next_chat).all(|(current, loaded)| {
                                current.role == loaded.role && current.content == loaded.content
                            });
                        if same_visible_history {
                            app.chat_message_ids = next_ids;
                            let shared_chat =
                                crate::app::session::shared_chat_messages(app.chat.clone());
                            app.store_session_chat_snapshot(
                                id.clone(),
                                shared_chat,
                                app.chat_message_ids.clone(),
                            );
                            app.usage = usage;
                            if !loaded_steps.is_empty() {
                                app.replace_active_session_steps(loaded_steps);
                            }
                            if restored_session_model {
                                app.sync_active_session_preferences();
                            }
                            app.rebuild_active_session_message_meta();
                            return Some(
                                crate::app::message::chat::refresh_active_trajectory_raw_task(
                                    app, &id,
                                ),
                            );
                        }

                        let shared_chat = crate::app::session::shared_chat_messages(next_chat);
                        app.chat_scroll_id = iced::widget::Id::unique();
                        app.chat = shared_chat.iter().cloned().collect();
                        app.chat_message_ids = next_ids;
                        app.store_session_chat_snapshot(
                            id.clone(),
                            shared_chat.clone(),
                            app.chat_message_ids.clone(),
                        );
                        app.usage = usage;
                        let existing_calls = app.active_session_view_state.calls.clone();
                        let existing_steps = app.active_session_view_state.steps.clone();
                        let next_steps =
                            if loaded_steps.is_empty() { existing_steps } else { loaded_steps };
                        app.active_session_view_state.updated_ms = 0;
                        app.clear_active_session_steps();
                        app.replace_active_session_steps(next_steps);
                        app.replace_active_session_calls(existing_calls);
                        if restored_session_model {
                            app.sync_active_session_preferences();
                        }
                        app.invalidate_chat_ui_state();
                        if app.chat.is_empty() {
                            tracing::warn!(
                                target: "vw_desktop",
                                session_id = %id,
                                "session message load completed but chat remained empty"
                            );
                        } else {
                            tracing::info!(
                                target: "vw_desktop",
                                session_id = %id,
                                chat_messages = app.chat.len(),
                                "applied loaded session messages to active chat"
                            );
                        }

                        if app.chat.is_empty() {
                            app.active_session_view_state.ui_preparing = false;
                            app.active_session_view_state.base_ready = true;
                            app.pin_chat_ui_chunk(None);
                        } else {
                            app.active_session_view_state.ui_preparing = true;
                            app.active_session_view_state.base_ready = false;
                            let base_chunk_start = app.preferred_base_chat_ui_chunk_start();
                            app.pin_chat_ui_chunk(Some(base_chunk_start));
                            app.mark_chat_ui_chunks_preparing(&[base_chunk_start]);
                            return Some(prepare_session_ui_task(
                                id,
                                shared_chat,
                                base_chunk_start,
                                true,
                                app.dialogue_flow_show_reasoning_summary,
                            ));
                        }
                    } else {
                        tracing::warn!(
                            target: "vw_desktop",
                            session_id = %id,
                            active_session_id = app.active_session_id.as_deref().unwrap_or("<none>"),
                            "discarded session messages because active session changed"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "vw_desktop",
                        active_session_id = app.active_session_id.as_deref().unwrap_or("<none>"),
                        error = %e,
                        "failed to load session messages"
                    );
                    if let Some(id) = app.active_session_id.clone()
                        && let Some(local) = app.cached_chat_messages(&id)
                        && !local.is_empty()
                    {
                        let local_messages = local.iter().cloned().collect::<Vec<_>>();
                        let local_message_ids = if let Some(ids) = app.cached_chat_message_ids(&id)
                            && ids.len() == local_messages.len()
                        {
                            ids
                        } else {
                            vec![None; local_messages.len()]
                        };
                        let shared_chat = crate::app::session::shared_chat_messages(local_messages);
                        app.chat = shared_chat.iter().cloned().collect();
                        app.chat_message_ids = local_message_ids;
                        app.store_session_chat_snapshot(
                            id.clone(),
                            shared_chat.clone(),
                            app.chat_message_ids.clone(),
                        );
                        app.clear_active_session_steps();
                        app.active_session_view_state.ui_preparing = true;
                        app.active_session_view_state.base_ready = false;
                        app.invalidate_chat_ui_state();
                        let base_chunk_start = app.preferred_base_chat_ui_chunk_start();
                        app.pin_chat_ui_chunk(Some(base_chunk_start));
                        app.mark_chat_ui_chunks_preparing(&[base_chunk_start]);
                        return Some(prepare_session_ui_task(
                            id,
                            shared_chat,
                            base_chunk_start,
                            true,
                            app.dialogue_flow_show_reasoning_summary,
                        ));
                    } else {
                        app.error_message = Some(format!("Failed to load session messages: {}", e));
                    }
                }
            }
            Some(iced::Task::none())
        }
        ProjectMessage::SessionUiPrepared { session_id, stream_generation, phase } => {
            let is_active_session = app.active_session_id.as_ref() == Some(&session_id);
            if let Some(generation) = stream_generation {
                let reprepare = {
                    let state = &mut app.get_session_runtime_mut(&session_id).stream_tail_state;
                    state.finish_ui_preparation(generation);
                    if state.ui_generation() == generation || !is_active_session {
                        None
                    } else {
                        state.message_idx().zip(state.begin_ui_preparation())
                    }
                };
                if let Some((message_idx, latest_generation)) = reprepare
                    && let Some(message) = app.chat.get(message_idx).cloned()
                {
                    return Some(crate::app::message::project::prepare_stream_message_ui_task(
                        session_id,
                        message,
                        message_idx,
                        latest_generation,
                        app.dialogue_flow_show_reasoning_summary,
                    ));
                }
                if app.get_session_runtime(&session_id).stream_tail_state.ui_generation()
                    != generation
                {
                    return Some(iced::Task::none());
                }
            }
            if is_active_session {
                let is_base_phase =
                    matches!(phase, crate::app::session::PreparedChatUiPhase::Base(_));
                app.apply_prepared_chat_ui_phase(phase);
                if is_base_phase {
                    app.active_session_view_state.base_ready = true;
                    app.rebuild_active_session_message_meta();
                    app.active_session_view_state.ui_preparing = false;
                    let (visible_start_idx, visible_end_idx) = app.visible_chat_message_window();
                    let nearby_chunk_starts =
                        app.pending_chat_ui_chunk_starts(visible_start_idx, visible_end_idx, true);
                    if !nearby_chunk_starts.is_empty() {
                        app.mark_chat_ui_chunks_preparing(&nearby_chunk_starts);
                        return Some(prepare_session_ui_chunks_task(
                            session_id,
                            app.active_shared_chat_messages(),
                            nearby_chunk_starts,
                            None,
                            app.dialogue_flow_show_reasoning_summary,
                        ));
                    }
                }
            }
            Some(iced::Task::none())
        }
        _ => None,
    }
}

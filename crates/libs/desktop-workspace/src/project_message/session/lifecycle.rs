//! 项目会话登记与网关历史转换；桌面壳负责跨域效果编排。
use serde_json::{Map, Value};
use message_types as agent_message;
use session_ui_types as models;

pub fn cache_project_session(
    current_sessions: &mut Vec<session_info::Info>,
    project_sessions: &mut std::collections::HashMap<String, Vec<session_info::Info>>,
    project_path: String,
    info: session_info::Info,
) {
    if let Some(existing) = current_sessions.iter_mut().find(|session| session.id == info.id) {
        *existing = info.clone();
    } else {
        current_sessions.insert(0, info.clone());
    }

    let sessions = project_sessions.entry(project_path).or_default();
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
    processing_timing: &mut Option<session_ui_types::ProcessingTiming>,
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
    processing_timing: &mut Option<session_ui_types::ProcessingTiming>,
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
) -> Option<session_ui_types::ProcessingTiming> {
    let agent_message::Info::Assistant(info) = info else {
        return None;
    };
    let completed_ms = info.time.completed?;
    Some(session_ui_types::ProcessingTiming {
        started_ms: info.time.created,
        completed_ms: Some(completed_ms),
        processing_elapsed_ms: info.time.processing_elapsed_ms,
        llm_elapsed_ms: info.time.llm_elapsed_ms,
    })
}

pub fn latest_assistant_model_label(msgs: &[agent_message::WithParts]) -> Option<String> {
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

pub fn steps_from_gateway_messages(
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
pub fn loaded_chat_from_gateway_messages(
    mut msgs: Vec<agent_message::WithParts>,
    compaction_completed_text: &str,
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
                        compaction_completed_text.to_string(),
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

fn reset_boundary_loaded_end(
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

pub fn remote_history_is_incomplete(
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



/// 合并网关与本地历史，保留重置边界及较完整的本地快照。
pub fn select_loaded_history(
    msgs: Vec<agent_message::WithParts>,
    local_messages: Option<Vec<models::ChatMessage>>,
    local_message_ids: Option<Vec<Option<String>>>,
    reset_boundary: Option<Vec<Option<String>>>,
    compaction_completed_text: &str,
) -> (Vec<models::ChatMessage>, Vec<Option<String>>, Option<(usize, usize)>) {
    let mut shorter_gateway = None;
    let (next_chat, next_ids) = if msgs.is_empty()
        && let Some(local) = local_messages.clone()
        && !local.is_empty()
    {
        let ids = local_message_ids.unwrap_or_else(|| vec![None; local.len()]);
        (local, ids)
    } else {
        let (converted, converted_ids) =
            loaded_chat_from_gateway_messages(msgs, compaction_completed_text);
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
            shorter_gateway = Some((converted.len(), local.len()));
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
    (next_chat, next_ids, shorter_gateway)
}

pub fn same_visible_history(current: &[models::ChatMessage], loaded: &[models::ChatMessage]) -> bool {
    current.len() == loaded.len() && current.iter().zip(loaded).all(|(current, loaded)| {
        current.role == loaded.role && current.content == loaded.content
    })
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;

//! 会话对话历史的构造、裁剪与状态同步。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use agent_client_protocol::{ContentBlock, SessionNotification};
use serde_json::{Map, Value};
use vw_api_types::id::ToolId;
use vw_api_types::tools::{StructuredPatchHunkDto, ToolResultContentDto, ToolResultDto};

use crate::prompt_content::{PromptInput, text_prompt};
use crate::types::{
    ClientOperation, SessionAcpxState, SessionAgentContent, SessionAgentMessage,
    SessionConversation, SessionMention, SessionMessage, SessionMessageImage, SessionThinking,
    SessionTokenUsage, SessionToolResult, SessionToolResultContent, SessionToolUse,
    SessionUserContent, SessionUserMessage,
};

const MAX_RUNTIME_MESSAGES: usize = 200;
const MAX_RUNTIME_AGENT_TEXT_CHARS: usize = 8_000;
const MAX_RUNTIME_THINKING_CHARS: usize = 4_000;
const MAX_RUNTIME_TOOL_IO_CHARS: usize = 4_000;
const MAX_RUNTIME_REQUEST_TOKEN_USAGE: usize = 100;

static NEXT_USER_MESSAGE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyHistoryEntry {
    pub role: LegacyHistoryRole,
    pub timestamp: String,
    pub text_preview: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyHistoryRole {
    User,
    Assistant,
}

fn now_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn next_user_message_id() -> String {
    let counter = NEXT_USER_MESSAGE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("user-{nanos}-{counter}")
}

fn content_block_value(content: &ContentBlock) -> Option<Value> {
    serde_json::to_value(content).ok()
}

fn content_block_record(content: &ContentBlock) -> Option<Map<String, Value>> {
    content_block_value(content)?.as_object().cloned()
}

fn extract_text(content: &ContentBlock) -> Option<String> {
    let record = content_block_record(content)?;
    match record.get("type").and_then(Value::as_str) {
        Some("text") => record.get("text").and_then(Value::as_str).map(ToOwned::to_owned),
        Some("resource_link") => record
            .get("title")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .or_else(|| record.get("name").and_then(Value::as_str))
            .filter(|value| !value.trim().is_empty())
            .or_else(|| record.get("uri").and_then(Value::as_str))
            .map(ToOwned::to_owned),
        Some("resource") => {
            let resource = record.get("resource").and_then(Value::as_object)?;
            resource
                .get("text")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .or_else(|| resource.get("uri").and_then(Value::as_str))
                .map(ToOwned::to_owned)
        }
        _ => None,
    }
}

fn content_to_user_content(content: &ContentBlock) -> Option<SessionUserContent> {
    let record = content_block_record(content)?;
    match record.get("type").and_then(Value::as_str) {
        Some("text") => record
            .get("text")
            .and_then(Value::as_str)
            .map(|value| SessionUserContent::Text(value.to_string())),
        Some("resource_link") => {
            let uri = record.get("uri").and_then(Value::as_str)?.to_string();
            let label = record
                .get("title")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .or_else(|| record.get("name").and_then(Value::as_str))
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(&uri)
                .to_string();
            Some(SessionUserContent::Mention(SessionMention { uri, content: label }))
        }
        Some("resource") => {
            let resource = record.get("resource").and_then(Value::as_object)?;
            if let Some(text) = resource
                .get("text")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
            {
                return Some(SessionUserContent::Text(text.to_string()));
            }
            let uri = resource.get("uri").and_then(Value::as_str)?.to_string();
            Some(SessionUserContent::Mention(SessionMention { uri: uri.clone(), content: uri }))
        }
        Some("image") => record.get("data").and_then(Value::as_str).map(|source| {
            SessionUserContent::Image(SessionMessageImage {
                source: source.to_string(),
                size: None,
            })
        }),
        _ => None,
    }
}

fn update_conversation_timestamp(conversation: &mut SessionConversation, timestamp: &str) {
    conversation.updated_at = timestamp.to_string();
}

fn ensure_agent_message(conversation: &mut SessionConversation) -> &mut SessionAgentMessage {
    let needs_new = !matches!(conversation.messages.last(), Some(SessionMessage::Agent(_)));
    if needs_new {
        conversation.messages.push(SessionMessage::Agent(SessionAgentMessage {
            content: Vec::new(),
            tool_results: HashMap::new(),
            reasoning_details: None,
        }));
    }

    match conversation.messages.last_mut() {
        Some(SessionMessage::Agent(agent)) => agent,
        _ => unreachable!(),
    }
}

fn append_agent_text(agent: &mut SessionAgentMessage, text: &str) {
    if text.trim().is_empty() {
        return;
    }

    if let Some(SessionAgentContent::Text(existing)) = agent.content.last_mut() {
        *existing = trim_runtime_text(&(existing.clone() + text), MAX_RUNTIME_AGENT_TEXT_CHARS);
        return;
    }

    agent.content.push(SessionAgentContent::Text(text.to_string()));
}

fn append_agent_thinking(agent: &mut SessionAgentMessage, text: &str) {
    if text.trim().is_empty() {
        return;
    }

    if let Some(SessionAgentContent::Thinking(existing)) = agent.content.last_mut() {
        existing.text =
            trim_runtime_text(&(existing.text.clone() + text), MAX_RUNTIME_THINKING_CHARS);
        return;
    }

    agent.content.push(SessionAgentContent::Thinking(SessionThinking {
        text: text.to_string(),
        signature: None,
    }));
}

fn trim_runtime_text(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }

    let keep = max_chars.saturating_sub(3);
    value.chars().take(keep).collect::<String>() + "..."
}

fn status_indicates_complete(status: Option<&str>) -> bool {
    let Some(status) = status else {
        return false;
    };
    let normalized = status.to_ascii_lowercase();
    normalized.contains("complete")
        || normalized.contains("done")
        || normalized.contains("success")
        || normalized.contains("failed")
        || normalized.contains("error")
        || normalized.contains("cancel")
}

fn status_indicates_error(status: Option<&str>) -> bool {
    let Some(status) = status else {
        return false;
    };
    let normalized = status.to_ascii_lowercase();
    normalized.contains("fail") || normalized.contains("error")
}

fn extract_tool_result_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        Value::Array(values) => values.iter().find_map(extract_tool_result_text),
        Value::Object(object) => {
            for key in [
                "summary",
                "model_result",
                "output",
                "content",
                "text",
                "stdout",
                "stderr",
                "message",
                "data",
                "value",
            ] {
                if let Some(extracted) = object.get(key).and_then(extract_tool_result_text)
                    && !extracted.trim().is_empty()
                {
                    return Some(extracted);
                }
            }
            None
        }
    }
}

fn structured_patch_diff_text(hunks: &[StructuredPatchHunkDto]) -> Option<String> {
    if hunks.is_empty() {
        return None;
    }

    let mut diff = String::new();
    let mut current_path = String::new();

    for hunk in hunks {
        let Some(path) = hunk.path.as_deref().map(str::trim).filter(|path| !path.is_empty()) else {
            continue;
        };

        if path != current_path {
            if !diff.is_empty() && !diff.ends_with('\n') {
                diff.push('\n');
            }
            diff.push_str("--- a/");
            diff.push_str(path);
            diff.push('\n');
            diff.push_str("+++ b/");
            diff.push_str(path);
            diff.push('\n');
            current_path = path.to_string();
        }

        if !hunk.header.trim().is_empty() {
            diff.push_str(&hunk.header);
            diff.push('\n');
        }
        for line in &hunk.lines {
            diff.push_str(line);
            diff.push('\n');
        }
    }

    (!diff.trim().is_empty()).then_some(diff)
}

fn extract_tool_result_dto_text(result: &ToolResultDto) -> Option<String> {
    if let Some(summary) = result
        .render_hint
        .as_ref()
        .and_then(|hint| hint.summary.as_deref())
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
    {
        return Some(summary.to_string());
    }

    for block in &result.content {
        match block {
            ToolResultContentDto::Text { text } if !text.trim().is_empty() => {
                return Some(text.clone());
            }
            ToolResultContentDto::Json { value } => {
                if let Some(text) = extract_tool_result_text(value)
                    && !text.trim().is_empty()
                {
                    return Some(text);
                }
            }
            ToolResultContentDto::StructuredPatch { hunks } => {
                if let Some(diff) = structured_patch_diff_text(hunks) {
                    return Some(diff);
                }
            }
            _ => {}
        }
    }

    extract_tool_result_text(&result.model_result)
        .or_else(|| extract_tool_result_text(&result.data))
}

fn to_tool_result_content_from_dto(result: &ToolResultDto) -> SessionToolResultContent {
    let text = extract_tool_result_dto_text(result)
        .map(|value| trim_runtime_text(&value, MAX_RUNTIME_TOOL_IO_CHARS))
        .unwrap_or_default();
    SessionToolResultContent::Text(text)
}

fn parse_tool_result_dto(
    raw_output: &Value,
    tool_call_id: &str,
    tool_kind: Option<&str>,
) -> Option<ToolResultDto> {
    let raw_record = raw_output.as_object()?;
    let has_dto_shape = raw_record.get("content").is_some_and(Value::is_array)
        || raw_record.contains_key("tool_use_id")
        || raw_record.contains_key("toolUseId")
        || raw_record.contains_key("tool_id")
        || raw_record.contains_key("toolId")
        || raw_record.contains_key("success")
        || raw_record.contains_key("data")
        || raw_record.contains_key("model_result")
        || raw_record.contains_key("modelResult")
        || raw_record.contains_key("render_hint")
        || raw_record.contains_key("renderHint");
    if !has_dto_shape {
        return None;
    }

    let mut result = serde_json::from_value::<ToolResultDto>(raw_output.clone()).ok()?;

    if result.tool_use_id.as_deref().is_none_or(|value| value.trim().is_empty()) {
        result.tool_use_id = Some(tool_call_id.to_string());
    }

    if result.tool_id.is_none()
        && let Some(tool_kind) = normalize_agent_name(tool_kind)
    {
        result.tool_id = Some(ToolId::from(tool_kind.as_str()));
    }

    Some(result)
}

fn to_tool_result_content(value: &Value) -> SessionToolResultContent {
    let text = extract_tool_result_text(value)
        .map(|value| trim_runtime_text(&value, MAX_RUNTIME_TOOL_IO_CHARS))
        .unwrap_or_else(|| {
            serde_json::to_string(value)
                .map(|value| trim_runtime_text(&value, MAX_RUNTIME_TOOL_IO_CHARS))
                .unwrap_or_else(|_| "[Unserializable value]".to_string())
        });
    SessionToolResultContent::Text(text)
}

fn to_raw_input(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(value)) => trim_runtime_text(value, MAX_RUNTIME_TOOL_IO_CHARS),
        Some(Value::Null) | None => "{}".to_string(),
        Some(value) => serde_json::to_string(value)
            .map(|value| trim_runtime_text(&value, MAX_RUNTIME_TOOL_IO_CHARS))
            .unwrap_or_else(|_| "[Unserializable input]".to_string()),
    }
}

fn ensure_tool_use_content<'a>(
    agent: &'a mut SessionAgentMessage,
    tool_call_id: &str,
) -> &'a mut SessionToolUse {
    let existing_index = agent.content.iter().position(|content| {
        matches!(
            content,
            SessionAgentContent::ToolUse(tool_use) if tool_use.id == tool_call_id
        )
    });

    if let Some(index) = existing_index {
        match agent.content.get_mut(index) {
            Some(SessionAgentContent::ToolUse(tool_use)) => return tool_use,
            _ => unreachable!(),
        }
    }

    agent.content.push(SessionAgentContent::ToolUse(SessionToolUse {
        id: tool_call_id.to_string(),
        name: "tool_call".to_string(),
        raw_input: "{}".to_string(),
        input: Value::Object(Map::new()),
        is_input_complete: false,
        thought_signature: None,
    }));

    match agent.content.last_mut() {
        Some(SessionAgentContent::ToolUse(tool_use)) => tool_use,
        _ => unreachable!(),
    }
}

fn upsert_tool_result(
    agent: &mut SessionAgentMessage,
    tool_call_id: &str,
    tool_name: Option<String>,
    is_error: Option<bool>,
    content: Option<SessionToolResultContent>,
    output: Option<Value>,
    result: Option<ToolResultDto>,
) {
    let existing = agent.tool_results.get(tool_call_id).cloned();
    let next = SessionToolResult {
        tool_use_id: tool_call_id.to_string(),
        tool_name: tool_name
            .or_else(|| existing.as_ref().map(|result| result.tool_name.clone()))
            .unwrap_or_else(|| "tool_call".to_string()),
        is_error: is_error
            .or_else(|| existing.as_ref().map(|result| result.is_error))
            .unwrap_or(false),
        content: content
            .or_else(|| existing.as_ref().map(|result| result.content.clone()))
            .unwrap_or_else(|| SessionToolResultContent::Text(String::new())),
        output: output.or_else(|| existing.as_ref().and_then(|result| result.output.clone())),
        result: result.or_else(|| existing.and_then(|result| result.result)),
    };
    agent.tool_results.insert(tool_call_id.to_string(), next);
}

fn normalize_agent_name(value: Option<&str>) -> Option<String> {
    value.map(str::trim).filter(|value| !value.is_empty()).map(ToOwned::to_owned)
}

fn apply_tool_call_update(agent: &mut SessionAgentMessage, update: &Map<String, Value>) {
    let Some(tool_call_id) = update.get("toolCallId").and_then(Value::as_str) else {
        return;
    };

    let mut tool_name_override: Option<String> = None;
    let mut output: Option<Value> = None;
    let mut result: Option<ToolResultDto> = None;
    let status: Option<&str>;

    {
        let tool = ensure_tool_use_content(agent, tool_call_id);

        if let Some(title) = normalize_agent_name(update.get("title").and_then(Value::as_str)) {
            tool.name = title.clone();
            tool_name_override = Some(title);
        }

        if let Some(kind_name) = normalize_agent_name(update.get("kind").and_then(Value::as_str))
            && tool.name == "tool_call"
        {
            tool.name = kind_name.clone();
            tool_name_override = Some(kind_name);
        }

        if let Some(raw_input) = update.get("rawInput") {
            tool.input =
                if raw_input.is_null() { Value::Object(Map::new()) } else { raw_input.clone() };
            tool.raw_input = to_raw_input(Some(raw_input));
        }

        status = update.get("status").and_then(Value::as_str);
        if status.is_some() {
            tool.is_input_complete = status_indicates_complete(status);
        }

        if let Some(current_name) = normalize_agent_name(Some(&tool.name)) {
            tool_name_override = Some(current_name);
        }

        if let Some(raw_output) = update.get("rawOutput") {
            output = Some(raw_output.clone());
            result = parse_tool_result_dto(
                raw_output,
                tool_call_id,
                tool_name_override
                    .as_deref()
                    .or_else(|| update.get("kind").and_then(Value::as_str)),
            );
        }
    }

    if update.contains_key("rawOutput")
        || update.contains_key("status")
        || update.contains_key("title")
        || update.contains_key("kind")
    {
        let content = result
            .as_ref()
            .map(to_tool_result_content_from_dto)
            .or_else(|| output.as_ref().map(to_tool_result_content));
        let is_error = status
            .map(|status| status_indicates_error(Some(status)))
            .or_else(|| result.as_ref().and_then(|result| result.success).map(|success| !success));
        upsert_tool_result(
            agent,
            tool_call_id,
            tool_name_override,
            is_error,
            content,
            output,
            result,
        );
    }
}

fn as_record(value: Option<&Value>) -> Option<&Map<String, Value>> {
    value?.as_object()
}

fn number_field(source: &Map<String, Value>, keys: &[&str]) -> Option<i64> {
    for key in keys {
        let Some(value) = source.get(*key) else {
            continue;
        };
        if let Some(value) = value.as_i64()
            && value >= 0
        {
            return Some(value);
        }
        if let Some(value) = value.as_u64()
            && let Ok(value) = i64::try_from(value)
        {
            return Some(value);
        }
    }
    None
}

fn usage_to_token_usage(update: &Map<String, Value>) -> Option<SessionTokenUsage> {
    let usage_meta = as_record(update.get("_meta")).and_then(|meta| as_record(meta.get("usage")));
    let source = usage_meta.unwrap_or(update);
    let normalized = SessionTokenUsage {
        input_tokens: number_field(source, &["input_tokens", "inputTokens"]),
        output_tokens: number_field(source, &["output_tokens", "outputTokens"]),
        cache_creation_input_tokens: number_field(
            source,
            &["cache_creation_input_tokens", "cacheCreationInputTokens", "cachedWriteTokens"],
        ),
        cache_read_input_tokens: number_field(
            source,
            &["cache_read_input_tokens", "cacheReadInputTokens", "cachedReadTokens"],
        ),
    };

    if normalized.input_tokens.is_none()
        && normalized.output_tokens.is_none()
        && normalized.cache_creation_input_tokens.is_none()
        && normalized.cache_read_input_tokens.is_none()
    {
        return None;
    }

    Some(normalized)
}

fn last_user_message_id(conversation: &SessionConversation) -> Option<String> {
    conversation.messages.iter().rev().find_map(|message| match message {
        SessionMessage::User(user) => Some(user.id.clone()),
        _ => None,
    })
}

fn ensure_vwacp_state(state: Option<&SessionAcpxState>) -> SessionAcpxState {
    state.cloned().unwrap_or(SessionAcpxState {
        current_mode_id: None,
        desired_mode_id: None,
        current_model_id: None,
        available_models: None,
        available_commands: None,
        config_options: None,
        desired_config_options: None,
        session_options: None,
    })
}

pub fn create_session_conversation(timestamp: Option<&str>) -> SessionConversation {
    SessionConversation {
        title: None,
        messages: Vec::new(),
        updated_at: timestamp.map(ToOwned::to_owned).unwrap_or_else(now_iso),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: HashMap::new(),
    }
}

pub fn clone_session_conversation(
    conversation: Option<&SessionConversation>,
) -> SessionConversation {
    conversation.cloned().unwrap_or_else(|| create_session_conversation(None))
}

pub fn clone_session_vwacp_state(state: Option<&SessionAcpxState>) -> Option<SessionAcpxState> {
    state.cloned()
}

pub fn append_legacy_history(
    conversation: &mut SessionConversation,
    entries: &[LegacyHistoryEntry],
) {
    for entry in entries {
        let text = entry.text_preview.trim();
        if text.is_empty() {
            continue;
        }

        match entry.role {
            LegacyHistoryRole::User => {
                conversation.messages.push(SessionMessage::User(SessionUserMessage {
                    id: next_user_message_id(),
                    content: vec![SessionUserContent::Text(text.to_string())],
                }))
            }
            LegacyHistoryRole::Assistant => {
                conversation.messages.push(SessionMessage::Agent(SessionAgentMessage {
                    content: vec![SessionAgentContent::Text(text.to_string())],
                    tool_results: HashMap::new(),
                    reasoning_details: None,
                }))
            }
        }

        update_conversation_timestamp(conversation, &entry.timestamp);
    }
}

pub fn record_prompt_submission(
    conversation: &mut SessionConversation,
    prompt: &PromptInput,
    timestamp: Option<&str>,
) {
    let user_content: Vec<SessionUserContent> = prompt
        .iter()
        .filter_map(content_to_user_content)
        .map(|content| match content {
            SessionUserContent::Text(text) => {
                SessionUserContent::Text(trim_runtime_text(&text, MAX_RUNTIME_AGENT_TEXT_CHARS))
            }
            other => other,
        })
        .collect();

    if user_content.is_empty() {
        return;
    }

    conversation.messages.push(SessionMessage::User(SessionUserMessage {
        id: next_user_message_id(),
        content: user_content,
    }));
    let effective_timestamp = timestamp.map(ToOwned::to_owned).unwrap_or_else(now_iso);
    update_conversation_timestamp(conversation, &effective_timestamp);
    trim_conversation_for_runtime(conversation);
}

pub fn record_text_prompt_submission(
    conversation: &mut SessionConversation,
    prompt: &str,
    timestamp: Option<&str>,
) {
    if prompt.trim().is_empty() {
        return;
    }

    let input = text_prompt(prompt);
    record_prompt_submission(conversation, &input, timestamp);
}

pub fn record_session_update(
    conversation: &mut SessionConversation,
    state: Option<&SessionAcpxState>,
    notification: &SessionNotification,
    timestamp: Option<&str>,
) -> SessionAcpxState {
    let mut vwacp = ensure_vwacp_state(state);
    let Some(update_value) = serde_json::to_value(&notification.update).ok() else {
        let effective_timestamp =
            timestamp.map(ToOwned::to_owned).unwrap_or_else(|| conversation.updated_at.clone());
        update_conversation_timestamp(conversation, &effective_timestamp);
        trim_conversation_for_runtime(conversation);
        return vwacp;
    };
    let Some(update) = update_value.as_object() else {
        let effective_timestamp =
            timestamp.map(ToOwned::to_owned).unwrap_or_else(|| conversation.updated_at.clone());
        update_conversation_timestamp(conversation, &effective_timestamp);
        trim_conversation_for_runtime(conversation);
        return vwacp;
    };

    match update.get("sessionUpdate").and_then(Value::as_str) {
        Some("user_message_chunk") => {
            if let Some(content_value) = update.get("content")
                && let Ok(content) = serde_json::from_value::<ContentBlock>(content_value.clone())
                && let Some(user_content) = content_to_user_content(&content)
            {
                conversation.messages.push(SessionMessage::User(SessionUserMessage {
                    id: next_user_message_id(),
                    content: vec![user_content],
                }));
            }
        }
        Some("agent_message_chunk") => {
            if let Some(content_value) = update.get("content")
                && let Ok(content) = serde_json::from_value::<ContentBlock>(content_value.clone())
                && let Some(text) = extract_text(&content)
                && !text.trim().is_empty()
            {
                let agent = ensure_agent_message(conversation);
                append_agent_text(agent, &text);
            }
        }
        Some("agent_thought_chunk") => {
            if let Some(content_value) = update.get("content")
                && let Ok(content) = serde_json::from_value::<ContentBlock>(content_value.clone())
                && let Some(text) = extract_text(&content)
                && !text.trim().is_empty()
            {
                let agent = ensure_agent_message(conversation);
                append_agent_thinking(agent, &text);
            }
        }
        Some("tool_call") | Some("tool_call_update") => {
            let agent = ensure_agent_message(conversation);
            apply_tool_call_update(agent, update);
        }
        Some("usage_update") => {
            if let Some(usage) = usage_to_token_usage(update) {
                conversation.cumulative_token_usage = usage.clone();
                if let Some(user_id) = last_user_message_id(conversation) {
                    conversation.request_token_usage.insert(user_id, usage);
                }
            }
        }
        Some("session_info_update") => {
            if update.contains_key("title") {
                conversation.title =
                    update.get("title").and_then(Value::as_str).map(ToOwned::to_owned);
            }
            if let Some(updated_at) = update.get("updatedAt").and_then(Value::as_str) {
                conversation.updated_at = updated_at.to_string();
            }
        }
        Some("available_commands_update") => {
            let names = update
                .get("availableCommands")
                .and_then(Value::as_array)
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|entry| entry.as_object())
                        .filter_map(|entry| entry.get("name").and_then(Value::as_str))
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            vwacp.available_commands = Some(names);
        }
        Some("current_mode_update") => {
            vwacp.current_mode_id =
                update.get("currentModeId").and_then(Value::as_str).map(ToOwned::to_owned);
        }
        Some("config_option_update") => {
            vwacp.config_options = update
                .get("configOptions")
                .cloned()
                .and_then(|value| serde_json::from_value(value).ok());
        }
        _ => {}
    }

    let effective_timestamp =
        timestamp.map(ToOwned::to_owned).unwrap_or_else(|| conversation.updated_at.clone());
    update_conversation_timestamp(conversation, &effective_timestamp);
    trim_conversation_for_runtime(conversation);
    vwacp
}

pub fn record_client_operation(
    conversation: &mut SessionConversation,
    state: Option<&SessionAcpxState>,
    _operation: &ClientOperation,
    timestamp: Option<&str>,
) -> SessionAcpxState {
    let vwacp = ensure_vwacp_state(state);
    let effective_timestamp = timestamp.map(ToOwned::to_owned).unwrap_or_else(now_iso);
    update_conversation_timestamp(conversation, &effective_timestamp);
    trim_conversation_for_runtime(conversation);
    vwacp
}

pub fn trim_conversation_for_runtime(conversation: &mut SessionConversation) {
    if conversation.messages.len() > MAX_RUNTIME_MESSAGES {
        let keep_from = conversation.messages.len() - MAX_RUNTIME_MESSAGES;
        conversation.messages = conversation.messages.split_off(keep_from);
    }

    for message in &mut conversation.messages {
        match message {
            SessionMessage::User(user) => {
                for content in &mut user.content {
                    if let SessionUserContent::Text(text) = content {
                        *text = trim_runtime_text(text, MAX_RUNTIME_AGENT_TEXT_CHARS);
                    }
                }
            }
            SessionMessage::Agent(agent) => {
                for content in &mut agent.content {
                    match content {
                        SessionAgentContent::Text(text) => {
                            *text = trim_runtime_text(text, MAX_RUNTIME_AGENT_TEXT_CHARS);
                        }
                        SessionAgentContent::Thinking(thinking) => {
                            thinking.text =
                                trim_runtime_text(&thinking.text, MAX_RUNTIME_THINKING_CHARS);
                        }
                        SessionAgentContent::ToolUse(tool_use) => {
                            tool_use.raw_input =
                                trim_runtime_text(&tool_use.raw_input, MAX_RUNTIME_TOOL_IO_CHARS);
                        }
                        SessionAgentContent::RedactedThinking(_) => {}
                    }
                }

                for result in agent.tool_results.values_mut() {
                    if let SessionToolResultContent::Text(text) = &mut result.content {
                        *text = trim_runtime_text(text, MAX_RUNTIME_TOOL_IO_CHARS);
                    }
                    if let Some(Value::String(output)) = &mut result.output {
                        *output = trim_runtime_text(output, MAX_RUNTIME_TOOL_IO_CHARS);
                    }
                }
            }
            SessionMessage::Resume => {}
        }
    }

    if conversation.request_token_usage.len() > MAX_RUNTIME_REQUEST_TOKEN_USAGE {
        let mut entries: Vec<(String, SessionTokenUsage)> = conversation
            .request_token_usage
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        let keep_from = entries.len() - MAX_RUNTIME_REQUEST_TOKEN_USAGE;
        conversation.request_token_usage =
            entries.into_iter().skip(keep_from).collect::<HashMap<_, _>>();
    }
}

#[cfg(test)]
#[path = "session_conversation_model_tests.rs"]
mod session_conversation_model_tests;

//! 磁盘会话记录的 JSON 解析与兼容转换。

use std::collections::HashMap;
use std::path::PathBuf;

use agent_client_protocol::{AgentCapabilities, SessionConfigOption};
use serde_json::{Map, Value};

use crate::{
    DEFAULT_EVENT_MAX_SEGMENTS, DEFAULT_EVENT_SEGMENT_MAX_BYTES, SESSION_RECORD_SCHEMA,
    SessionAcpxState, SessionConversation, SessionEventLog, SessionMessage, SessionRecord,
    SessionStateOptions, SessionTokenUsage, default_session_event_log,
    normalize_runtime_session_id, session_event_log,
};

#[cfg(test)]
#[path = "parse_tests.rs"]
mod parse_tests;

fn as_record(value: &Value) -> Option<&Map<String, Value>> {
    match value {
        Value::Object(record) => Some(record),
        _ => None,
    }
}

fn is_string_array(value: &Value) -> Option<Vec<String>> {
    let entries = value.as_array()?;
    entries.iter().map(Value::as_str).map(|entry| entry.map(ToOwned::to_owned)).collect()
}

fn parse_non_negative_number(record: &Map<String, Value>, key: &str) -> Option<Option<i64>> {
    let Some(value) = record.get(key) else {
        return Some(None);
    };
    let number = value.as_i64()?;
    if number < 0 {
        return None;
    }
    Some(Some(number))
}

fn parse_token_usage(raw: Option<&Value>) -> Option<SessionTokenUsage> {
    let Some(raw) = raw else {
        return Some(SessionTokenUsage::default());
    };
    if raw.is_null() {
        return Some(SessionTokenUsage::default());
    }

    let record = as_record(raw)?;
    Some(SessionTokenUsage {
        input_tokens: parse_non_negative_number(record, "input_tokens")?,
        output_tokens: parse_non_negative_number(record, "output_tokens")?,
        cache_creation_input_tokens: parse_non_negative_number(
            record,
            "cache_creation_input_tokens",
        )?,
        cache_read_input_tokens: parse_non_negative_number(record, "cache_read_input_tokens")?,
    })
}

fn parse_request_token_usage(raw: Option<&Value>) -> Option<HashMap<String, SessionTokenUsage>> {
    let Some(raw) = raw else {
        return Some(HashMap::new());
    };
    if raw.is_null() {
        return Some(HashMap::new());
    }

    let record = as_record(raw)?;
    let mut usage = HashMap::with_capacity(record.len());
    for (key, value) in record {
        usage.insert(key.clone(), parse_token_usage(Some(value))?);
    }
    Some(usage)
}

fn parse_conversation_record(record: &Map<String, Value>) -> Option<SessionConversation> {
    let messages_value = record.get("messages")?;
    let updated_at = record.get("updated_at")?.as_str()?.to_string();
    let messages = messages_value
        .as_array()?
        .iter()
        .map(|entry| serde_json::from_value::<SessionMessage>(entry.clone()).ok())
        .collect::<Option<Vec<_>>>()?;

    let title = match record.get("title") {
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Null) | None => None,
        _ => return None,
    };

    Some(SessionConversation {
        title,
        messages,
        updated_at,
        cumulative_token_usage: parse_token_usage(record.get("cumulative_token_usage"))?,
        request_token_usage: parse_request_token_usage(record.get("request_token_usage"))?,
    })
}

fn parse_vwacp_state(raw: Option<&Value>) -> Option<Option<SessionAcpxState>> {
    let Some(raw) = raw else {
        return Some(None);
    };

    let record = as_record(raw)?;
    let mut state = SessionAcpxState {
        current_mode_id: None,
        desired_mode_id: None,
        current_model_id: None,
        available_models: None,
        available_commands: None,
        config_options: None,
        desired_config_options: None,
        session_options: None,
    };

    if let Some(current_mode_id) = record.get("current_mode_id").and_then(Value::as_str) {
        state.current_mode_id = Some(current_mode_id.to_string());
    }
    if let Some(desired_mode_id) = record.get("desired_mode_id").and_then(Value::as_str) {
        state.desired_mode_id = Some(desired_mode_id.to_string());
    }
    if let Some(current_model_id) = record.get("current_model_id").and_then(Value::as_str) {
        state.current_model_id = Some(current_model_id.to_string());
    }
    if let Some(available_models) = record.get("available_models").and_then(is_string_array) {
        state.available_models = Some(available_models);
    }
    if let Some(available_commands) = record.get("available_commands").and_then(is_string_array) {
        state.available_commands = Some(available_commands);
    }
    if let Some(config_options) = record.get("config_options")
        && config_options.is_array()
    {
        state.config_options =
            serde_json::from_value::<Vec<SessionConfigOption>>(config_options.clone()).ok();
    }
    if let Some(desired_config_options) = record.get("desired_config_options") {
        state.desired_config_options = serde_json::from_value(desired_config_options.clone()).ok();
    }
    if let Some(session_options) = record.get("session_options").and_then(as_record) {
        let model = session_options.get("model").and_then(Value::as_str).map(ToOwned::to_owned);
        let allowed_tools = session_options.get("allowed_tools").and_then(is_string_array);
        let max_turns =
            session_options.get("max_turns").and_then(Value::as_i64).filter(|value| *value > 0);

        if model.is_some() || allowed_tools.is_some() || max_turns.is_some() {
            state.session_options = Some(SessionStateOptions { model, allowed_tools, max_turns });
        }
    }

    Some(Some(state))
}

fn fallback_event_log(session_id: &str) -> SessionEventLog {
    default_session_event_log(session_id).unwrap_or_else(|| SessionEventLog {
        active_path: session_event_log(session_id, PathBuf::new()).active_path,
        segment_count: DEFAULT_EVENT_MAX_SEGMENTS,
        max_segment_bytes: DEFAULT_EVENT_SEGMENT_MAX_BYTES,
        max_segments: DEFAULT_EVENT_MAX_SEGMENTS,
        last_write_at: None,
        last_write_error: None,
    })
}

fn parse_event_log(raw: Option<&Value>, session_id: &str) -> SessionEventLog {
    let Some(raw) = raw else {
        return fallback_event_log(session_id);
    };

    let Some(record) = as_record(raw) else {
        return fallback_event_log(session_id);
    };

    let Some(active_path) = record.get("active_path").and_then(Value::as_str) else {
        return fallback_event_log(session_id);
    };
    let Some(segment_count) = record.get("segment_count").and_then(Value::as_i64) else {
        return fallback_event_log(session_id);
    };
    let Some(max_segment_bytes) = record.get("max_segment_bytes").and_then(Value::as_i64) else {
        return fallback_event_log(session_id);
    };
    let Some(max_segments) = record.get("max_segments").and_then(Value::as_i64) else {
        return fallback_event_log(session_id);
    };

    if segment_count < 1 || max_segment_bytes < 1 || max_segments < 1 {
        return fallback_event_log(session_id);
    }

    let last_write_at = match record.get("last_write_at") {
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Null) | None => None,
        _ => None,
    };
    let last_write_error = match record.get("last_write_error") {
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Null) | None => None,
        _ => None,
    };

    SessionEventLog {
        active_path: active_path.to_string(),
        segment_count,
        max_segment_bytes,
        max_segments,
        last_write_at,
        last_write_error,
    }
}

fn normalize_optional_name(value: Option<&Value>) -> Option<Option<String>> {
    match value {
        None | Some(Value::Null) => Some(None),
        Some(Value::String(value)) => {
            let trimmed = value.trim();
            if trimmed.is_empty() { Some(None) } else { Some(Some(trimmed.to_string())) }
        }
        _ => None,
    }
}

fn normalize_optional_pid(value: Option<&Value>) -> Option<Option<u32>> {
    match value {
        None | Some(Value::Null) => Some(None),
        Some(number) => {
            let pid = number.as_u64()?;
            if pid == 0 {
                return None;
            }
            u32::try_from(pid).ok().map(Some)
        }
    }
}

fn normalize_optional_boolean(value: Option<&Value>, fallback: bool) -> Option<bool> {
    match value {
        None | Some(Value::Null) => Some(fallback),
        Some(Value::Bool(value)) => Some(*value),
        _ => None,
    }
}

fn normalize_optional_string(value: Option<&Value>) -> Option<Option<String>> {
    match value {
        None | Some(Value::Null) => Some(None),
        Some(Value::String(value)) => Some(Some(value.clone())),
        _ => None,
    }
}

fn normalize_optional_exit_code(value: Option<&Value>) -> Option<Option<i32>> {
    match value {
        None => Some(None),
        Some(Value::Null) => Some(None),
        Some(value) => {
            let exit_code = value.as_i64()?;
            i32::try_from(exit_code).ok().map(Some)
        }
    }
}

fn normalize_optional_agent_capabilities(
    value: Option<&Value>,
) -> Option<Option<AgentCapabilities>> {
    match value {
        None | Some(Value::Null) => Some(None),
        Some(Value::Object(_)) => {
            serde_json::from_value::<AgentCapabilities>(value?.clone()).ok().map(Some)
        }
        _ => None,
    }
}

pub fn parse_session_record(raw: &Value) -> Option<SessionRecord> {
    let record = as_record(raw)?;
    if record.get("schema").and_then(Value::as_str) != Some(SESSION_RECORD_SCHEMA) {
        return None;
    }

    let vwacp_record_id = record.get("vwacp_record_id")?.as_str()?.to_string();
    let acp_session_id = record.get("acp_session_id")?.as_str()?.to_string();
    let agent_command = record.get("agent_command")?.as_str()?.to_string();
    let cwd = record.get("cwd")?.as_str()?.to_string();
    let created_at = record.get("created_at")?.as_str()?.to_string();
    let last_used_at = record.get("last_used_at")?.as_str()?.to_string();
    let last_seq = record.get("last_seq")?.as_i64()?;
    if last_seq < 0 {
        return None;
    }

    let name = normalize_optional_name(record.get("name"))?;
    let pid = normalize_optional_pid(record.get("pid"))?;
    let closed = Some(normalize_optional_boolean(record.get("closed"), false)?);
    let closed_at = normalize_optional_string(record.get("closed_at"))?;
    let agent_started_at = normalize_optional_string(record.get("agent_started_at"))?;
    let last_prompt_at = normalize_optional_string(record.get("last_prompt_at"))?;
    let last_agent_exit_code = normalize_optional_exit_code(record.get("last_agent_exit_code"))?;
    let last_agent_exit_signal = normalize_optional_string(record.get("last_agent_exit_signal"))?;
    let last_agent_exit_at = normalize_optional_string(record.get("last_agent_exit_at"))?;
    let last_agent_disconnect_reason =
        normalize_optional_string(record.get("last_agent_disconnect_reason"))?;
    let last_request_id = normalize_optional_string(record.get("last_request_id"))?;

    let conversation = parse_conversation_record(record)?;
    let event_log = parse_event_log(record.get("event_log"), &vwacp_record_id);
    let protocol_version = record.get("protocol_version").and_then(Value::as_i64);
    let agent_capabilities =
        normalize_optional_agent_capabilities(record.get("agent_capabilities"))?;
    let vwacp = parse_vwacp_state(record.get("vwacp"))?;

    Some(SessionRecord {
        schema: SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id,
        acp_session_id,
        agent_session_id: record.get("agent_session_id").and_then(normalize_runtime_session_id),
        agent_command,
        agent_config: record
            .get("agent_config")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok()),
        cwd,
        name,
        created_at,
        last_used_at,
        last_seq,
        last_request_id,
        event_log,
        closed,
        closed_at,
        pid,
        agent_started_at,
        last_prompt_at,
        last_agent_exit_code,
        last_agent_exit_signal,
        last_agent_exit_at,
        last_agent_disconnect_reason,
        protocol_version,
        agent_capabilities,
        title: conversation.title,
        messages: conversation.messages,
        updated_at: conversation.updated_at,
        cumulative_token_usage: conversation.cumulative_token_usage,
        request_token_usage: conversation.request_token_usage,
        vwacp,
    })
}

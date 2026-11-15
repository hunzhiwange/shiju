//! ACP 错误结构的提取、识别与格式化辅助。

use serde_json::Value;

use crate::types::OutputErrorAcpPayload;

const RESOURCE_NOT_FOUND_ACP_CODES: [i64; 2] = [-32001, -32002];

fn as_record(value: &Value) -> Option<&serde_json::Map<String, Value>> {
    match value {
        Value::Object(record) => Some(record),
        _ => None,
    }
}

fn to_acp_error_payload(value: &Value) -> Option<OutputErrorAcpPayload> {
    let record = as_record(value)?;
    let code = record.get("code")?.as_i64()?;
    let message = record.get("message")?.as_str()?.trim();
    if message.is_empty() {
        return None;
    }

    Some(OutputErrorAcpPayload {
        code,
        message: message.to_string(),
        data: record.get("data").cloned(),
    })
}

fn extract_acp_error_internal(value: &Value, depth: usize) -> Option<OutputErrorAcpPayload> {
    if depth > 5 {
        return None;
    }

    if let Some(direct) = to_acp_error_payload(value) {
        return Some(direct);
    }

    let record = as_record(value)?;
    for key in ["error", "acp", "cause"] {
        if let Some(nested) =
            record.get(key).and_then(|entry| extract_acp_error_internal(entry, depth + 1))
        {
            return Some(nested);
        }
    }

    None
}

pub fn format_unknown_error_message(value: &Value) -> String {
    match value {
        Value::String(text) if !text.is_empty() => text.clone(),
        Value::Object(record) => record
            .get("message")
            .and_then(Value::as_str)
            .filter(|message| !message.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| value.to_string()),
        _ => value.to_string(),
    }
}

fn session_not_found_pattern(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let Some(session_index) = lower.find("session") else {
        return false;
    };
    let tail = lower[session_index + "session".len()..].trim_start();
    if tail.is_empty() {
        return false;
    }

    let Some(not_found_index) = tail.find("not found") else {
        return false;
    };
    let between = tail[..not_found_index].trim();
    if between.is_empty() {
        return false;
    }

    let between = between.trim_matches(|ch| matches!(ch, '"' | '\'' | '`'));
    !between.is_empty()
        && between.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
}

fn is_session_not_found_text(value: &Value) -> bool {
    let Some(text) = value.as_str() else {
        return false;
    };

    let normalized = text.to_ascii_lowercase();
    normalized.contains("resource_not_found")
        || normalized.contains("resource not found")
        || normalized.contains("session not found")
        || normalized.contains("unknown session")
        || normalized.contains("invalid session identifier")
        || session_not_found_pattern(text)
}

fn has_session_not_found_hint(value: &Value, depth: usize) -> bool {
    if depth > 4 {
        return false;
    }

    if is_session_not_found_text(value) {
        return true;
    }

    match value {
        Value::Array(entries) => {
            entries.iter().any(|entry| has_session_not_found_hint(entry, depth + 1))
        }
        Value::Object(record) => {
            record.values().any(|entry| has_session_not_found_hint(entry, depth + 1))
        }
        _ => false,
    }
}

pub fn extract_acp_error(error: &Value) -> Option<OutputErrorAcpPayload> {
    extract_acp_error_internal(error, 0)
}

pub fn is_acp_resource_not_found_error(error: &Value) -> bool {
    if let Some(acp) = extract_acp_error(error) {
        if RESOURCE_NOT_FOUND_ACP_CODES.contains(&acp.code) {
            return true;
        }
        if is_session_not_found_text(&Value::String(acp.message.clone())) {
            return true;
        }
        if acp.data.as_ref().is_some_and(|data| has_session_not_found_hint(data, 0)) {
            return true;
        }
    }

    is_session_not_found_text(&Value::String(format_unknown_error_message(error)))
        || has_session_not_found_hint(error, 0)
}

#[cfg(test)]
#[path = "acp_error_shapes_tests.rs"]
mod acp_error_shapes_tests;

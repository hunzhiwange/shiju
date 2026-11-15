//! Agent 会话标识的提取与标准化。

use serde_json::{Map, Value};

pub const AGENT_SESSION_ID_META_KEYS: &[&str] = &["agentSessionId", "sessionId"];

pub fn normalize_agent_session_id(value: &Value) -> Option<String> {
    let trimmed = value.as_str()?.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn as_meta_record(meta: &Value) -> Option<&Map<String, Value>> {
    match meta {
        Value::Object(record) => Some(record),
        _ => None,
    }
}

pub fn extract_agent_session_id(meta: &Value) -> Option<String> {
    let record = as_meta_record(meta)?;
    for key in AGENT_SESSION_ID_META_KEYS {
        if let Some(normalized) = record.get(*key).and_then(normalize_agent_session_id) {
            return Some(normalized);
        }
    }
    None
}

#[cfg(test)]
#[path = "agent_session_id_tests.rs"]
mod agent_session_id_tests;

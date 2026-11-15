//! 会话记录序列化为磁盘格式的辅助逻辑。

use serde_json::{Map, Value, json};

use crate::{SESSION_RECORD_SCHEMA, SessionRecord, normalize_runtime_session_id};

#[cfg(test)]
#[path = "serialize_tests.rs"]
mod serialize_tests;

fn insert_optional<T>(record: &mut Map<String, Value>, key: &str, value: Option<&T>)
where
    T: serde::Serialize,
{
    if let Some(value) = value {
        record.insert(key.to_string(), serde_json::to_value(value).unwrap_or(Value::Null));
    }
}

pub fn serialize_session_record_for_disk(record: &SessionRecord) -> Value {
    let mut serialized = Map::new();

    serialized.insert("schema".to_string(), Value::String(SESSION_RECORD_SCHEMA.to_string()));
    serialized.insert("vwacp_record_id".to_string(), Value::String(record.vwacp_record_id.clone()));
    serialized.insert("acp_session_id".to_string(), Value::String(record.acp_session_id.clone()));
    if let Some(agent_session_id) = record
        .agent_session_id
        .as_ref()
        .and_then(|value| normalize_runtime_session_id(&Value::String(value.clone())))
    {
        serialized.insert("agent_session_id".to_string(), Value::String(agent_session_id));
    }
    // Runtime launch environments must never become executable recovery material.
    if let Some(reference) =
        record.agent_config.as_ref().and_then(|config| config.env.get("VW_ACP_LAUNCH_REFERENCE"))
    {
        serialized.insert("agent_command".into(), Value::String(reference.clone()));
    } else {
        serialized.insert("agent_command".to_string(), Value::String(record.agent_command.clone()));
        insert_optional(&mut serialized, "agent_config", record.agent_config.as_ref());
    }
    serialized.insert("cwd".to_string(), Value::String(record.cwd.clone()));
    insert_optional(&mut serialized, "name", record.name.as_ref());
    serialized.insert("created_at".to_string(), Value::String(record.created_at.clone()));
    serialized.insert("last_used_at".to_string(), Value::String(record.last_used_at.clone()));
    serialized.insert("last_seq".to_string(), json!(record.last_seq));
    insert_optional(&mut serialized, "last_request_id", record.last_request_id.as_ref());
    serialized.insert(
        "event_log".to_string(),
        serde_json::to_value(&record.event_log).unwrap_or(Value::Null),
    );
    insert_optional(&mut serialized, "closed", record.closed.as_ref());
    insert_optional(&mut serialized, "closed_at", record.closed_at.as_ref());
    insert_optional(&mut serialized, "pid", record.pid.as_ref());
    insert_optional(&mut serialized, "agent_started_at", record.agent_started_at.as_ref());
    insert_optional(&mut serialized, "last_prompt_at", record.last_prompt_at.as_ref());
    insert_optional(&mut serialized, "last_agent_exit_code", record.last_agent_exit_code.as_ref());
    insert_optional(
        &mut serialized,
        "last_agent_exit_signal",
        record.last_agent_exit_signal.as_ref(),
    );
    insert_optional(&mut serialized, "last_agent_exit_at", record.last_agent_exit_at.as_ref());
    insert_optional(
        &mut serialized,
        "last_agent_disconnect_reason",
        record.last_agent_disconnect_reason.as_ref(),
    );
    insert_optional(&mut serialized, "protocol_version", record.protocol_version.as_ref());
    insert_optional(&mut serialized, "agent_capabilities", record.agent_capabilities.as_ref());
    insert_optional(&mut serialized, "title", record.title.as_ref());
    serialized.insert(
        "messages".to_string(),
        serde_json::to_value(&record.messages).unwrap_or(Value::Null),
    );
    serialized.insert("updated_at".to_string(), Value::String(record.updated_at.clone()));
    serialized.insert(
        "cumulative_token_usage".to_string(),
        serde_json::to_value(&record.cumulative_token_usage).unwrap_or(Value::Null),
    );
    serialized.insert(
        "request_token_usage".to_string(),
        serde_json::to_value(&record.request_token_usage).unwrap_or(Value::Null),
    );
    insert_optional(&mut serialized, "vwacp", record.vwacp.as_ref());

    Value::Object(serialized)
}

//! 验证会话记录落盘时的蛇形字段和可选字段省略契约。

use super::*;
use crate::{
    SESSION_RECORD_SCHEMA, SessionAcpxState, SessionEventLog, SessionRecord, SessionStateOptions,
    SessionTokenUsage,
};

fn test_record() -> SessionRecord {
    SessionRecord {
        schema: "ignored".to_string(),
        vwacp_record_id: "record-1".to_string(),
        acp_session_id: "acp-1".to_string(),
        agent_session_id: Some("agent-1".to_string()),
        agent_command: "agent".to_string(),
        agent_config: None,
        cwd: "/tmp/project".to_string(),
        name: Some("main".to_string()),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_used_at: "2026-01-02T00:00:00Z".to_string(),
        last_seq: 2,
        last_request_id: None,
        event_log: SessionEventLog {
            active_path: "/tmp/active.ndjson".to_string(),
            segment_count: 1,
            max_segment_bytes: 1024,
            max_segments: 3,
            last_write_at: None,
            last_write_error: None,
        },
        closed: Some(false),
        closed_at: None,
        pid: Some(42),
        agent_started_at: None,
        last_prompt_at: None,
        last_agent_exit_code: None,
        last_agent_exit_signal: None,
        last_agent_exit_at: None,
        last_agent_disconnect_reason: None,
        protocol_version: None,
        agent_capabilities: None,
        title: None,
        messages: Vec::new(),
        updated_at: "2026-01-02T00:00:00Z".to_string(),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: std::collections::HashMap::new(),
        vwacp: Some(SessionAcpxState {
            current_mode_id: None,
            desired_mode_id: Some("plan".to_string()),
            current_model_id: None,
            available_models: None,
            available_commands: None,
            config_options: None,
            desired_config_options: None,
            session_options: Some(SessionStateOptions {
                model: Some("gpt".to_string()),
                allowed_tools: None,
                max_turns: None,
            }),
        }),
    }
}

#[test]
fn insert_optional_writes_some_values_and_skips_none() {
    let mut record = Map::new();

    insert_optional(&mut record, "name", Some(&"main"));
    insert_optional::<String>(&mut record, "missing", None);

    assert_eq!(record.get("name").and_then(Value::as_str), Some("main"));
    assert!(!record.contains_key("missing"));
}

#[test]
fn serialize_session_record_for_disk_writes_snake_case_disk_shape() {
    let serialized = serialize_session_record_for_disk(&test_record());
    let record = serialized.as_object().expect("object");

    assert_eq!(record.get("schema").and_then(Value::as_str), Some(SESSION_RECORD_SCHEMA));
    assert_eq!(record.get("vwacp_record_id").and_then(Value::as_str), Some("record-1"));
    assert_eq!(record.get("agent_session_id").and_then(Value::as_str), Some("agent-1"));
    assert_eq!(record.get("name").and_then(Value::as_str), Some("main"));
    assert_eq!(record.get("pid").and_then(Value::as_u64), Some(42));
    assert!(!record.contains_key("closed_at"));
    assert_eq!(record["vwacp"]["desired_mode_id"].as_str(), Some("plan"));
}

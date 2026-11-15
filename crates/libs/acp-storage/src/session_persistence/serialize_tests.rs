//! 验证会话记录落盘时的蛇形字段和可选字段省略契约。
//!
//! 临时目录和最小记录 fixture 用于验证磁盘格式，不共享真实用户会话数据。

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

// 下列行为用例覆盖合法往返、损坏输入、路径边界和索引恢复。
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

#[test]
fn acp_model_source_launch_profile_disk_cannot_restore_runtime_capability() {
    let mut record = test_record();
    record.agent_command = "agent --token runtime-secret-9".into();
    record.agent_config = Some(crate::AcpAgentConfig {
        command: "agent".into(),
        args: vec!["runtime-secret-9".into()],
        env: std::collections::HashMap::from([
            ("VW_ACP_LAUNCH_REFERENCE".into(), "vw-session-model:opaque-reference".into()),
            ("API_KEY".into(), "runtime-secret-9".into()),
        ]),
    });
    let disk = super::serialize_session_record_for_disk(&record);
    assert!(!disk.to_string().contains("runtime-secret-9"));
    assert_eq!(disk["agent_command"], "vw-session-model:opaque-reference");
    assert!(disk.get("agent_config").is_none());
}

#[test]
fn acp_model_source_launch_profile_local_config_remains_restorable() {
    let mut record = test_record();
    record.agent_config = Some(crate::AcpAgentConfig {
        command: "local-agent".into(),
        args: vec!["--local".into()],
        env: std::collections::HashMap::from([("LOCAL_SETTING".into(), "user-value".into())]),
    });
    let restored =
        crate::parse_session_record(&super::serialize_session_record_for_disk(&record)).unwrap();
    let config = restored.agent_config.unwrap();
    assert_eq!(config.command, "local-agent");
    assert_eq!(config.env["LOCAL_SETTING"], "user-value");
    let mut legacy = super::serialize_session_record_for_disk(&record);
    legacy["schema"] = crate::SESSION_RECORD_SCHEMA.into();
    legacy["agent_command"] = "agent --token revoked-token".into();
    legacy["agent_config"]["env"]["VW_ACP_LAUNCH_REFERENCE"] = "vw-session-model:legacy".into();
    let restored = crate::parse_session_record(&legacy).unwrap();
    assert!(restored.agent_config.is_none());
    assert_eq!(restored.agent_command, "vw-session-model:legacy");
}

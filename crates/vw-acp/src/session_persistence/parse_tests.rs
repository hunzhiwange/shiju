//! 验证持久化会话记录各层字段的解析、默认值与拒绝条件。
//!
//! 临时目录和最小记录 fixture 用于验证磁盘格式，不共享真实用户会话数据。

use super::*;
use crate::SESSION_RECORD_SCHEMA;

fn minimal_conversation() -> Map<String, Value> {
    serde_json::json!({
        "messages": [],
        "updated_at": "2026-01-02T00:00:00Z"
    })
    .as_object()
    .expect("conversation object")
    .clone()
}

fn valid_record() -> Value {
    serde_json::json!({
        "schema": SESSION_RECORD_SCHEMA,
        "vwacp_record_id": "record-1",
        "acp_session_id": "acp-1",
        "agent_session_id": "agent-1",
        "agent_command": "agent",
        "cwd": "/tmp/project",
        "created_at": "2026-01-01T00:00:00Z",
        "last_used_at": "2026-01-02T00:00:00Z",
        "last_seq": 2,
        "name": "  main  ",
        "closed": false,
        "conversation": "ignored",
        "messages": [],
        "updated_at": "2026-01-02T00:00:00Z",
        "cumulative_token_usage": {"input_tokens": 1, "output_tokens": 2},
        "request_token_usage": {"req-1": {"input_tokens": 3}},
        "event_log": {
            "active_path": "/tmp/active.ndjson",
            "segment_count": 1,
            "max_segment_bytes": 1024,
            "max_segments": 3,
            "last_write_at": null,
            "last_write_error": "disk full"
        },
        "vwacp": {
            "desired_mode_id": "plan",
            "available_models": ["a", "b"],
            "available_commands": ["cmd"],
            "session_options": {"model": "gpt", "allowed_tools": ["shell"], "max_turns": 4}
        }
    })
}

// 下列行为用例覆盖合法往返、损坏输入、路径边界和索引恢复。
#[test]
fn parse_non_negative_number_rejects_negative_and_non_numeric_values() {
    let record = serde_json::json!({"ok": 0, "bad": -1, "text": "1"});
    let record = record.as_object().unwrap();

    assert_eq!(parse_non_negative_number(record, "ok"), Some(Some(0)));
    assert_eq!(parse_non_negative_number(record, "missing"), Some(None));
    assert_eq!(parse_non_negative_number(record, "bad"), None);
    assert_eq!(parse_non_negative_number(record, "text"), None);
}

#[test]
fn parse_session_record_accepts_valid_record_and_normalizes_optional_fields() {
    let record = parse_session_record(&valid_record()).expect("valid record");

    assert_eq!(record.vwacp_record_id, "record-1");
    assert_eq!(record.name.as_deref(), Some("main"));
    assert_eq!(record.cumulative_token_usage.input_tokens, Some(1));
    assert_eq!(record.request_token_usage["req-1"].input_tokens, Some(3));
    assert_eq!(record.event_log.last_write_error.as_deref(), Some("disk full"));
    assert_eq!(
        record.vwacp.as_ref().unwrap().session_options.as_ref().unwrap().model.as_deref(),
        Some("gpt")
    );
}

#[test]
fn parse_session_record_rejects_wrong_schema_and_negative_sequence() {
    let mut wrong_schema = valid_record();
    wrong_schema["schema"] = Value::String("other".to_string());
    assert!(parse_session_record(&wrong_schema).is_none());

    let mut negative_seq = valid_record();
    negative_seq["last_seq"] = Value::Number((-1).into());
    assert!(parse_session_record(&negative_seq).is_none());
}

#[test]
fn normalize_optional_name_trims_blank_and_rejects_non_strings() {
    assert_eq!(
        normalize_optional_name(Some(&Value::String("  name ".to_string()))),
        Some(Some("name".to_string()))
    );
    assert_eq!(normalize_optional_name(None), Some(None));
    assert_eq!(normalize_optional_name(Some(&Value::String(" ".to_string()))), Some(None));
    assert_eq!(normalize_optional_name(Some(&Value::Bool(true))), None);
}

#[test]
fn as_record_and_string_array_reject_wrong_shapes() {
    assert!(as_record(&serde_json::json!({"ok": true})).is_some());
    assert!(as_record(&Value::Bool(true)).is_none());

    assert_eq!(
        is_string_array(&serde_json::json!(["shell", "read"])),
        Some(vec!["shell".to_string(), "read".to_string()])
    );
    assert!(is_string_array(&serde_json::json!(["shell", 1])).is_none());
    assert!(is_string_array(&Value::String("shell".to_string())).is_none());
}

#[test]
fn parse_token_usage_defaults_missing_and_null_values() {
    assert_eq!(parse_token_usage(None), Some(SessionTokenUsage::default()));
    assert_eq!(parse_token_usage(Some(&Value::Null)), Some(SessionTokenUsage::default()));

    let usage = parse_token_usage(Some(&serde_json::json!({
        "input_tokens": 1,
        "output_tokens": 2,
        "cache_creation_input_tokens": 3,
        "cache_read_input_tokens": 4
    })))
    .expect("token usage");
    assert_eq!(usage.input_tokens, Some(1));
    assert_eq!(usage.output_tokens, Some(2));
    assert_eq!(usage.cache_creation_input_tokens, Some(3));
    assert_eq!(usage.cache_read_input_tokens, Some(4));

    assert!(parse_token_usage(Some(&Value::Bool(true))).is_none());
    assert!(
        parse_token_usage(Some(&serde_json::json!({"cache_creation_input_tokens": -1}))).is_none()
    );
}

#[test]
fn parse_request_token_usage_defaults_and_rejects_malformed_entries() {
    assert_eq!(parse_request_token_usage(None), Some(HashMap::new()));
    assert_eq!(parse_request_token_usage(Some(&Value::Null)), Some(HashMap::new()));

    let usage = parse_request_token_usage(Some(&serde_json::json!({
        "req-1": {"input_tokens": 5}
    })))
    .expect("request usage");
    assert_eq!(usage["req-1"].input_tokens, Some(5));

    assert!(parse_request_token_usage(Some(&Value::Bool(false))).is_none());
    assert!(
        parse_request_token_usage(Some(&serde_json::json!({
            "req-1": {"output_tokens": -1}
        })))
        .is_none()
    );
}

#[test]
fn parse_conversation_record_accepts_optional_title_forms() {
    let mut with_title = minimal_conversation();
    with_title.insert("title".to_string(), Value::String("Work".to_string()));
    assert_eq!(
        parse_conversation_record(&with_title).expect("conversation").title.as_deref(),
        Some("Work")
    );

    let mut null_title = minimal_conversation();
    null_title.insert("title".to_string(), Value::Null);
    assert_eq!(parse_conversation_record(&null_title).expect("conversation").title, None);
}

#[test]
fn parse_conversation_record_rejects_malformed_required_values() {
    let mut invalid_title = minimal_conversation();
    invalid_title.insert("title".to_string(), Value::Bool(true));
    assert!(parse_conversation_record(&invalid_title).is_none());

    let mut invalid_messages = minimal_conversation();
    invalid_messages.insert("messages".to_string(), Value::String("bad".to_string()));
    assert!(parse_conversation_record(&invalid_messages).is_none());

    let mut invalid_message_entry = minimal_conversation();
    invalid_message_entry.insert("messages".to_string(), serde_json::json!([{"bad": true}]));
    assert!(parse_conversation_record(&invalid_message_entry).is_none());

    let mut invalid_updated_at = minimal_conversation();
    invalid_updated_at.insert("updated_at".to_string(), Value::Null);
    assert!(parse_conversation_record(&invalid_updated_at).is_none());
}

#[test]
fn parse_vwacp_state_maps_supported_fields_and_ignores_invalid_optional_parts() {
    assert_eq!(parse_vwacp_state(None), Some(None));
    assert!(parse_vwacp_state(Some(&Value::Bool(true))).is_none());

    let state = parse_vwacp_state(Some(&serde_json::json!({
        "current_mode_id": "act",
        "desired_mode_id": "plan",
        "current_model_id": "model-a",
        "available_models": ["model-a", "model-b"],
        "available_commands": ["run"],
        "config_options": [],
        "session_options": {
            "model": "gpt",
            "allowed_tools": ["shell"],
            "max_turns": 8
        }
    })))
    .expect("vwacp parse")
    .expect("vwacp state");

    assert_eq!(state.current_mode_id.as_deref(), Some("act"));
    assert_eq!(state.desired_mode_id.as_deref(), Some("plan"));
    assert_eq!(state.current_model_id.as_deref(), Some("model-a"));
    assert_eq!(
        state.available_models.as_deref(),
        Some(&["model-a".to_string(), "model-b".to_string()][..])
    );
    assert_eq!(state.available_commands.as_deref(), Some(&["run".to_string()][..]));
    assert_eq!(state.config_options.as_ref().map(Vec::len), Some(0));
    assert_eq!(
        state.session_options.as_ref().and_then(|options| options.allowed_tools.as_deref()),
        Some(&["shell".to_string()][..])
    );
    assert_eq!(state.session_options.as_ref().and_then(|options| options.max_turns), Some(8));

    let ignored = parse_vwacp_state(Some(&serde_json::json!({
        "available_models": ["model-a", 1],
        "available_commands": "run",
        "config_options": {},
        "session_options": {
            "model": 1,
            "allowed_tools": ["shell", 1],
            "max_turns": 0
        }
    })))
    .expect("vwacp parse")
    .expect("vwacp state");
    assert!(ignored.available_models.is_none());
    assert!(ignored.available_commands.is_none());
    assert!(ignored.config_options.is_none());
    assert!(ignored.session_options.is_none());
    let empty = parse_vwacp_state(Some(&serde_json::json!({}))).unwrap().unwrap();
    assert!(empty.session_options.is_none());
}

#[test]
fn parse_vwacp_state_maps_desired_config_options() {
    let state = parse_vwacp_state(Some(&serde_json::json!({
        "desired_config_options": {
            "reasoning_effort": "high",
            "temperature": "0.2"
        }
    })))
    .expect("vwacp parse")
    .expect("vwacp state");

    let desired = state.desired_config_options.expect("desired config options");
    assert_eq!(desired.get("reasoning_effort").map(String::as_str), Some("high"));
    assert_eq!(desired.get("temperature").map(String::as_str), Some("0.2"));
}

#[test]
fn parse_event_log_accepts_valid_record_and_normalizes_optional_strings() {
    let log = parse_event_log(
        Some(&serde_json::json!({
            "active_path": "/tmp/events.ndjson",
            "segment_count": 2,
            "max_segment_bytes": 4096,
            "max_segments": 4,
            "last_write_at": "2026-01-03T00:00:00Z",
            "last_write_error": null
        })),
        "session-1",
    );

    assert_eq!(log.active_path, "/tmp/events.ndjson");
    assert_eq!(log.segment_count, 2);
    assert_eq!(log.max_segment_bytes, 4096);
    assert_eq!(log.max_segments, 4);
    assert_eq!(log.last_write_at.as_deref(), Some("2026-01-03T00:00:00Z"));
    assert_eq!(log.last_write_error, None);

    let non_string_optional = parse_event_log(
        Some(&serde_json::json!({
            "active_path": "/tmp/events.ndjson",
            "segment_count": 2,
            "max_segment_bytes": 4096,
            "max_segments": 4,
            "last_write_at": false,
            "last_write_error": 1
        })),
        "session-1",
    );
    assert_eq!(non_string_optional.last_write_at, None);
    assert_eq!(non_string_optional.last_write_error, None);
}

#[test]
fn parse_event_log_falls_back_for_missing_or_invalid_record_shapes() {
    let cases = [
        None,
        Some(Value::Bool(true)),
        Some(serde_json::json!({"segment_count": 1, "max_segment_bytes": 1, "max_segments": 1})),
        Some(
            serde_json::json!({"active_path": "/tmp/a", "max_segment_bytes": 1, "max_segments": 1}),
        ),
        Some(serde_json::json!({"active_path": "/tmp/a", "segment_count": 1, "max_segments": 1})),
        Some(
            serde_json::json!({"active_path": "/tmp/a", "segment_count": 1, "max_segment_bytes": 1}),
        ),
        Some(serde_json::json!({
            "active_path": "/tmp/a",
            "segment_count": 0,
            "max_segment_bytes": 1,
            "max_segments": 1
        })),
        Some(serde_json::json!({
            "active_path": "/tmp/a",
            "segment_count": 1,
            "max_segment_bytes": 0,
            "max_segments": 1
        })),
        Some(serde_json::json!({
            "active_path": "/tmp/a",
            "segment_count": 1,
            "max_segment_bytes": 1,
            "max_segments": 0
        })),
    ];

    for raw in cases {
        let fallback = parse_event_log(raw.as_ref(), "session-fallback");
        assert!(fallback.active_path.contains("session-fallback"));
    }
}

#[cfg(not(windows))]
#[tokio::test]
async fn parse_event_log_fallback_uses_empty_home_when_default_home_is_missing() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let previous_home = std::env::var_os("HOME");
    unsafe { std::env::remove_var("HOME") };

    let fallback = parse_event_log(None, "session-no-home");
    let empty_home_log = session_event_log("session-no-home", PathBuf::new());

    match previous_home {
        Some(value) => unsafe { std::env::set_var("HOME", value) },
        None => unsafe { std::env::remove_var("HOME") },
    }

    assert_eq!(fallback.active_path, empty_home_log.active_path);
}

#[test]
fn optional_normalizers_accept_boundaries_and_reject_invalid_values() {
    assert_eq!(normalize_optional_pid(None), Some(None));
    assert_eq!(normalize_optional_pid(Some(&Value::Null)), Some(None));
    assert_eq!(normalize_optional_pid(Some(&serde_json::json!(1))), Some(Some(1)));
    assert_eq!(normalize_optional_pid(Some(&serde_json::json!(u32::MAX))), Some(Some(u32::MAX)));
    assert!(normalize_optional_pid(Some(&serde_json::json!(0))).is_none());
    assert!(normalize_optional_pid(Some(&serde_json::json!(u64::from(u32::MAX) + 1))).is_none());
    assert!(normalize_optional_pid(Some(&Value::String("1".to_string()))).is_none());

    assert_eq!(normalize_optional_boolean(None, true), Some(true));
    assert_eq!(normalize_optional_boolean(Some(&Value::Null), true), Some(true));
    assert_eq!(normalize_optional_boolean(Some(&Value::Bool(false)), true), Some(false));
    assert!(normalize_optional_boolean(Some(&Value::String("false".to_string())), true).is_none());

    assert_eq!(normalize_optional_string(None), Some(None));
    assert_eq!(normalize_optional_string(Some(&Value::Null)), Some(None));
    assert_eq!(
        normalize_optional_string(Some(&Value::String("".to_string()))),
        Some(Some("".to_string()))
    );
    assert!(normalize_optional_string(Some(&Value::Bool(false))).is_none());

    assert_eq!(normalize_optional_exit_code(None), Some(None));
    assert_eq!(normalize_optional_exit_code(Some(&Value::Null)), Some(None));
    assert_eq!(
        normalize_optional_exit_code(Some(&serde_json::json!(i32::MIN))),
        Some(Some(i32::MIN))
    );
    assert_eq!(
        normalize_optional_exit_code(Some(&serde_json::json!(i32::MAX))),
        Some(Some(i32::MAX))
    );
    assert!(
        normalize_optional_exit_code(Some(&serde_json::json!(i64::from(i32::MAX) + 1))).is_none()
    );
    assert!(normalize_optional_exit_code(Some(&Value::Bool(true))).is_none());
}

#[test]
fn normalize_optional_agent_capabilities_accepts_null_object_and_rejects_other_shapes() {
    let capabilities = serde_json::to_value(agent_client_protocol::AgentCapabilities::new())
        .expect("capabilities");

    assert_eq!(normalize_optional_agent_capabilities(None), Some(None));
    assert_eq!(normalize_optional_agent_capabilities(Some(&Value::Null)), Some(None));
    assert!(
        normalize_optional_agent_capabilities(Some(&capabilities))
            .expect("capabilities parsed")
            .is_some()
    );
    assert!(normalize_optional_agent_capabilities(Some(&Value::Bool(true))).is_none());
}

#[test]
fn parse_session_record_maps_optional_runtime_fields() {
    let mut raw = valid_record();
    raw["pid"] = serde_json::json!(42);
    raw["closed"] = Value::Bool(true);
    raw["closed_at"] = Value::String("2026-01-04T00:00:00Z".to_string());
    raw["agent_started_at"] = Value::String("2026-01-01T00:00:01Z".to_string());
    raw["last_prompt_at"] = Value::String("2026-01-02T00:00:01Z".to_string());
    raw["last_agent_exit_code"] = serde_json::json!(-15);
    raw["last_agent_exit_signal"] = Value::String("TERM".to_string());
    raw["last_agent_exit_at"] = Value::String("2026-01-05T00:00:00Z".to_string());
    raw["last_agent_disconnect_reason"] = Value::String("closed".to_string());
    raw["last_request_id"] = Value::String("req-2".to_string());
    raw["protocol_version"] = serde_json::json!(2);
    raw["agent_config"] = serde_json::json!({
        "command": "agent",
        "args": ["--fast"],
        "env": {"A": "B"}
    });
    raw["agent_capabilities"] =
        serde_json::to_value(agent_client_protocol::AgentCapabilities::new())
            .expect("capabilities");

    let record = parse_session_record(&raw).expect("record");
    assert_eq!(record.pid, Some(42));
    assert_eq!(record.closed, Some(true));
    assert_eq!(record.closed_at.as_deref(), Some("2026-01-04T00:00:00Z"));
    assert_eq!(record.agent_started_at.as_deref(), Some("2026-01-01T00:00:01Z"));
    assert_eq!(record.last_prompt_at.as_deref(), Some("2026-01-02T00:00:01Z"));
    assert_eq!(record.last_agent_exit_code, Some(-15));
    assert_eq!(record.last_agent_exit_signal.as_deref(), Some("TERM"));
    assert_eq!(record.last_agent_exit_at.as_deref(), Some("2026-01-05T00:00:00Z"));
    assert_eq!(record.last_agent_disconnect_reason.as_deref(), Some("closed"));
    assert_eq!(record.last_request_id.as_deref(), Some("req-2"));
    assert_eq!(record.protocol_version, Some(2));
    assert_eq!(
        record.agent_config.as_ref().map(|config| config.args.as_slice()),
        Some(&["--fast".to_string()][..])
    );
    assert!(record.agent_capabilities.is_some());
}

#[test]
fn parse_session_record_rejects_malformed_optional_runtime_fields() {
    for key in [
        "name",
        "pid",
        "closed_at",
        "agent_started_at",
        "last_prompt_at",
        "last_agent_exit_code",
        "last_agent_exit_signal",
        "last_agent_exit_at",
        "last_agent_disconnect_reason",
        "last_request_id",
        "agent_capabilities",
        "vwacp",
    ] {
        let mut raw = valid_record();
        raw[key] = Value::Bool(true);
        assert!(parse_session_record(&raw).is_none(), "{key} should reject bool");
    }
}

#[test]
fn parse_session_record_rejects_missing_or_malformed_required_fields() {
    assert!(parse_session_record(&Value::Null).is_none());

    for key in [
        "vwacp_record_id",
        "acp_session_id",
        "agent_command",
        "cwd",
        "created_at",
        "last_used_at",
        "last_seq",
        "messages",
        "updated_at",
    ] {
        let mut raw = valid_record();
        raw.as_object_mut().expect("object").remove(key);
        assert!(parse_session_record(&raw).is_none(), "{key} should be required");
    }

    for key in
        ["vwacp_record_id", "acp_session_id", "agent_command", "cwd", "created_at", "last_used_at"]
    {
        let mut raw = valid_record();
        raw[key] = Value::Bool(true);
        assert!(parse_session_record(&raw).is_none(), "{key} should require a string");
    }

    let mut bad_last_seq = valid_record();
    bad_last_seq["last_seq"] = Value::String("2".to_string());
    assert!(parse_session_record(&bad_last_seq).is_none());
}

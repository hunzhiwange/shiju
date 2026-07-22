//! 验证会话模式与模型偏好的归一化、持久化和 advertised state 同步。
//!
//! 模式/模型字段会被 UI、运行时和持久化记录共同读取；这些测试确保空白输入被
//! 清理，写入 desired 字段时保留其它 session option，并以 ACP 通知中的模型状态
//! 覆盖当前可用模型列表。

use std::collections::HashMap;

use agent_client_protocol::{ModelInfo, SessionModelState};
use vw_acp::{
    SESSION_RECORD_SCHEMA, SessionAcpxState, SessionEventLog, SessionRecord, SessionStateOptions,
    SessionTokenUsage, get_desired_mode_id, get_desired_model_id, normalize_mode_id,
    set_current_model_id, set_desired_mode_id, set_desired_model_id, sync_advertised_model_state,
};

/// 构造包含完整必填字段的最小 session 记录，供偏好 helper 做就地修改。
fn sample_record() -> SessionRecord {
    SessionRecord {
        schema: SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: "record-1".to_string(),
        acp_session_id: "session-1".to_string(),
        agent_session_id: None,
        agent_command: "agent".to_string(),
        agent_config: None,
        cwd: "/tmp".to_string(),
        name: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_used_at: "2026-01-01T00:00:00Z".to_string(),
        last_seq: 0,
        last_request_id: None,
        event_log: SessionEventLog {
            active_path: "/tmp/session-1.log".to_string(),
            segment_count: 1,
            max_segment_bytes: 1024,
            max_segments: 4,
            last_write_at: None,
            last_write_error: None,
        },
        closed: None,
        closed_at: None,
        pid: None,
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
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: HashMap::new(),
        vwacp: None,
    }
}

/// 模式 id 只保留非空白的规范化值。
#[test]
fn normalize_mode_id_trims_and_filters_empty_values() {
    assert_eq!(normalize_mode_id(Some("  code-review  ")), Some("code-review".to_string()));
    assert_eq!(normalize_mode_id(Some("   ")), None);
    assert_eq!(normalize_mode_id(None), None);
}

/// desired mode 写入和读取应使用同一套归一化规则。
#[test]
fn desired_mode_helpers_round_trip_normalized_value() {
    let mut record = sample_record();

    set_desired_mode_id(&mut record, Some("  focus  "));

    assert_eq!(get_desired_mode_id(record.vwacp.as_ref()), Some("focus".to_string()));
}

/// 清空 desired model 时不能丢失其它 session option，例如 allowed tools。
#[test]
fn desired_model_helpers_preserve_other_session_options() {
    let mut record = sample_record();
    record.vwacp = Some(SessionAcpxState {
        current_mode_id: None,
        desired_mode_id: None,
        current_model_id: None,
        available_models: None,
        available_commands: None,
        config_options: None,
        desired_config_options: None,
        session_options: Some(SessionStateOptions {
            model: Some("model-a".to_string()),
            allowed_tools: Some(vec!["apply_patch".to_string()]),
            max_turns: None,
        }),
    });

    set_desired_model_id(&mut record, None);

    assert_eq!(get_desired_model_id(record.vwacp.as_ref()), None);
    assert_eq!(
        record.vwacp.as_ref().and_then(|state| state.session_options.as_ref()),
        Some(&SessionStateOptions {
            model: None,
            allowed_tools: Some(vec!["apply_patch".to_string()]),
            max_turns: None,
        })
    );
}

/// advertised model state 应同步当前模型和可选模型列表。
#[test]
fn current_model_and_advertised_model_state_are_synced() {
    let mut record = sample_record();

    set_current_model_id(&mut record, Some("  model-a  "));
    sync_advertised_model_state(
        &mut record,
        Some(&SessionModelState::new(
            "model-b",
            vec![ModelInfo::new("model-b", "Model B"), ModelInfo::new("model-c", "Model C")],
        )),
    );

    let vwacp = record.vwacp.as_ref().expect("vwacp state");
    assert_eq!(vwacp.current_model_id.as_deref(), Some("model-b"));
    assert_eq!(
        vwacp.available_models.as_ref(),
        Some(&vec!["model-b".to_string(), "model-c".to_string()])
    );
}

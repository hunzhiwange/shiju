use super::*;
use crate::{SESSION_RECORD_SCHEMA, SessionEventLog, SessionTokenUsage};
use agent_client_protocol::{
    ModelInfo, SessionConfigOption, SessionConfigOptionCategory, SessionConfigSelectGroup,
    SessionConfigSelectOption, SessionModelState,
};

fn test_record() -> SessionRecord {
    SessionRecord {
        schema: SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: "record-1".to_string(),
        acp_session_id: "acp-1".to_string(),
        agent_session_id: None,
        agent_command: "agent".to_string(),
        agent_config: None,
        cwd: "/tmp/project".to_string(),
        name: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_used_at: "2026-01-01T00:00:00Z".to_string(),
        last_seq: 0,
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
        request_token_usage: std::collections::HashMap::new(),
        vwacp: None,
    }
}

#[test]
fn normalize_mode_id_trims_and_rejects_empty_values() {
    assert_eq!(normalize_mode_id(Some("  default ")).as_deref(), Some("default"));
    assert_eq!(normalize_mode_id(Some("  ")), None);
    assert_eq!(normalize_mode_id(None), None);
}

#[test]
fn set_desired_mode_id_creates_state_and_clears_blank_values() {
    let mut record = test_record();

    set_desired_mode_id(&mut record, Some("  plan "));
    assert_eq!(get_desired_mode_id(record.vwacp.as_ref()).as_deref(), Some("plan"));

    set_desired_mode_id(&mut record, Some(" "));
    assert_eq!(get_desired_mode_id(record.vwacp.as_ref()), None);
    assert!(record.vwacp.is_some());
}

#[test]
fn set_desired_model_id_preserves_other_session_options_and_drops_empty_options() {
    let mut record = test_record();

    set_desired_model_id(&mut record, Some(" gpt "));
    assert_eq!(get_desired_model_id(record.vwacp.as_ref()).as_deref(), Some("gpt"));

    let session_options = record.vwacp.as_mut().unwrap().session_options.as_mut().unwrap();
    session_options.allowed_tools = Some(vec!["shell".to_string()]);
    set_desired_model_id(&mut record, None);

    let session_options = record.vwacp.as_ref().unwrap().session_options.as_ref().unwrap();
    assert_eq!(session_options.model, None);
    assert_eq!(session_options.allowed_tools.as_deref(), Some(&["shell".to_string()][..]));

    record.vwacp.as_mut().unwrap().session_options.as_mut().unwrap().allowed_tools = None;
    set_desired_model_id(&mut record, None);
    assert_eq!(record.vwacp.as_ref().unwrap().session_options, None);
}

#[test]
fn set_current_model_id_normalizes_current_model() {
    let mut record = test_record();

    set_current_model_id(&mut record, Some(" model-a "));
    assert_eq!(record.vwacp.as_ref().unwrap().current_model_id.as_deref(), Some("model-a"));

    set_current_model_id(&mut record, Some(""));
    assert_eq!(record.vwacp.as_ref().unwrap().current_model_id, None);
}

#[test]
fn sync_advertised_model_state_ignores_missing_state() {
    let mut record = test_record();

    set_current_model_id(&mut record, Some("model-a"));
    record.vwacp.as_mut().unwrap().available_models =
        Some(vec!["model-a".to_string(), "model-b".to_string()]);

    sync_advertised_model_state(&mut record, None);

    let vwacp = record.vwacp.as_ref().unwrap();
    assert_eq!(vwacp.current_model_id.as_deref(), Some("model-a"));
    assert_eq!(
        vwacp.available_models.as_ref(),
        Some(&vec!["model-a".to_string(), "model-b".to_string()])
    );
}

#[test]
fn sync_advertised_model_state_updates_current_and_available_models() {
    let mut record = test_record();

    sync_advertised_model_state(
        &mut record,
        Some(&SessionModelState::new(
            "model-b",
            vec![ModelInfo::new("model-b", "Model B"), ModelInfo::new("model-c", "Model C")],
        )),
    );

    let vwacp = record.vwacp.as_ref().unwrap();
    assert_eq!(vwacp.current_model_id.as_deref(), Some("model-b"));
    assert_eq!(
        vwacp.available_models.as_ref(),
        Some(&vec!["model-b".to_string(), "model-c".to_string()])
    );
}

#[test]
fn config_resolution_prefers_categories_and_reports_available_values() {
    let options = vec![
        SessionConfigOption::select(
            "custom_model",
            "Model",
            "model-a",
            vec![
                SessionConfigSelectOption::new("model-a", "Model A"),
                SessionConfigSelectOption::new("model-b", "Model B"),
            ],
        )
        .category(SessionConfigOptionCategory::Model),
        SessionConfigOption::select(
            "effort",
            "Effort",
            "high",
            vec![SessionConfigSelectOption::new("high", "High")],
        )
        .category(SessionConfigOptionCategory::ThoughtLevel),
    ];

    assert_eq!(
        resolve_model_config_value(Some(&options), "model-b").expect("supported model"),
        Some("custom_model".to_string())
    );
    assert_eq!(
        resolve_config_value(Some(&options), "reasoning_effort", "high").expect("supported effort"),
        "effort"
    );
    assert_eq!(
        resolve_config_value(Some(&options), "thought_level", "max")
            .expect_err("unsupported effort"),
        "Unsupported value max for config option effort; available values: high"
    );
}

#[test]
fn desired_config_options_roundtrip_without_overwriting_current_capabilities() {
    let mut record = test_record();
    set_desired_config_option(&mut record, "reasoning_effort", "high");
    set_desired_config_option(&mut record, "temperature", "0.2");

    let state = record.vwacp.as_ref().expect("vwacp state");
    assert_eq!(
        get_desired_config_options(Some(state)),
        [
            ("reasoning_effort".to_string(), "high".to_string()),
            ("temperature".to_string(), "0.2".to_string()),
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(state.config_options, None);
}

#[test]
fn config_state_updates_ignore_blank_values_and_missing_capabilities() {
    let mut record = test_record();

    set_desired_config_option(&mut record, " ", "high");
    set_desired_config_option(&mut record, "effort", " ");
    sync_config_options(&mut record, None);

    assert!(record.vwacp.is_none());
    assert!(get_desired_config_options(None).is_empty());
    assert_eq!(resolve_model_config_value(None, "model-a"), Ok(None));
    assert_eq!(
        resolve_config_value(None, "effort", "high"),
        Err("Agent did not advertise session config options; cannot set effort".to_string())
    );
}

#[test]
fn resolve_config_value_accepts_values_from_grouped_options() {
    let grouped = SessionConfigOption::select(
        "effort",
        "Effort",
        "medium",
        vec![SessionConfigSelectGroup::new(
            "quality",
            "Quality",
            vec![
                SessionConfigSelectOption::new("medium", "Medium"),
                SessionConfigSelectOption::new("high", "High"),
            ],
        )],
    );

    assert_eq!(resolve_config_value(Some(&[grouped]), "effort", "high"), Ok("effort".to_string()));
}

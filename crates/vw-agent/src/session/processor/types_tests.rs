//! 验证会话处理请求类型的默认状态、引用保留与可靠性快照契约。

#[test]
fn types_tests_module_is_wired() {
    let marker = String::from("types_tests");
    assert_eq!(marker.as_str(), "types_tests");
}

#[test]
fn tool_session_state_defaults_without_background_children() {
    let state = super::ToolSessionState::default();

    assert!(state.agent_background_children.is_empty());
}

#[test]
fn resolved_request_model_keeps_full_reference_and_limit() {
    let resolved = super::ResolvedRequestModel::new(
        "provider/model".to_string(),
        crate::app::agent::provider::provider::ModelLimit {
            context: 16_384,
            input: None,
            output: 4_096,
        },
    );

    assert_eq!(resolved.model, "provider/model");
    assert_eq!(resolved.limit.context, 16_384);
}

#[test]
fn request_keeps_strongly_typed_reliability_snapshot() {
    let mut request = super::Request {
        stream: 1,
        session: "reliability-snapshot".to_string(),
        query: String::new(),
        root: None,
        model: None,
        reliability: crate::app::agent::config::ReliabilityConfig::default(),
        options: serde_json::json!({}),
        approval: None,
        channel_name: None,
        non_cli_approval_context: None,
        assistant_message_id: None,
        history: Vec::new(),
        persist_app_session_artifacts: false,
        app_session_scope: None,
        abort_token: None,
        session_tools: crate::session::runtime::SessionToolset::default(),
    };
    request.reliability.provider_retries = 9;
    request.reliability.provider_backoff_ms = 2_500;

    let cloned = request.clone();
    assert_eq!(cloned.reliability.provider_retries, 9);
    assert_eq!(cloned.reliability.provider_backoff_ms, 2_500);
}

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

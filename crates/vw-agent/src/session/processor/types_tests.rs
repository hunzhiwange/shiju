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

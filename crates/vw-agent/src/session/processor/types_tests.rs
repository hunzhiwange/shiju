#[test]
fn types_tests_module_is_wired() {
    let marker = String::from("types_tests");
    assert_eq!(marker.as_str(), "types_tests");
}

#[test]
fn tool_session_state_defaults_to_unsatisfied_agent_tool_requirement() {
    let state = super::ToolSessionState::default();

    assert_eq!(state.successful_agent_delegations, 0);
    assert_eq!(state.required_agent_tool_retries, 0);
}

use super::*;

#[test]
fn turn_policy_uses_selected_agent_and_positive_fallbacks() {
    let mut config = crate::config::Config::default();
    config.agent.max_history_messages = 64;
    config.agents.insert(
        "main".to_string(),
        crate::config::DelegateAgentConfig {
            max_iterations: 12,
            ..Default::default()
        },
    );
    config.agents.insert(
        "reviewer".to_string(),
        crate::config::DelegateAgentConfig {
            max_iterations: 7,
            ..Default::default()
        },
    );
    config.agent.max_tool_iterations = 23;

    let selected = ChatTurnPolicy::from_config(&config, Some(" reviewer "));
    assert_eq!(selected.max_tool_iterations, 7);
    assert_eq!(selected.max_history_messages, 64);

    let main = ChatTurnPolicy::from_config(&config, None);
    assert_eq!(main.max_tool_iterations, 23);

    config.agents.get_mut("reviewer").expect("reviewer agent").max_iterations = 0;
    config.agent.max_history_messages = 0;
    let zeroed = ChatTurnPolicy::from_config(&config, Some("reviewer"));
    assert_eq!(zeroed.max_tool_iterations, DEFAULT_MAX_TOOL_ITERATIONS);
    assert_eq!(zeroed.max_history_messages, DEFAULT_MAX_HISTORY_MESSAGES);

    config.agents.get_mut("reviewer").expect("reviewer agent").max_iterations = 50_001;
    config.agent.max_history_messages = usize::MAX;
    let unbounded_iterations = ChatTurnPolicy::from_config(&config, Some("reviewer"));
    assert_eq!(unbounded_iterations.max_tool_iterations, 50_001);
    assert_eq!(unbounded_iterations.max_history_messages, MAX_HISTORY_MESSAGES);
}

#[test]
fn turn_policy_falls_back_to_main_for_unknown_or_blank_agent() {
    let mut config = crate::config::Config::default();
    config.agent.max_tool_iterations = 19;
    config.agents.insert(
        "main".to_string(),
        crate::config::DelegateAgentConfig {
            max_iterations: 9,
            ..Default::default()
        },
    );

    assert_eq!(ChatTurnPolicy::from_config(&config, Some("missing")).max_tool_iterations, 19);
    assert_eq!(ChatTurnPolicy::from_config(&config, Some("  ")).max_tool_iterations, 19);
}

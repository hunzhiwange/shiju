//! 验证轮次策略选择代理及无效配置的主代理回退。

use super::*;

#[test]
fn turn_policy_uses_selected_agent_and_positive_fallbacks() {
    let mut config = crate::config::Config::default();
    config.agents.insert(
        "main".to_string(),
        crate::config::DelegateAgentConfig {
            max_iterations: 23,
            max_history_messages: 64,
            parallel_tools: true,
            ..Default::default()
        },
    );
    config.agents.insert(
        "reviewer".to_string(),
        crate::config::DelegateAgentConfig {
            max_iterations: 7,
            max_history_messages: 31,
            ..Default::default()
        },
    );
    let selected = ChatTurnPolicy::from_config(&config, Some(" reviewer "));
    assert_eq!(selected.max_iterations, 7);
    assert_eq!(selected.max_history_messages, 31);
    assert!(!selected.parallel_tools);

    let main = ChatTurnPolicy::from_config(&config, None);
    assert_eq!(main.max_iterations, 23);
    assert!(main.parallel_tools);

    let reviewer = config.agents.get_mut("reviewer").expect("reviewer agent");
    reviewer.max_iterations = 0;
    reviewer.max_history_messages = 0;
    let zeroed = ChatTurnPolicy::from_config(&config, Some("reviewer"));
    assert_eq!(zeroed.max_iterations, DEFAULT_MAX_ITERATIONS);
    assert_eq!(zeroed.max_history_messages, DEFAULT_MAX_HISTORY_MESSAGES);

    let reviewer = config.agents.get_mut("reviewer").expect("reviewer agent");
    reviewer.max_iterations = 50_001;
    reviewer.max_history_messages = usize::MAX;
    let unbounded_iterations = ChatTurnPolicy::from_config(&config, Some("reviewer"));
    assert_eq!(unbounded_iterations.max_iterations, 50_001);
    assert_eq!(unbounded_iterations.max_history_messages, MAX_HISTORY_MESSAGES);
}

#[test]
fn turn_policy_falls_back_to_main_for_unknown_or_blank_agent() {
    let mut config = crate::config::Config::default();
    config.agents.insert(
        "main".to_string(),
        crate::config::DelegateAgentConfig { max_iterations: 19, ..Default::default() },
    );

    assert_eq!(ChatTurnPolicy::from_config(&config, Some("missing")).max_iterations, 19);
    assert_eq!(ChatTurnPolicy::from_config(&config, Some("  ")).max_iterations, 19);
}

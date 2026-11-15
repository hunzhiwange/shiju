use super::sort_acp_agents;
use std::collections::HashMap;
use config_aggregate::AcpAgentConfig;
#[test]
fn sort_acp_agents_puts_codex_first_then_sorts_remaining_names() {
    let acp_cfg = HashMap::from([
        ("zeta".to_string(), AcpAgentConfig::default()),
        ("codex".to_string(), AcpAgentConfig::default()),
        ("alpha".to_string(), AcpAgentConfig::default()),
    ]);

    assert_eq!(sort_acp_agents(&acp_cfg), ["codex", "alpha", "zeta"]);
}

#[test]
fn sort_acp_agents_sorts_without_codex() {
    let acp_cfg = HashMap::from([
        ("shell".to_string(), AcpAgentConfig::default()),
        ("browser".to_string(), AcpAgentConfig::default()),
    ]);

    assert_eq!(sort_acp_agents(&acp_cfg), ["browser", "shell"]);
}

#[test]
fn sort_acp_agents_returns_empty_list_for_empty_config() {
    let acp_cfg = HashMap::new();

    assert!(sort_acp_agents(&acp_cfg).is_empty());
}

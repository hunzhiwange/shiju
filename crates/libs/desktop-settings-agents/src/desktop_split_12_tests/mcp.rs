use crate::defaults;
use config_agent_profile::AgentDefault;
#[test]
fn agent_config_cleanup_mcp_inherit_and_fixed_empty_roundtrip() {
    let mut defaults = config_agent_profile::AgentDefaults::default();
    for selection in [AgentDefault::Inherit, AgentDefault::Fixed(vec![])] {
        defaults.mcp = selection.clone();
        let restored: config_agent_profile::AgentDefaults =
            serde_json::from_value(defaults::patch(&defaults)).unwrap();
        assert_eq!(restored.mcp, selection);
    }
    assert!(
        defaults::patch(&config_agent_profile::AgentDefaults::default())["mcp"]["value"].is_null()
    );
}

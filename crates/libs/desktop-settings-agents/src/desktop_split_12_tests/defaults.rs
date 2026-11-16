use crate::state::DelegateAgentSettingsEntry;
use config_agent::AgentDefinitionConfig;
use config_agent_profile::{AgentDefault, AgentDefaults};
#[test]
fn agent_defaults_form_roundtrips_explicit_empty_lists() {
    let config = AgentDefinitionConfig {
        defaults: AgentDefaults {
            skills: AgentDefault::Fixed(vec![]),
            mcp: AgentDefault::Fixed(vec![]),
            ..Default::default()
        },
        ..Default::default()
    };
    let entry = DelegateAgentSettingsEntry::from_config("custom", Some(config));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("agent.json");
    std::fs::write(&path, serde_json::to_vec(&entry.defaults).unwrap()).unwrap();
    let wire = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let restored: AgentDefaults = serde_json::from_value(wire).unwrap();
    assert_eq!(restored.skills, AgentDefault::Fixed(vec![]));
    assert_eq!(restored.mcp, AgentDefault::Fixed(vec![]));
}

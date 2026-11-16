use crate::agent_binding::AgentBinding;
use config_agent::AgentDefinitionConfig;
use config_agent_profile::AgentRuntime;
#[test]
fn agent_session_identity_rejects_engine_and_agent_changes() {
    let binding = AgentBinding::from_definition("main".into(), &AgentDefinitionConfig::default());
    assert!(binding.validate_selection(Some("main"), None).is_ok());
    assert!(binding.validate_selection(None, None).is_ok());
    assert!(binding.validate_selection(Some("other"), None).is_err());
    assert!(binding.validate_selection(Some("main"), Some("codex")).is_err());
    let acp = AgentBinding::from_definition(
        "acp-codex".into(),
        &AgentDefinitionConfig {
            runtime: AgentRuntime::Acp { backend: "codex".into() },
            ..Default::default()
        },
    );
    assert!(acp.validate_selection(None, Some("codex")).is_ok());
    assert!(acp.validate_selection(None, Some("claude")).is_err());
}

#[test]
fn agent_session_identity_keeps_live_credentials_and_security_rules() {
    let original =
        AgentDefinitionConfig { system_prompt: Some("old rule".into()), ..Default::default() };
    let binding = AgentBinding::from_definition("main".into(), &original);
    let mut current = AgentDefinitionConfig {
        enabled: false,
        permission: serde_json::json!({"*":"deny"}),
        api_key: Some("rotated-credential".into()),
        system_prompt: Some("edited rule".into()),
        ..Default::default()
    };
    binding.apply(&mut current);
    assert!(current.enabled);
    assert_eq!(current.system_prompt.as_deref(), Some("old rule"));
    assert_eq!(current.permission, serde_json::json!({"*":"deny"}));
    assert_eq!(current.api_key.as_deref(), Some("rotated-credential"));
}

#[test]
fn agent_config_cleanup_full_access_retains_permission_denials() {
    let mut definition = AgentDefinitionConfig::default();
    definition.defaults.full_access = true;
    let binding = AgentBinding::from_definition("main".into(), &definition);
    let mut current =
        AgentDefinitionConfig { permission: serde_json::json!({"*":"deny"}), ..Default::default() };
    binding.apply(&mut current);
    assert!(current.defaults.full_access);
    assert_eq!(current.permission, serde_json::json!({"*":"deny"}));
}

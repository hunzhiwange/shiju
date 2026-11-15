use config_agent_profile::{AgentDefault, AgentDefaults, AgentRuntime};
#[test]
fn agent_defaults_form_rejects_incompatible_engine_values() {
    let mut defaults = AgentDefaults::default();
    defaults.permission = AgentDefault::Fixed(serde_json::json!({"*":"allow"}));
    assert!(
        super::validate(
            &AgentRuntime::Acp { backend: "codex".into() },
            &defaults,
            &[],
            &[],
            &[]
        )
        .is_err()
    );
    defaults.permission = AgentDefault::Fixed(serde_json::json!({"*":"deny"}));
    assert!(
        super::validate(
            &AgentRuntime::Acp { backend: "codex".into() },
            &defaults,
            &[],
            &[],
            &[]
        )
        .is_ok()
    );
    defaults.mcp = AgentDefault::Fixed(vec!["disabled".into()]);
    assert!(super::validate(&AgentRuntime::Native, &defaults, &[], &[], &[]).is_err());
}

#[test]
fn agent_defaults_form_inherit_patch_removes_fixed_payloads() {
    let patch = super::patch(&AgentDefaults::default());
    for field in ["model", "permission", "skills", "mcp"] {
        assert_eq!(patch[field]["mode"], "inherit");
        assert!(patch[field].as_object().unwrap().contains_key("value"));
        assert!(patch[field]["value"].is_null());
    }
}

#[test]
fn agent_config_cleanup_full_access_patch_preserves_permission() {
    let defaults = AgentDefaults {
        full_access: true,
        permission: AgentDefault::Fixed(serde_json::json!({"*":"deny"})),
        ..Default::default()
    };
    let patch = super::patch(&defaults);
    assert_eq!(patch["full_access"], true);
    assert_eq!(patch["permission"]["value"], serde_json::json!({"*":"deny"}));
}

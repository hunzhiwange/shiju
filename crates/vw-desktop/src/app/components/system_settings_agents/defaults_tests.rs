use crate::app::state::DelegateAgentSettingsEntry;
use vw_config_types::agent::{AgentDefault, AgentDefaults, AgentDefinitionConfig, AgentRuntime};

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

#[test]
fn agent_defaults_form_rejects_incompatible_engine_values() {
    let mut defaults = AgentDefaults::default();
    defaults.permission = AgentDefault::Fixed(serde_json::json!({"*":"allow"}));
    assert!(
        super::defaults::validate(
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
        super::defaults::validate(
            &AgentRuntime::Acp { backend: "codex".into() },
            &defaults,
            &[],
            &[],
            &[]
        )
        .is_ok()
    );
    defaults.mcp = AgentDefault::Fixed(vec!["disabled".into()]);
    assert!(super::defaults::validate(&AgentRuntime::Native, &defaults, &[], &[], &[]).is_err());
}

#[test]
fn agent_defaults_form_inherit_patch_removes_fixed_payloads() {
    let patch = super::defaults::patch(&AgentDefaults::default());
    for field in ["model", "permission", "skills", "mcp"] {
        assert_eq!(patch[field]["mode"], "inherit");
        assert!(patch[field].as_object().unwrap().contains_key("value"));
        assert!(patch[field]["value"].is_null());
    }
}

#[test]
fn agent_defaults_form_switch_preserves_invalid_draft_and_prevents_save() {
    use crate::app::{
        App,
        message::settings::{AgentsMessage, SettingsMessage},
    };
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    app.acp_agents = vec!["codex".into()];
    app.agents_settings.entries.push(DelegateAgentSettingsEntry::from_config("custom", None));
    let dispatch = |app: &mut App, message| {
        let _ = crate::app::message::settings::update(app, SettingsMessage::Agents(message));
    };
    dispatch(&mut app, AgentsMessage::ProfileOpen("custom".into()));
    let defaults = AgentDefaults {
        permission: AgentDefault::Fixed(serde_json::json!({"*":"ask"})),
        ..Default::default()
    };
    dispatch(&mut app, AgentsMessage::ProfileDefaultsChanged(defaults.clone()));
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".into()));
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(!app.agents_settings.profile_saving);
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().defaults, defaults);
    assert_eq!(app.agents_settings.entries.last().unwrap().defaults, AgentDefaults::default());
}

#[test]
fn agent_defaults_form_forced_binding_rejects_conflicting_default() {
    use vw_config_types::agent::AgentModelReference;
    let app = crate::app::App::new().0;
    let mut draft = DelegateAgentSettingsEntry::from_config("custom", None);
    draft.force_model_selection = true;
    draft.provider = "provider".into();
    draft.model = "forced".into();
    draft.defaults.model = AgentDefault::Fixed(AgentModelReference {
        provider: "provider".into(),
        model: "other".into(),
    });
    assert!(super::defaults::validation_error(&app, &draft).unwrap().contains("强制模型绑定冲突"));
}

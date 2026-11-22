//! Host contracts for profile capability and provider catalog validation.
use crate::app::state::DelegateAgentSettingsEntry;
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use config_agent::AgentDefinitionConfig;
use config_agent_profile::{AgentDefault, AgentDefaults, AgentRuntime};

#[test]
fn agent_model_inheritance_reads_acp_capabilities_without_chat() {
    use crate::app::state::{ModelSummary, ProviderModelsSummary, ProviderSummary};
    use config_agent_profile::AgentModelReference;
    let mut app = crate::app::App::new().0;
    let model = AgentModelReference { provider: "provider".into(), model: "selected".into() };
    app.agents_settings.providers = vec![ProviderSummary {
        id: "provider".into(),
        name: "Provider".into(),
        source_label: crate::app::i18n::settings_models::messages::Feedback::owned("配置", vec![]),
        connected: true,
    }];
    app.agents_settings.provider_models = vec![ProviderModelsSummary {
        id: "provider".into(),
        name: "Provider".into(),
        models: vec![ModelSummary {
            id: "selected".into(),
            name: "Selected".into(),
            enabled: true,
            toolcall: true,
            attachment: false,
            context_limit: 8192,
            detail: serde_json::Value::Null,
        }],
    }];
    let config = AgentDefinitionConfig {
        runtime: AgentRuntime::Acp { backend: "codex".into() },
        defaults: AgentDefaults { model: AgentDefault::Fixed(model), ..Default::default() },
        ..Default::default()
    };
    let draft = DelegateAgentSettingsEntry::from_config("custom", Some(config));
    app.agents_settings.entries.push(draft.clone());
    app.agents_settings.config_ready = true;
    let dispatch = |app: &mut crate::app::App, message| {
        let _ = crate::app::message::settings::agents::update(app, SettingsMessage::Agents(message));
    };
    let valid = |app: &crate::app::App| super::defaults::validation_error(app, &draft).is_none();
    dispatch(&mut app, AgentsMessage::ProfileOpen("custom".into()));
    dispatch(&mut app, AgentsMessage::ProfileModelSourceRefresh);
    dispatch(&mut app, AgentsMessage::ProfileModelSourceLoaded { generation: 1, result: Ok(true) });
    assert!(!valid(&app));
    dispatch(&mut app, AgentsMessage::ProfileModelSourceLoaded { generation: 2, result: Ok(true) });
    assert!(valid(&app));
    app.agents_settings.provider_models[0].models[0].enabled = false;
    assert!(!valid(&app));
    app.agents_settings.provider_models[0].models[0].enabled = true;
    app.agents_settings.providers[0].connected = false;
    assert!(!valid(&app));
}

//! Host contracts: profile update dispatch, session ACP capability projection and provider/MCP catalog validation.
use crate::app::state::DelegateAgentSettingsEntry;
use config_agent_profile::{AgentDefault, AgentDefaults, AgentRuntime};
use config_agent::AgentDefinitionConfig;

#[test]
fn agent_model_inheritance_verified_acp_models_survive_disconnect_and_reopen() {
    use crate::app::state::{ModelSummary, ProviderModelsSummary, ProviderSummary};
    use config_agent_profile::AgentModelReference;
    use vw_api_acp_model::acp_model_source::{
        AcpModelSource, AcpModelSourceCapability,
    };
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
    let runtime = app.get_session_runtime_mut("verified");
    runtime.acp_agent = Some("codex".into());
    runtime.acp_options_state.source_request_id = Some(0);
    let _ = crate::app::message::chat::update(
        &mut app,
        crate::app::message::ChatMessage::AcpSourceLoaded {
            session_id: "verified".into(),
            request_id: 0,
            agent: "codex".into(),
            result: Ok(vw_api_acp_model::acp_session::AcpSessionOptions {
                connected: false,
                model_source_capabilities: vec![AcpModelSourceCapability::Supported {
                    source: AcpModelSource::SessionModel,
                }],
                ..Default::default()
            }),
        },
    );
    let config = AgentDefinitionConfig {
        runtime: AgentRuntime::Acp { backend: "codex".into() },
        defaults: AgentDefaults { model: AgentDefault::Fixed(model), ..Default::default() },
        ..Default::default()
    };
    for draft in [
        {
            let mut draft = DelegateAgentSettingsEntry::from_config("custom", None);
            draft.runtime = config.runtime.clone();
            draft.defaults = config.defaults.clone();
            draft
        },
        DelegateAgentSettingsEntry::from_config("custom", Some(config)),
    ] {
        assert_eq!(super::defaults::validation_error(&app, &draft), None);
        app.session_runtime_states.get_mut("verified").unwrap().acp_agent = Some("other".into());
        assert!(super::defaults::validation_error(&app, &draft).is_some());
        app.session_runtime_states.get_mut("verified").unwrap().acp_agent = Some("codex".into());
    }
    app.agents_settings.provider_models[0].models[0].enabled = false;
    let draft = DelegateAgentSettingsEntry::from_config(
        "custom",
        Some(AgentDefinitionConfig {
            runtime: AgentRuntime::Acp { backend: "codex".into() },
            defaults: AgentDefaults {
                model: AgentDefault::Fixed(AgentModelReference {
                    provider: "provider".into(),
                    model: "selected".into(),
                }),
                ..Default::default()
            },
            ..Default::default()
        }),
    );
    assert!(super::defaults::validation_error(&app, &draft).is_some());
    app.agents_settings.provider_models[0].models[0].enabled = true;
    app.agents_settings.providers[0].connected = false;
    assert!(super::defaults::validation_error(&app, &draft).is_some());
}

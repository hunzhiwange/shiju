//! Host contracts: settings update dispatch, session draft launch, engine icon assets and root page projection.

fn catalog_config() -> config_aggregate::Config {
    // Wire-shaped catalog fixture: official identities come from the gateway, not CLI detection.
    let mut config: config_aggregate::Config =
        serde_json::from_value(serde_json::json!({})).unwrap();
    for backend in [
        "auggie",
        "claude",
        "codex",
        "codebuddy",
        "goose",
        "vibe",
        "hermes",
        "snow",
        "copilot",
        "cursor",
        "droid",
        "gemini",
        "iflow",
        "kilocode",
        "kimi",
        "kiro",
        "opencode",
        "openclaw",
        "pi",
        "qoder",
        "qwen",
        "trae",
    ] {
        let mut value = serde_json::to_value(config_agent::DelegateAgentConfig::default()).unwrap();
        let fields = serde_json::json!({"builtin":true, "enabled": backend == "codex", "label":backend,
            "runtime":{"kind":"acp","backend":backend}});
        value.as_object_mut().unwrap().extend(fields.as_object().unwrap().clone());
        config.agents.insert(format!("acp-{backend}"), serde_json::from_value(value).unwrap());
    }
    config
}

fn dispatch(app: &mut crate::app::App, message: crate::app::message::settings::AgentsMessage) {
    let _ = crate::app::message::settings::update(
        app,
        crate::app::message::settings::SettingsMessage::Agents(message),
    );
}

#[test]
fn desktop_first_response_catalog_launch_preserves_profile_in_a_draft() {
    use crate::app::message::settings::AgentsMessage;
    let mut app = crate::app::App::new().0;
    app.agents_settings.config_ready = true;
    app.agents_settings.entries =
        crate::app::state::agent_settings_entries_from_config(&catalog_config());
    app.navigation_tabs.project_path = Some("/tmp/catalog-project".into());
    app.agents_settings.project_path = app.navigation_tabs.project_path.clone();
    app.session_state.acp_agent = None;
    dispatch(&mut app, AgentsMessage::CatalogLaunch("acp-codex".into()));
    assert!(app.session_state.active_session_id.is_none());
    assert!(app.session_state.sessions.is_empty());
    assert_eq!(app.current_session_runtime().agent.as_deref(), Some("acp-codex"));
    assert!(!app.navigation_panels.show_settings);
    assert!(!app.agents_settings.catalog_busy);
}

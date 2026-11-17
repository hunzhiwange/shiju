use super::*;

#[test]
fn grok_generated_config_has_native_retry_integer_and_bound_models() {
    let mut profile = AcpLaunchProfile::new(
        &agent_config::schema::AcpAgentConfig {
            command: "grok".into(),
            args: vec!["agent".into(), "stdio".into()],
            ..Default::default()
        },
        model_transport::RelayIdentity {
            tenant: Default::default(),
            project: "project".into(),
            session: "grok-config".into(),
            provider: "selected-provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    configure(&mut profile, "http://127.0.0.1:12345").unwrap();
    let home = std::path::Path::new(&profile.config().env["GROK_HOME"]);
    let config: toml::Value =
        toml::from_str(&std::fs::read_to_string(home.join("config.toml")).unwrap()).unwrap();
    assert_eq!(config["models"]["max_retries"].as_integer(), Some(0));
    for name in ["shiju-session", "grok-4.6", "grok-4.5"] {
        assert_eq!(config["model"][name]["model"].as_str(), Some("selected-model"));
        assert_eq!(config["model"][name]["api_backend"].as_str(), Some("chat_completions"));
        assert_eq!(config["model"][name]["supports_reasoning_effort"].as_bool(), Some(false));
        assert_eq!(config["model"][name]["base_url"].as_str(), Some("http://127.0.0.1:12345/v1"));
    }
    assert_eq!(config["subagents"]["enabled"].as_bool(), Some(false));
    assert_eq!(config["cli"]["auto_update"].as_bool(), Some(false));
}

#[test]
fn grok_session_model_uses_native_chat_transport() {
    let api = serde_json::from_value(serde_json::json!({
        "id": "selected", "adapter": "openai-compatible", "url": "https://example.invalid/v1"
    }))
    .unwrap();
    assert_eq!(
        super::super::source_protocol("grok", &api).unwrap(),
        model_protocols::ModelProtocol::Chat
    );
}

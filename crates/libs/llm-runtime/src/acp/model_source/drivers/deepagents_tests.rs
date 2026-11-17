use super::*;

#[test]
fn deepagents_selected_model_uses_chat_protocol() {
    let api = serde_json::from_value(serde_json::json!({
        "id": "fixture", "adapter": "openai", "url": "https://example.invalid/v1/chat/completions"
    }))
    .unwrap();
    assert_eq!(source_protocol("deepagents", &api).unwrap(), model_protocols::ModelProtocol::Chat);
}

fn profile() -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &agent_config::schema::AcpAgentConfig {
            command: "deepagents-acp".into(),
            ..Default::default()
        },
        model_transport::RelayIdentity {
            tenant: Default::default(),
            project: "project".into(),
            session: "session:deepagents".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    profile
}

#[test]
fn deepagents_private_model_configuration_is_revoked() {
    let mut p = profile();
    deepagents::configure(&mut p, "http://127.0.0.1:12345", "/workspace").unwrap();
    assert_eq!(p.config().args, ["--model", "openai:selected-model", "--workspace", "/workspace"]);
    assert_eq!(p.config().env["OPENAI_BASE_URL"], "http://127.0.0.1:12345/v1");
    assert_eq!(p.config().env["OPENAI_API_KEY"], p.binding.bearer_token());
    let binding = p.binding.clone();
    let token = binding.bearer_token().to_owned();
    let path = p.directory().to_owned();
    drop(p);
    assert!(!path.exists());
    assert!(!binding.accepts(&token, binding.identity()));
}

#[test]
fn deepagents_rejects_overrides_and_nonlocal_relays() {
    for endpoint in
        ["https://example.com", "http://127.0.0.1:12345/v1", "http://user@127.0.0.1:12345"]
    {
        let mut p = profile();
        assert!(deepagents::configure(&mut p, endpoint, "/workspace").is_err());
        assert!(!p.config().env.contains_key("OPENAI_API_KEY"));
    }
    for key in ["OPENAI_API_BASE", "NODE_OPTIONS", "LANGSMITH_TRACING"] {
        let p = profile();
        let mut config = p.config().clone();
        config.env.remove(super::super::profile::REFERENCE_ENV);
        config.env.insert(key.into(), "override".into());
        let mut p = AcpLaunchProfile::new(&config, p.binding.identity().clone()).unwrap();
        p.model_supports_tool_call = true;
        assert!(deepagents::configure(&mut p, "http://127.0.0.1:12345", "/workspace").is_err());
    }
    let p = profile();
    let mut config = p.config().clone();
    config.env.remove(super::super::profile::REFERENCE_ENV);
    config.args = vec!["--model".into(), "other".into()];
    let mut p = AcpLaunchProfile::new(&config, p.binding.identity().clone()).unwrap();
    assert!(deepagents::configure(&mut p, "http://127.0.0.1:12345", "/workspace").is_err());
}

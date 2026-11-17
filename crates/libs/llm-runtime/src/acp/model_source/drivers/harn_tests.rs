use super::*;

#[test]
fn harn_selected_model_uses_chat_protocol() {
    let api = serde_json::from_value(serde_json::json!({
        "id": "fixture", "adapter": "openai", "url": "https://example.invalid/v1/chat/completions"
    }))
    .unwrap();
    assert_eq!(source_protocol("harn", &api).unwrap(), model_protocols::ModelProtocol::Chat);
    assert_eq!(verified_release("harn"), Some("0.10.201"));
}

fn profile() -> AcpLaunchProfile {
    AcpLaunchProfile::new(
        &agent_config::schema::AcpAgentConfig { command: "harn".into(), ..Default::default() },
        model_transport::RelayIdentity {
            tenant: Default::default(),
            project: "project".into(),
            session: "session:harn".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap()
}

#[test]
fn harn_rejects_unreviewed_arguments_and_environment() {
    for config in [
        agent_config::schema::AcpAgentConfig {
            command: "harn".into(),
            args: vec!["other.harn".into()],
            ..Default::default()
        },
        agent_config::schema::AcpAgentConfig {
            command: "harn".into(),
            env: std::collections::HashMap::from([("HARN_LLM_PROVIDER".into(), "other".into())]),
            ..Default::default()
        },
    ] {
        let identity = profile().binding.identity().clone();
        let mut p = AcpLaunchProfile::new(&config, identity).unwrap();
        assert!(harn::configure(&mut p, "http://127.0.0.1:12345", "/workspace").is_err());
        assert!(!p.config().env.contains_key("SHIJU_HARN_RELAY_TOKEN"));
    }
}

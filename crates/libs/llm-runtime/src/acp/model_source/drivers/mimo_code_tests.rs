use super::*;

#[test]
fn mimo_code_routes_only_through_the_reviewed_chat_transport() {
    let api = model_types::ApiInfo {
        id: "selected-model".into(),
        adapter: "openai-compatible".into(),
        url: "https://example.invalid/v1/chat/completions".into(),
        protocols: None,
    };
    assert_eq!(source_protocol("mimo-code", &api).unwrap(), model_protocols::ModelProtocol::Chat,);
    assert_eq!(verified_release("mimo-code"), Some("0.1.17"));
}

#[test]
fn mimo_code_rejects_an_unreviewed_binary_before_exposing_credentials() {
    let config = agent_config::schema::AcpAgentConfig {
        command: "mimo".into(),
        args: vec!["acp".into()],
        ..Default::default()
    };
    let identity = model_transport::RelayIdentity {
        tenant: Default::default(),
        project: "project".into(),
        session: "session:mimo-code".into(),
        provider: "provider".into(),
        model: "provider/model".into(),
        generation: 0,
    };
    let mut profile = AcpLaunchProfile::new(&config, identity).unwrap();
    let error = mimo_code::configure(&mut profile, "http://127.0.0.1:12345/", "/workspace");
    assert!(error.is_err());
    assert!(!profile.config().env.contains_key("SHIJU_MIMO_RELAY_TOKEN"));
}

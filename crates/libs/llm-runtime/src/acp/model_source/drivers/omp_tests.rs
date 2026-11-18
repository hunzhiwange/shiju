use super::*;

#[test]
fn omp_routes_only_through_the_reviewed_chat_transport() {
    let api = model_types::ApiInfo {
        id: "selected-model".into(),
        adapter: "openai-compatible".into(),
        url: "https://example.invalid/v1/chat/completions".into(),
        protocols: None,
    };
    assert_eq!(source_protocol("omp", &api).unwrap(), model_protocols::ModelProtocol::Chat,);
    assert_eq!(verified_release("omp"), Some("18.22.0"));
}

#[test]
fn omp_rejects_an_unreviewed_binary_before_exposing_credentials() {
    let config = agent_config::schema::AcpAgentConfig {
        command: "omp".into(),
        args: vec!["acp".into()],
        ..Default::default()
    };
    let identity = model_transport::RelayIdentity {
        tenant: Default::default(),
        project: "project".into(),
        session: "session:omp".into(),
        provider: "provider".into(),
        model: "provider/model".into(),
        generation: 0,
    };
    let mut profile = AcpLaunchProfile::new(&config, identity).unwrap();
    let error = omp::configure(&mut profile, "http://127.0.0.1:12345/", "/workspace");
    assert!(error.is_err());
    assert!(!profile.config().env.contains_key("SHIJU_OMP_RELAY_TOKEN"));
}

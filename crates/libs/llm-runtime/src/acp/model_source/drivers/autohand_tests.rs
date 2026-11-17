use super::*;

fn profile() -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &agent_config::schema::AcpAgentConfig {
            command: "autohand".into(),
            args: vec!["--acp".into()],
            ..Default::default()
        },
        model_transport::RelayIdentity {
            tenant: Default::default(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    profile.model_supports_temperature = true;
    profile
}

#[test]
fn autohand_private_model_configuration_is_revoked() {
    let mut profile = profile();
    autohand::configure(&mut profile, "http://127.0.0.1:12345").unwrap();
    let path = profile.directory().to_owned();
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path.join("config.json")).unwrap()).unwrap();
    assert_eq!(value["openai"]["model"], "selected-model");
    assert_eq!(value["openai"]["baseUrl"], "http://127.0.0.1:12345/v1");
    assert_eq!(value["openai"]["apiKey"], profile.binding.bearer_token());
    assert_eq!(profile.config().env["AUTOHAND_HOME"], path.to_string_lossy());
    let binding = profile.binding.clone();
    let token = binding.bearer_token().to_owned();
    drop(profile);
    assert!(!path.exists());
    assert!(!binding.accepts(&token, binding.identity()));
}

#[test]
fn autohand_rejects_overrides_and_nonlocal_relays_before_publication() {
    for endpoint in
        ["https://example.com", "http://127.0.0.1:12345/v1", "http://user@127.0.0.1:12345"]
    {
        let mut profile = profile();
        assert!(autohand::configure(&mut profile, endpoint).is_err());
        assert!(!profile.directory().join("config.json").exists());
    }
    let original = profile();
    let mut config = original.config().clone();
    config.env.remove(super::super::profile::REFERENCE_ENV);
    config.env.insert("autohand_config".into(), "/untrusted".into());
    let mut conflicting =
        AcpLaunchProfile::new(&config, original.binding.identity().clone()).unwrap();
    conflicting.model_supports_tool_call = true;
    conflicting.model_supports_temperature = true;
    assert!(autohand::configure(&mut conflicting, "http://127.0.0.1:12345").is_err());
    assert!(!conflicting.directory().join("config.json").exists());
}

#[test]
fn autohand_selected_model_uses_chat_protocol() {
    let api = serde_json::from_value(serde_json::json!({
        "id": "fixture", "adapter": "openai", "url": "https://example.invalid/v1/chat/completions"
    }))
    .unwrap();
    assert_eq!(source_protocol("autohand", &api).unwrap(), model_protocols::ModelProtocol::Chat);
}

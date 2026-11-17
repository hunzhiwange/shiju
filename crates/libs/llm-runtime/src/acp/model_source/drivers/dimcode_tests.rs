use super::*;

#[test]
fn dimcode_selected_model_uses_chat_protocol() {
    let api = serde_json::from_value(serde_json::json!({
        "id": "fixture", "adapter": "openai", "url": "https://example.invalid/v1/chat/completions"
    }))
    .unwrap();
    assert_eq!(source_protocol("dimcode", &api).unwrap(), model_protocols::ModelProtocol::Chat);
    assert_eq!(verified_release("dimcode"), Some("0.5.28"));
}

fn profile() -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &agent_config::schema::AcpAgentConfig {
            command: "dim".into(),
            args: vec!["acp".into()],
            ..Default::default()
        },
        model_transport::RelayIdentity {
            tenant: Default::default(),
            project: "project".into(),
            session: "session:dimcode".into(),
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
fn dimcode_private_model_configuration_is_revoked() {
    let mut p = profile();
    dimcode::configure(&mut p, "http://127.0.0.1:12345", "/workspace").unwrap();
    assert_eq!(p.config().args, ["acp"]);
    assert_eq!(p.session_model_id.as_deref(), Some("session-selected/selected-model"));
    let home = std::path::Path::new(&p.config().env["DIMCODE_HOME"]);
    let config: serde_json::Value =
        serde_json::from_slice(&std::fs::read(home.join("providers.json")).unwrap()).unwrap();
    assert_eq!(config["providers"][0]["apiKey"], p.binding.bearer_token());
    assert_eq!(config["providers"][0]["baseUrl"], "http://127.0.0.1:12345/v1");
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(std::fs::metadata(home).unwrap().permissions().mode() & 0o777, 0o700);
    assert_eq!(
        std::fs::metadata(home.join("providers.json")).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let binding = p.binding.clone();
    let token = binding.bearer_token().to_owned();
    let path = p.directory().to_owned();
    drop(p);
    assert!(!path.exists());
    assert!(!binding.accepts(&token, binding.identity()));
}

#[test]
fn dimcode_rejects_overrides_and_nonlocal_relays() {
    for endpoint in
        ["https://example.com", "http://127.0.0.1:12345/v1", "http://user@127.0.0.1:12345"]
    {
        let mut p = profile();
        assert!(dimcode::configure(&mut p, endpoint, "/workspace").is_err());
        assert!(!p.config().env.contains_key("OPENAI_API_KEY"));
    }
    for key in ["OPENAI_API_BASE", "NODE_OPTIONS", "LANGSMITH_TRACING"] {
        let p = profile();
        let mut config = p.config().clone();
        config.env.remove(super::super::profile::REFERENCE_ENV);
        config.env.insert(key.into(), "override".into());
        let mut p = AcpLaunchProfile::new(&config, p.binding.identity().clone()).unwrap();
        p.model_supports_tool_call = true;
        assert!(dimcode::configure(&mut p, "http://127.0.0.1:12345", "/workspace").is_err());
    }
    let p = profile();
    let mut config = p.config().clone();
    config.env.remove(super::super::profile::REFERENCE_ENV);
    config.args = vec!["--model".into(), "other".into()];
    let mut p = AcpLaunchProfile::new(&config, p.binding.identity().clone()).unwrap();
    assert!(dimcode::configure(&mut p, "http://127.0.0.1:12345", "/workspace").is_err());
}

#[test]
fn dimcode_rejects_project_provider_override() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir(workspace.path().join(".dimcode")).unwrap();
    std::fs::write(workspace.path().join(".dimcode/providers.json"), "{}").unwrap();
    assert!(
        dimcode::configure(
            &mut profile(),
            "http://127.0.0.1:12345",
            workspace.path().to_str().unwrap()
        )
        .is_err()
    );
}

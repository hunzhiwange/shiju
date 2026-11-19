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
    let config_path = if profile.config().command == "/usr/bin/python3" {
        let args = &profile.config().args;
        assert_eq!(args[0], "-I");
        assert_eq!(args[3..], ["12345", "grok", "agent", "stdio"]);
        assert!(std::path::Path::new(&args[1]).is_file());
        args[2].clone().into()
    } else {
        home.join("config.toml")
    };
    let config: toml::Value =
        toml::from_str(&std::fs::read_to_string(config_path).unwrap()).unwrap();
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

#[test]
fn grok_reuses_verified_state_with_a_new_private_relay() {
    let root = tempfile::tempdir().unwrap();
    let identity = model_transport::RelayIdentity {
        tenant: Default::default(),
        project: "project".into(),
        session: "grok-restored".into(),
        provider: "selected-provider".into(),
        model: "selected-model".into(),
        generation: 0,
    };
    let spawn = |endpoint| {
        let mut profile = AcpLaunchProfile::new(
            &agent_config::schema::AcpAgentConfig {
                command: "grok".into(),
                args: vec!["agent".into(), "stdio".into()],
                ..Default::default()
            },
            identity.clone(),
        )
        .unwrap();
        profile.state_root = Some(root.path().to_owned());
        profile.model_supports_tool_call = true;
        configure(&mut profile, endpoint).map(|_| profile)
    };

    let first = spawn("http://127.0.0.1:12001").unwrap();
    let state = std::path::PathBuf::from(&first.config().env["GROK_HOME"]).join("config.toml");
    if first.config().command == "/usr/bin/python3" {
        // Simulate the launcher publishing its private proxy port.
        let content = std::fs::read_to_string(&first.config().args[2]).unwrap();
        let mut config = tempfile::NamedTempFile::new_in(state.parent().unwrap()).unwrap();
        use std::io::Write;
        config.write_all(content.replace(":12001/", ":12101/").as_bytes()).unwrap();
        config.persist(&state).unwrap();
    }
    drop(first);
    let next = spawn("http://127.0.0.1:12002").unwrap();
    drop(next);
    let content = std::fs::read_to_string(&state).unwrap();
    std::fs::write(&state, content.replace("127.0.0.1", "example.invalid")).unwrap();
    assert!(spawn("http://127.0.0.1:12003").is_err());
}

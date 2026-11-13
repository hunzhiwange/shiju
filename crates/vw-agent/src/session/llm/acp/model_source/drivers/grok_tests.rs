use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

fn profile(args: &[&str]) -> AcpLaunchProfile {
    let mut launch = AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: "grok".into(),
            args: args.iter().map(|s| (*s).into()).collect(),
            ..Default::default()
        },
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "grok-fixture".into(),
            session: "isolated".into(),
            provider: "fixture".into(),
            model: "fixture-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    launch.model_supports_tool_call = true;
    launch
}

#[test]
fn acp_onboarding_grok_model_binding_is_private_and_conflicts_fail_closed() {
    let mut launch = profile(&["agent", "stdio"]);
    configure(&mut launch, "http://127.0.0.1:12345").unwrap();
    let env = &launch.config().env;
    assert_eq!(env["GROK_DEFAULT_MODEL"], "shiju-session");
    assert_eq!(env["SHIJU_GROK_RELAY_KEY"], launch.binding.bearer_token());
    let home = std::path::PathBuf::from(&env["GROK_HOME"]);
    let content = std::fs::read_to_string(home.join("config.toml")).unwrap();
    assert!(!content.contains(launch.binding.bearer_token()));
    let document: toml::Value = toml::from_str(&content).unwrap();
    assert_eq!(document["model"]["shiju-session"]["model"].as_str(), Some("fixture-model"));
    assert_eq!(document["model"]["grok-4.6"]["model"].as_str(), Some("fixture-model"));
    drop(launch);
    assert!(!home.exists());
    for args in [&["--acp", "--yolo"][..], &["--acp", "--config", "/tmp/other"]] {
        assert!(configure(&mut profile(args), "http://127.0.0.1:12345").is_err());
    }
    let mut launch = profile(&["agent", "stdio"]);
    assert!(configure(&mut launch, "https://example.test").is_err());
    let mut config = profile(&["agent", "stdio"]).config().clone();
    config.env.remove(crate::session::llm::acp::model_source::profile::REFERENCE_ENV);
    config.env.insert("GROK_HOME".into(), "other".into());
    let identity = profile(&["agent", "stdio"]).binding.identity.clone();
    let mut launch = AcpLaunchProfile::new(&config, identity).unwrap();
    launch.model_supports_tool_call = true;
    assert!(configure(&mut launch, "http://127.0.0.1:12345").is_err());
}

#[test]
fn acp_onboarding_grok_restore_republishes_generated_settings() {
    let root = tempfile::tempdir().unwrap();
    let mut launch = profile(&["agent", "stdio"]);
    launch.state_root = Some(root.path().to_path_buf());
    configure(&mut launch, "http://127.0.0.1:12345").unwrap();
    let path = std::path::PathBuf::from(&launch.config().env["GROK_HOME"]).join("config.toml");
    let mut content = std::fs::read_to_string(&path).unwrap();
    content.push_str("\n[ui]\nyolo = true\n");
    std::fs::write(&path, content).unwrap();
    let mut restored = profile(&["agent", "stdio"]);
    restored.state_root = Some(root.path().to_path_buf());
    configure(&mut restored, "http://127.0.0.1:12345").unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(!content.contains("yolo"));
    std::fs::write(&path, content.replace("fixture-model", "other-model")).unwrap();
    let mut changed = profile(&["agent", "stdio"]);
    changed.state_root = Some(root.path().to_path_buf());
    assert!(configure(&mut changed, "http://127.0.0.1:12345").is_err());
}

use super::*;
use crate::model_relay::RelayIdentity;

#[test]
fn acp_model_source_copilot_private_byok_launch_and_revocation() {
    verify_private_byok_launch(false);
}

#[test]
fn acp_model_source_copilot_reasoning_capability_preserves_native_model_routing() {
    verify_private_byok_launch(true);
}

fn verify_private_byok_launch(reasoning: bool) {
    let config = crate::config::schema::AcpAgentConfig {
        command: "copilot".into(),
        args: vec!["--acp".into(), "--stdio".into()],
        ..Default::default()
    };
    let mut profile = AcpLaunchProfile::new(
        &config,
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "p".into(),
            session: format!("copilot-reasoning-{reasoning}"),
            provider: "provider".into(),
            model: "chosen".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    profile.model_supports_temperature = true;
    profile.model_supports_reasoning = reasoning;
    profile.model_api = Some(crate::provider::provider::ApiInfo {
        id: "chosen".into(),
        adapter: "openai-compatible".into(),
        url: "https://example.invalid/v1".into(),
    });
    configure(&mut profile, "http://127.0.0.1:12345").unwrap();
    let env = &profile.config().env;
    assert_eq!(env["COPILOT_MODEL"], "vibewindow/chosen");
    let providers: serde_json::Value =
        serde_json::from_slice(&std::fs::read(profile.directory().join("providers.json")).unwrap())
            .unwrap();
    assert_eq!(providers["providers"][0]["baseUrl"], "http://127.0.0.1:12345/v1");
    assert_eq!(providers["providers"][0]["apiKey"], profile.binding.bearer_token());
    assert_eq!(providers["models"][0]["wireModel"], "chosen");
    assert_eq!(env["COPILOT_OFFLINE"], "true");
    assert!(!profile.config().args.iter().any(|a| a.contains(profile.binding.bearer_token())));
    let directory = profile.directory().to_path_buf();
    assert_eq!(env["COPILOT_HOME"], directory.to_string_lossy());
    let history = std::fs::read_link(directory.join("session-state")).unwrap();
    std::fs::write(history.join("history-probe"), "no credentials").unwrap();
    let binding = profile.binding.clone();
    let token = binding.bearer_token().to_owned();
    let identity = binding.identity.clone();
    drop(profile);
    assert!(!directory.exists());
    assert!(!binding.accepts(&token, &identity));
    assert_eq!(std::fs::read_to_string(history.join("history-probe")).unwrap(), "no credentials");
    let reissued = AcpLaunchProfile::new(&config, identity).unwrap();
    preserve_history(&reissued).unwrap();
    assert_eq!(std::fs::read_link(reissued.directory().join("session-state")).unwrap(), history);
    assert_ne!(reissued.binding.bearer_token(), token);
    drop(reissued);
    std::fs::remove_dir_all(history).unwrap();
}

#[test]
fn acp_model_source_copilot_conflicting_account_environment_is_rejected() {
    for key in
        ["COPILOT_HOME", "COPILOT_PROVIDER_API_KEY_COMMAND", "GH_COPILOT_TOKEN", "GITHUB_TOKEN"]
    {
        let config = crate::config::schema::AcpAgentConfig {
            command: "copilot".into(),
            args: vec!["--acp".into(), "--stdio".into()],
            env: std::collections::HashMap::from([(key.into(), "conflicting".into())]),
            ..Default::default()
        };
        let mut profile = AcpLaunchProfile::new(
            &config,
            RelayIdentity {
                tenant: crate::session::ui_store::current_tenant_scope(),
                project: "p".into(),
                session: "s".into(),
                provider: "provider".into(),
                model: "chosen".into(),
                generation: 0,
            },
        )
        .unwrap();
        assert!(
            configure(&mut profile, "http://127.0.0.1:12345")
                .unwrap_err()
                .to_string()
                .contains("conflicting")
        );
        assert!(!profile.directory().join("providers.json").exists());
        assert_eq!(profile.config().env[key], "conflicting");
    }
}

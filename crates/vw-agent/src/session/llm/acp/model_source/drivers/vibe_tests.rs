use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

fn profile() -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig { command: "vibe-acp".into(), ..Default::default() },
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "chosen-\"模型".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_api = Some(crate::provider::provider::ApiInfo {
        protocols: None,
        id: "selected".into(),
        url: "https://example.invalid/v1".into(),
        adapter: "openai-compatible".into(),
    });
    profile.model_supports_temperature = true;
    profile
}

#[test]
fn acp_model_source_vibe_private_binding_pins_primary_and_auxiliary_models() {
    let mut profile = profile();
    profile.model_context_limit = 1_000_000;
    profile.model_output_limit = 384_000;
    configure(&mut profile, "http://127.0.0.1:12345").unwrap();
    let path = profile.directory().join("session-model.toml");
    let value: toml::Value = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for model in [&value["models"][0], &value["compaction_model"]] {
        assert_eq!(model["max_context_length"].as_integer(), Some(1_000_000));
        assert_eq!(model["max_output_tokens"].as_integer(), Some(384_000));
    }
    assert_eq!(value["active_model"].as_str(), Some("session-selected"));
    assert_eq!(value["models"][0]["name"].as_str(), Some("chosen-\"模型"));
    assert_eq!(value["compaction_model"]["name"].as_str(), Some("chosen-\"模型"));
    assert_eq!(value["utility_models"]["title"].as_str(), Some("session-selected"));
    assert_eq!(value["utility_models"]["smart_approve"].as_str(), Some("session-selected"));
    assert!(profile.config().args.iter().any(|arg| arg == "--session-model-config"));
    assert!(!profile.config().args.join(" ").contains(profile.binding.bearer_token()));
    assert_eq!(profile.config().env["VIBE_HOME"], profile.directory().to_string_lossy());
    let binding = profile.binding.clone();
    let token = binding.bearer_token().to_owned();
    let identity = binding.identity.clone();
    drop(profile);
    assert!(!path.exists());
    assert!(!binding.accepts(&token, &identity));
}

#[test]
fn acp_model_source_vibe_rejects_unsupported_protocol_before_publication() {
    let mut profile = profile();
    profile.model_api.as_mut().unwrap().adapter = "unknown".into();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert!(!profile.directory().join("session-model.toml").exists());
}

#[test]
fn acp_model_source_vibe_rejects_conflicting_native_home() {
    let mut profile = profile();
    profile
        .overlay_environment(std::collections::HashMap::from([(
            "VIBE_HOME".into(),
            "/conflicting-native-config".into(),
        )]))
        .unwrap();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert!(!profile.directory().join("session-model.toml").exists());
}

#[test]
fn acp_model_source_vibe_rejects_temperature_and_missing_metadata() {
    let mut profile = profile();
    profile.model_supports_temperature = false;
    assert!(
        configure(&mut profile, "http://127.0.0.1:12345")
            .unwrap_err()
            .to_string()
            .contains("temperature")
    );
    assert!(!profile.directory().join("session-model.toml").exists());
    profile.model_api = None;
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
}

#[test]
fn acp_model_source_vibe_selects_native_protocol_and_rejects_remote_relay() {
    for (adapter, expected, base) in [
        ("openai-compatible", "openai", "http://127.0.0.1:12345/v1"),
        ("openai-responses", "openai-responses", "http://127.0.0.1:12345/v1"),
        ("anthropic", "anthropic", "http://127.0.0.1:12345"),
    ] {
        let mut profile = profile();
        profile.model_api.as_mut().unwrap().adapter = adapter.into();
        configure(&mut profile, "http://127.0.0.1:12345").unwrap();
        let value: toml::Value = toml::from_str(
            &std::fs::read_to_string(profile.directory().join("session-model.toml")).unwrap(),
        )
        .unwrap();
        assert_eq!(value["providers"][0]["api_style"].as_str(), Some(expected));
        assert_eq!(value["providers"][0]["api_base"].as_str(), Some(base));
        assert!(
            !std::fs::read_to_string(profile.directory().join("session-model.toml"))
                .unwrap()
                .contains("example.invalid")
        );
    }
    let mut profile = profile();
    assert!(configure(&mut profile, "https://example.invalid").is_err());
    assert!(!profile.directory().join("session-model.toml").exists());
}

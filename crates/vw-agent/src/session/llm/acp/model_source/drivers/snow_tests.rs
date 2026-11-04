use super::*;
use crate::model_relay::RelayIdentity;
fn profile(config: &crate::config::schema::AcpAgentConfig) -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        config,
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_api = Some(crate::provider::provider::ApiInfo {
        id: "selected-model".into(),
        url: "https://example.invalid/v1".into(),
        adapter: "openai-compatible".into(),
    });
    profile
}

#[test]
fn acp_model_source_snow_private_config_and_revocation() {
    let config = crate::config::schema::AcpAgentConfig {
        command: "snow".into(),
        args: vec!["--acp".into()],
        ..Default::default()
    };
    let mut profile = profile(&config);
    profile.model_supports_temperature = true;
    profile.model_output_limit = 1024;
    configure(&mut profile, "http://127.0.0.1:12345").unwrap();
    let directory = std::path::PathBuf::from(&profile.config().env["SNOW_CONFIG_DIR"]);
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("config.json")).unwrap()).unwrap();
    assert_eq!(value["snowcfg"]["advancedModel"], "selected-model");
    assert_eq!(value["snowcfg"]["basicModel"], "selected-model");
    assert_eq!(value["snowcfg"]["maxTokens"], 1024);
    assert_eq!(value["snowcfg"]["baseUrl"], "http://127.0.0.1:12345/v1");
    assert!(value["snowcfg"]["apiKey"].as_str() == Some(profile.binding.bearer_token()));
    assert!(value["snowcfg"].get("oauth").is_none());
    assert!(value["snowcfg"].get("visionModel").is_none());
    assert_eq!(profile.config().args, config.args);
    let binding = profile.binding.clone();
    let token = binding.bearer_token().to_owned();
    let identity = binding.identity.clone();
    drop(profile);
    assert!(!directory.exists());
    assert!(!binding.accepts(&token, &identity));
}

#[test]
fn acp_model_source_snow_rejects_unsupported_temperature_before_config_publication() {
    let mut profile = profile(&Default::default());
    let error = configure(&mut profile, "http://127.0.0.1:12345").unwrap_err();
    assert!(error.to_string().contains("supporting temperature"));
    assert!(!profile.config().env.contains_key("SNOW_CONFIG_DIR"));
    assert!(!profile.directory().join("config.json").exists());
}

#[test]
fn acp_model_source_snow_selects_native_protocol_without_changing_upstream_credentials() {
    for (adapter, url, method, suffix) in [
        ("openai-responses", "https://example.invalid/v1", "responses", "/v1"),
        ("openai-compatible", "https://example.invalid/v1/responses", "responses", "/v1"),
        ("anthropic", "https://example.invalid/v1", "anthropic", "/v1"),
        ("google", "https://example.invalid/v1beta", "gemini", "/v1beta"),
    ] {
        let mut profile = profile(&Default::default());
        profile.model_supports_temperature = true;
        let api = profile.model_api.as_mut().unwrap();
        api.adapter = adapter.into();
        api.url = url.into();
        configure(&mut profile, "http://127.0.0.1:12345").unwrap();
        let value: serde_json::Value = serde_json::from_slice(
            &std::fs::read(profile.directory().join("config.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(value["snowcfg"]["requestMethod"], method);
        assert_eq!(value["snowcfg"]["baseUrl"], format!("http://127.0.0.1:12345{suffix}"));
        assert!(!serde_json::to_string(&value).unwrap().contains("example.invalid"));
    }
    let mut profile = profile(&Default::default());
    profile.model_api.as_mut().unwrap().adapter = "unsupported".into();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert!(!profile.directory().join("config.json").exists());
}

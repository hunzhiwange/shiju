use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

fn profile() -> AcpLaunchProfile {
    AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: "/not-installed/openclaw".into(),
            args: vec!["acp".into()],
            ..Default::default()
        },
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap()
}

#[test]
fn acp_model_source_openclaw_config_routes_only_owned_gateway_and_model() {
    let mut launch = profile();
    // Preparing material must not require or start an installed Gateway process.
    configure(&mut launch, "http://127.0.0.1:32123").unwrap();
    let config: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&launch.config().env["OPENCLAW_CONFIG_PATH"]).unwrap(),
    )
    .unwrap();
    assert_eq!(
        config["models"]["providers"]["session-provider"]["baseUrl"],
        "http://127.0.0.1:32123/v1"
    );
    assert_eq!(config["agents"]["defaults"]["model"]["primary"], "session-provider/selected-model");
    assert_eq!(config["agents"]["defaults"]["model"]["fallbacks"], serde_json::json!([]));
    assert_eq!(config["models"]["providers"].as_object().unwrap().len(), 1);
    let token = launch.binding.bearer_token();
    assert!(!serde_json::to_string(&config).unwrap().contains(token));
    assert!(launch.config().args.iter().all(|arg| !arg.contains(token)));
    assert!(launch.config().args.contains(&"--token-file".into()));
    assert_ne!(
        launch.config().env["OPENCLAW_STATE_DIR"],
        std::env::var("HOME").unwrap_or_default()
    );
}

#[test]
fn acp_model_source_openclaw_rejects_remote_gateway_and_conflicting_configuration() {
    for endpoint in
        ["https://remote.example", "http://127.0.0.1:32123/path", "http://user@127.0.0.1:32123"]
    {
        assert!(configure(&mut profile(), endpoint).is_err());
    }
    let mut launch = profile();
    launch
        .overlay_environment(std::collections::HashMap::from([(
            "OPENCLAW_CONFIG_PATH".into(),
            "/conflicting/config".into(),
        )]))
        .unwrap();
    assert!(configure(&mut launch, "http://127.0.0.1:32123").is_err());
    let mut launch = profile();
    launch.append_arguments(&["--url".into(), "ws://remote.example".into()]).unwrap();
    assert!(configure(&mut launch, "http://127.0.0.1:32123").is_err());
}

#[test]
fn acp_model_source_openclaw_reissue_preserves_history_without_persisting_credentials() {
    let root = tempfile::tempdir().unwrap();
    let mut first = profile();
    first.state_root = Some(root.path().to_owned());
    configure(&mut first, "http://127.0.0.1:32123").unwrap();
    let read = |launch: &AcpLaunchProfile| -> serde_json::Value {
        serde_json::from_slice(
            &std::fs::read(&launch.config().env["OPENCLAW_CONFIG_PATH"]).unwrap(),
        )
        .unwrap()
    };
    let first_config = read(&first);
    let old_token = first.binding.bearer_token().to_owned();
    let old_binding = first.binding.clone();
    let runtime = first.directory().to_owned();
    drop(first);
    assert!(!runtime.exists());
    assert!(old_binding.cancelled.is_cancelled());
    let mut second = profile();
    second.state_root = Some(root.path().to_owned());
    second.model_alias = "switched-model".into();
    configure(&mut second, "http://127.0.0.1:32124").unwrap();
    let second_config = read(&second);
    assert_eq!(first_config["session"]["store"], second_config["session"]["store"]);
    assert_ne!(old_token, second.binding.bearer_token());
    assert_eq!(
        second_config["agents"]["defaults"]["model"]["primary"],
        "session-provider/switched-model"
    );
    assert!(!second_config.to_string().contains(&old_token));
}

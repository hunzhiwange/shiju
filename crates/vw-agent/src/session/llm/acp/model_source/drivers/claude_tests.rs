use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

fn profile(config: &AcpAgentConfig) -> AcpLaunchProfile {
    let mut config = config.clone();
    if config.command.is_empty() {
        config.command = "claude-agent-acp".into();
    }
    let mut launch = AcpLaunchProfile::new(
        &config,
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
    launch.model_supports_temperature = true;
    launch.model_supports_tool_call = true;
    launch.model_api = Some(crate::provider::provider::ApiInfo {
        id: "selected-model".into(),
        url: "https://example.invalid/v1".into(),
        adapter: "openai-compatible".into(),
    });
    launch
}

#[test]
fn acp_model_source_claude_private_settings_revoke_and_reissue() {
    let config = AcpAgentConfig { command: "claude-agent-acp".into(), ..Default::default() };
    let mut launch = profile(&config);
    configure(&mut launch, "http://127.0.0.1:12345").unwrap();
    let path = launch.config().env["VW_ACP_CLAUDE_SETTINGS"].clone();
    let settings: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let token = launch.binding.bearer_token().to_owned();
    for key in [
        "ANTHROPIC_MODEL",
        "ANTHROPIC_SMALL_FAST_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        "CLAUDE_CODE_SUBAGENT_MODEL",
    ] {
        assert_eq!(settings["env"][key], "selected-model");
    }
    assert_eq!(settings["env"]["ANTHROPIC_BASE_URL"], "http://127.0.0.1:12345");
    assert!(settings["env"]["ANTHROPIC_AUTH_TOKEN"] == token);
    assert_eq!(settings["env"]["MAX_THINKING_TOKENS"], "0");
    assert_eq!(settings["env"]["CLAUDE_CODE_PROVIDER_MANAGED_BY_HOST"], "1");
    assert_eq!(settings["env"]["CLAUDE_CODE_MAX_OUTPUT_TOKENS"], "4096");
    assert_eq!(launch.config().args, config.args);
    assert!(!launch.config().args.iter().any(|a| a.contains(&token)));
    let binding = launch.binding.clone();
    let identity = binding.identity.clone();
    drop(launch);
    assert!(!std::path::Path::new(&path).exists());
    assert!(!binding.accepts(&token, &identity));
    let restored = profile(&config);
    assert!(restored.binding.bearer_token() != token);
    assert_ne!(restored.directory().join("settings.json"), std::path::PathBuf::from(path));
}

#[test]
fn acp_model_source_claude_conflicting_credentials_fail_before_files() {
    let config = AcpAgentConfig {
        env: std::collections::HashMap::from([(
            "ANTHROPIC_AUTH_TOKEN".into(),
            "competing-credential".into(),
        )]),
        ..Default::default()
    };
    let mut launch = profile(&config);
    let error = configure(&mut launch, "http://127.0.0.1:12345").unwrap_err().to_string();
    assert!(!error.contains("competing-credential"));
    assert!(!launch.directory().join("settings.json").exists());
}

#[test]
fn acp_model_source_claude_rejects_nonlocal_relay_and_native_arguments() {
    for endpoint in
        ["https://example.invalid", "http://127.0.0.1:12345/other", "http://token@127.0.0.1:12345"]
    {
        let mut launch = profile(&Default::default());
        assert!(configure(&mut launch, endpoint).is_err());
        assert!(!launch.directory().join("settings.json").exists());
    }
    for arg in ["--cli", "--settings=unsafe.json"] {
        let mut launch = profile(&AcpAgentConfig { args: vec![arg.into()], ..Default::default() });
        assert!(configure(&mut launch, "http://127.0.0.1:12345").is_err());
        assert!(!launch.directory().join("settings.json").exists());
    }
}

#[test]
fn acp_model_source_claude_enterprise_routes_fail_without_secret_diagnostics() {
    for bytes in [
        br#"{"env":{"ANTHROPIC_AUTH_TOKEN":"enterprise-secret"}}"#.as_slice(),
        br#"{"model":"native"}"#.as_slice(),
        br#"{"apiKeyHelper":"helper"}"#.as_slice(),
        br#"{"env":{"https_proxy":"http://proxy.invalid"}}"#.as_slice(),
        b"invalid enterprise-secret",
    ] {
        let error = validate_managed_settings(bytes).unwrap_err().to_string();
        assert!(!error.contains("enterprise-secret"));
    }
    validate_managed_settings(br#"{"permissions":{"deny":["Bash"]},"env":{}}"#).unwrap();
}

#[tokio::test]
async fn acp_model_source_claude_disabled_model_and_missing_credentials_do_not_fallback() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    struct Restore(Option<std::ffi::OsString>);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe {
                match self.0.take() {
                    Some(value) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", value),
                    None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
                }
            }
        }
    }
    let root = tempfile::tempdir().unwrap();
    let _restore = Restore(std::env::var_os("VIBEWINDOW_CONFIG_DIR"));
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", root.path().join("config"));
    }
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, root.path()).await.unwrap();
    // This unique provider has no local account and names an absent credential.
    let missing = format!("VW_CLAUDE_ABSENT_{}", uuid::Uuid::new_v4().simple());
    crate::config::update_project_for_tenant(&tenant, root.path(), serde_json::json!({
        "providers": {"claude-negative-test": {
            "api":"http://127.0.0.1:1/v1/chat/completions", "adapter":"openai-compatible", "env":[missing],
            "models":{"selected-model":{"status":"active","tool_call":true,"temperature":true,"limit":{"context":12000,"output":1024}},"disabled":{"status":"disabled"}}
        }}
    })).await.unwrap();
    let context = crate::project::instance::Context {
        directory: root.path().to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    let config = AcpAgentConfig { command: "claude-agent-acp".into(), ..Default::default() };
    crate::project::instance::provide_context(context, async {
        let active = crate::provider::provider::get_model("claude-negative-test","selected-model").await.unwrap();
        // A persisted active snapshot must not resurrect a now-disabled catalog key.
        let mut disabled = active.clone();
        disabled.id = "disabled".into();
        assert!(super::super::super::prepare("claude",&config,&disabled,"negative").await.is_err());
        let launch = super::super::super::prepare("claude",&config,&active,"negative").await.unwrap();
        assert_eq!(launch.model_context_limit,12000);
        assert_eq!(launch.model_output_limit,1024);
        let response = crate::model_relay::upstream_http_client().unwrap()
            .post(format!("{}/v1/messages",launch.config().env["ANTHROPIC_BASE_URL"]))
            .bearer_auth(launch.binding.bearer_token())
            .json(&serde_json::json!({"model":"selected-model","max_tokens":16,"messages":[{"role":"user","content":"hello"}]}))
            .send().await.unwrap();
        assert_eq!(response.status(),reqwest::StatusCode::UNAUTHORIZED);
    }).await;
}

#[test]
fn acp_model_source_claude_rejects_credentials_in_existing_arguments() {
    let mut launch = profile(&AcpAgentConfig {
        command: "claude-agent-acp".into(),
        args: vec!["--auth-token".into(), "sensitive-native-credential".into()],
        ..Default::default()
    });
    let error = configure(&mut launch, "http://127.0.0.1:12345").unwrap_err().to_string();
    assert!(!error.contains("sensitive-native-credential"));
    assert!(!launch.directory().join("settings.json").exists());
}

#[test]
fn acp_model_source_claude_rejects_unrepresentable_capabilities_before_publication() {
    for mode in ["reasoning", "temperature", "tools", "protocol", "responses"] {
        let mut launch = profile(&Default::default());
        match mode {
            "reasoning" => launch.model_supports_reasoning = true,
            "temperature" => launch.model_supports_temperature = false,
            "tools" => launch.model_supports_tool_call = false,
            "protocol" => launch.model_api.as_mut().unwrap().adapter = "google".into(),
            "responses" => {
                launch.model_api.as_mut().unwrap().url =
                    "https://example.invalid/v1/responses".into()
            }
            _ => unreachable!(),
        }
        assert!(configure(&mut launch, "http://127.0.0.1:12345").is_err());
        assert!(!launch.directory().join("settings.json").exists());
    }
    let mut native = profile(&Default::default());
    native.model_api.as_mut().unwrap().adapter = "anthropic".into();
    native.model_supports_reasoning = true;
    configure(&mut native, "http://127.0.0.1:12345").unwrap();
    assert!(!native.config().env.contains_key("MAX_THINKING_TOKENS"));
}

#[test]
fn acp_model_source_claude_verified_npx_arguments_are_preserved() {
    for package in [
        "@agentclientprotocol/claude-agent-acp@^0.26.0",
        "@agentclientprotocol/claude-agent-acp@0.26.0",
    ] {
        let config = AcpAgentConfig {
            command: "npx".into(),
            args: vec!["-y".into(), package.into()],
            ..Default::default()
        };
        let mut launch = profile(&config);
        configure(&mut launch, "http://127.0.0.1:12345").unwrap();
        assert_eq!(launch.config().args, config.args);
    }
}

#[test]
fn acp_model_source_claude_history_survives_credential_revocation_and_is_scoped() {
    let root = tempfile::tempdir().unwrap();
    let config = AcpAgentConfig::default();
    let mut first = profile(&config);
    first.state_root = Some(root.path().to_path_buf());
    configure(&mut first, "http://127.0.0.1:12345").unwrap();
    let state = std::path::PathBuf::from(&first.config().env["CLAUDE_CONFIG_DIR"]);
    let credentials = first.config().env["VW_ACP_CLAUDE_SETTINGS"].clone();
    let bearer = first.binding.bearer_token().to_owned();
    let binding = first.binding.clone();
    std::fs::write(state.join("history.jsonl"), b"private-history").unwrap();
    drop(first);
    assert!(!std::path::Path::new(&credentials).exists());
    assert!(!binding.accepts(&bearer, &binding.identity));
    let mut second = profile(&config);
    second.state_root = Some(root.path().to_path_buf());
    configure(&mut second, "http://127.0.0.1:12346").unwrap();
    assert_eq!(std::path::Path::new(&second.config().env["CLAUDE_CONFIG_DIR"]), state);
    assert_eq!(std::fs::read(state.join("history.jsonl")).unwrap(), b"private-history");
    assert_ne!(bearer, second.binding.bearer_token());
    assert_ne!(credentials, second.config().env["VW_ACP_CLAUDE_SETTINGS"]);
    assert!(
        !std::fs::read(state.join("history.jsonl"))
            .unwrap()
            .windows(bearer.len())
            .any(|w| w == bearer.as_bytes())
    );
    let mut other = profile(&config);
    other.state_root = Some(root.path().to_path_buf());
    other.binding = crate::model_relay::RelayBinding::new(
        crate::model_relay::RelayIdentity {
            session: "different-session".into(),
            ..other.binding.identity.clone()
        },
        std::time::Duration::from_secs(900),
    )
    .unwrap();
    // A different launch identity receives a different stable state namespace.
    let mut other = AcpLaunchProfile::new(&config, other.binding.identity.clone()).unwrap();
    other.state_root = Some(root.path().to_path_buf());
    assert_ne!(other.claude_state_directory().unwrap(), state);
}

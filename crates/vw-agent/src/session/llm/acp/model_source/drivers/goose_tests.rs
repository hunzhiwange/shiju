use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

fn profile(config: &AcpAgentConfig) -> AcpLaunchProfile {
    AcpLaunchProfile::new(
        config,
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected-model-high".into(),
            generation: 0,
        },
    )
    .unwrap()
}

#[test]
fn acp_model_source_goose_child_environment_is_private_and_revocable() {
    let original =
        AcpAgentConfig { command: "goose".into(), args: vec!["acp".into()], ..Default::default() };
    let mut profile = profile(&original);
    configure(&mut profile, "http://127.0.0.1:12345").unwrap();
    let binding = profile.binding.clone();
    let token = binding.bearer_token().to_owned();
    let identity = binding.identity.clone();
    let directory = profile.directory().to_owned();
    let output = std::process::Command::new("python3")
        .args(["-c", "import os,sys; assert os.environ['GOOSE_PROVIDER']=='openai'; assert os.environ['GOOSE_MODEL']=='selected-model-high'; assert os.environ['SHIJU_GOOSE_MODEL_LOCK']==os.environ['GOOSE_MODEL']; assert os.environ['OPENAI_HOST']=='http://127.0.0.1:12345'; assert os.environ['OPENAI_BASE_PATH']=='v1/chat/completions'; assert os.environ['OPENAI_CUSTOM_HEADERS']==''; assert os.path.isabs(os.environ['GOOSE_PATH_ROOT']); assert os.stat(os.environ['GOOSE_PATH_ROOT']).st_mode & 0o777 == 0o700; assert os.environ['OPENAI_API_KEY']; assert sys.argv[1:]==['acp','--shiju-model-source']; print('PRIVATE_GOOSE_LAUNCH_OK')"])
        .args(&profile.config().args)
        .envs(&profile.config().env)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "PRIVATE_GOOSE_LAUNCH_OK");
    assert!(!profile.config().args.iter().any(|arg| arg.contains(&token)));
    assert_eq!(original.args, ["acp"]);
    assert!(original.env.is_empty());
    assert!(binding.accepts(&token, &identity));
    drop(profile);
    assert!(!directory.exists());
    assert!(!binding.accepts(&token, &identity));
}

#[test]
fn acp_model_source_goose_rejects_conflicting_overrides_without_secrets() {
    for key in [
        "GOOSE_PATH_ROOT",
        "GOOSE_PROVIDER",
        "GOOSE_MODEL",
        "OPENAI_HOST",
        "OPENAI_API_KEY",
        "OPENAI_CUSTOM_HEADERS",
        "SHIJU_GOOSE_MODEL_LOCK",
    ] {
        let original = AcpAgentConfig {
            command: "goose".into(),
            args: vec!["acp".into()],
            env: std::collections::HashMap::from([(
                key.to_lowercase(),
                "conflicting-secret".into(),
            )]),
        };
        let mut profile = profile(&original);
        let error = configure(&mut profile, "http://127.0.0.1:12345").unwrap_err().to_string();
        assert!(!error.contains("conflicting-secret"));
        assert!(!error.contains(profile.binding.bearer_token()));
        assert_eq!(profile.config().args, original.args);
        assert_eq!(profile.config().env.len(), original.env.len() + 1);
    }
}

#[test]
fn acp_model_source_goose_rejects_non_acp_command_and_non_loopback_relay() {
    for args in [vec![], vec!["serve".into()], vec!["acp".into(), "--shiju-model-source".into()]] {
        let mut profile =
            profile(&AcpAgentConfig { command: "goose".into(), args, ..Default::default() });
        assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
        assert!(!profile.config().env.contains_key("GOOSE_PATH_ROOT"));
    }
    for endpoint in [
        "https://example.invalid",
        "http://localhost:12345",
        "http://127.0.0.1:12345/path",
        "http://user:secret@127.0.0.1:12345",
    ] {
        let mut profile = profile(&AcpAgentConfig {
            command: "goose".into(),
            args: vec!["acp".into()],
            ..Default::default()
        });
        assert!(configure(&mut profile, endpoint).is_err());
        assert!(!profile.config().env.contains_key("GOOSE_PATH_ROOT"));
    }
}

#[tokio::test]
async fn acp_model_source_goose_missing_credentials_and_disabled_model_never_use_native_account() {
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
    crate::config::update_project_for_tenant(
        &tenant,
        root.path(),
        serde_json::json!({
            "providers": {"goose-offline-no-account": {
                "api": "http://127.0.0.1:1/v1/chat/completions",
                "adapter":"openai-compatible", "env":[],
                "models": {
                    "active-model":{"status":"active","tool_call":true},
                    "disabled-model":{"status":"disabled","tool_call":true}
                }
            }}
        }),
    )
    .await
    .unwrap();
    let context = crate::project::instance::Context {
        directory: root.path().to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    let relay_context = context.clone();
    crate::project::instance::provide_context(context, async {
        let config = AcpAgentConfig { command:"goose".into(), args:vec!["acp".into()], ..Default::default() };
        let active = crate::provider::provider::get_model("goose-offline-no-account", "active-model").await.unwrap();
        // Exercise the real relay/configuration boundary without requiring an installed CLI.
        // Supplier executable identity is covered separately by real-CLI acceptance.
        let mut profile = AcpLaunchProfile::new(&config, RelayIdentity {
            tenant: tenant.clone(), project: relay_context.project.id.clone(),
            session: "missing-key".into(), provider: active.provider_id.clone(),
            model: active.id.clone(), generation: 0,
        }).unwrap();
        profile.model_api = Some(active.api.clone());
        let server = crate::model_relay::RelayServer::start(
            profile.binding.clone(), std::sync::Arc::new(crate::model_relay::ProviderRelayBackend::new().unwrap()), relay_context,
        ).await.unwrap();
        let relay_endpoint = format!("http://{}", server.address());
        profile.attach_server(server);
        configure(&mut profile, &relay_endpoint).unwrap();
        let endpoint = &profile.config().env["OPENAI_HOST"];
        let response = crate::model_relay::upstream_http_client().unwrap()
            .post(format!("{endpoint}/v1/chat/completions"))
            .bearer_auth(profile.binding.bearer_token())
            .json(&serde_json::json!({"model":"active-model","messages":[{"role":"user","content":"hello"}]}))
            .send().await.unwrap();
        assert_eq!(response.status(), 401);
        // A stale caller snapshot cannot resurrect a catalog-disabled model.
        let mut disabled = active.clone();
        disabled.id = "disabled-model".into();
        let result = crate::session::llm::acp::model_source::prepare("goose", &config, &disabled, "disabled").await;
        let error = match result {
            Ok(_) => panic!("disabled model must not create a Goose launch"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("disabled-model"));
    }).await;
}

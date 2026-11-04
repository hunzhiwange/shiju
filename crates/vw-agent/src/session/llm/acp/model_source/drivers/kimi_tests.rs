use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

fn profile(model: &str) -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig { command: "kimi".into(), args: vec!["acp".into()], ..Default::default() },
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: model.into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_api = Some(crate::provider::provider::ApiInfo {
        id: model.into(),
        url: "https://example.invalid/v1/chat/completions".into(),
        adapter: "openai-compatible".into(),
    });
    profile.model_supports_tool_call = true;
    profile
}

#[test]
fn acp_model_source_kimi_child_reads_private_route_without_argv_credentials() {
    let mut profile = profile("selected-model");
    configure(&mut profile, "http://127.0.0.1:12345").unwrap();
    let output = std::process::Command::new("python3")
        .args(["-c", "import os,json,pathlib; p=pathlib.Path(os.environ['KIMI_CODE_HOME'])/'config.toml'; s=p.read_text(); assert 'selected-model' in s; assert 'http://127.0.0.1:12345/v1' in s; assert 'api_key_env' in s; assert os.environ['VW_KIMI_RELAY_TOKEN'] not in s; print('BOUND')"])
        .envs(&profile.config().env).output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"BOUND\n");
    assert_eq!(profile.config().args, ["acp"]);
    assert!(!profile.config().args.iter().any(|arg| arg.contains(profile.binding.bearer_token())));
    let binding = profile.binding.clone();
    let token = binding.bearer_token().to_owned();
    drop(profile);
    assert!(!binding.accepts(&token, &binding.identity));
}

#[test]
fn acp_model_source_kimi_reissue_preserves_history_without_persisting_credentials() {
    let state = tempfile::tempdir().unwrap();
    let mut first = profile("first");
    first.state_root = Some(state.path().to_owned());
    configure(&mut first, "http://127.0.0.1:12345").unwrap();
    let home = std::path::PathBuf::from(&first.config().env["KIMI_CODE_HOME"]);
    std::fs::write(home.join("history-probe"), b"history").unwrap();
    let token = first.binding.bearer_token().to_owned();
    drop(first);
    let mut next = profile("second");
    next.state_root = Some(state.path().to_owned());
    configure(&mut next, "http://127.0.0.1:12346").unwrap();
    assert_eq!(next.config().env["KIMI_CODE_HOME"], home.to_string_lossy());
    assert_eq!(std::fs::read(home.join("history-probe")).unwrap(), b"history");
    let content = std::fs::read_to_string(home.join("config.toml")).unwrap();
    assert!(content.contains("second"));
    assert!(!content.contains(&token));
    assert!(!content.contains(next.binding.bearer_token()));
    assert_ne!(next.binding.bearer_token(), token);
}

#[test]
fn acp_model_source_kimi_conflicts_and_unsupported_routes_fail_before_publication() {
    let mut profile = profile("selected");
    profile.append_arguments(&["--login".into()]).unwrap();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    let mut profile = self::profile("selected");
    profile
        .overlay_environment(std::collections::HashMap::from([(
            "KIMI_CODE_HOME".into(),
            "/native".into(),
        )]))
        .unwrap();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    let mut profile = self::profile("selected");
    assert!(configure(&mut profile, "https://example.invalid").is_err());
    profile.model_api = None;
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
}

#[tokio::test]
async fn acp_model_source_kimi_missing_credentials_and_disabled_model_never_use_native_account() {
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
            "providers": {"kimi-offline-no-account": {
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
    crate::project::instance::provide_context(context, async {
        let config = AcpAgentConfig { command:"kimi".into(), args:vec!["acp".into()], ..Default::default() };
        let active = crate::provider::provider::get_model("kimi-offline-no-account", "active-model").await.unwrap();
        let profile = crate::session::llm::acp::model_source::prepare("kimi", &config, &active, "missing-key").await.unwrap();
        let settings: toml::Value = toml::from_str(&std::fs::read_to_string(std::path::Path::new(&profile.config().env["KIMI_CODE_HOME"]).join("config.toml")).unwrap()).unwrap();
        let endpoint = settings["providers"]["session-provider"]["base_url"].as_str().unwrap();
        let response = crate::model_relay::upstream_http_client().unwrap()
            .post(format!("{endpoint}/chat/completions"))
            .bearer_auth(profile.binding.bearer_token())
            .json(&serde_json::json!({"model":"active-model","messages":[{"role":"user","content":"hello"}]}))
            .send().await.unwrap();
        assert_eq!(response.status(), 401);
        // A stale caller snapshot cannot resurrect a catalog-disabled model.
        let mut disabled = active.clone();
        disabled.id = "disabled-model".into();
        let result = crate::session::llm::acp::model_source::prepare("kimi", &config, &disabled, "disabled").await;
        let error = match result {
            Ok(_) => panic!("disabled model must not create a Kimi launch"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("disabled-model"));
    }).await;
}

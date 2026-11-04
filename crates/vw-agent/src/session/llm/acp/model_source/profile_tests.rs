use super::*;
use std::collections::HashMap;
fn identity(session: &str) -> RelayIdentity {
    RelayIdentity {
        tenant: crate::session::ui_store::current_tenant_scope(),
        project: "p".into(),
        session: session.into(),
        provider: "provider".into(),
        model: "model".into(),
        generation: 0,
    }
}

#[test]
fn acp_model_source_launch_profile_private_files_child_env_and_revocation() {
    let config = AcpAgentConfig { command: "python3".into(), ..Default::default() };
    let mut profile = AcpLaunchProfile::new(&config, identity("one")).unwrap();
    let binding = profile.binding.clone();
    let token = binding.bearer_token().to_owned();
    let directory = profile.directory().to_owned();
    let path = profile.write_config("config.json", token.as_bytes()).unwrap();
    assert!(profile.write_config("config.json", b"overwrite").is_err());
    assert!(profile.write_config("../escape", b"bad").is_err());
    assert!(profile.overlay_environment(HashMap::from([("HOME".into(), "/tmp".into())])).is_err());
    profile
        .overlay_environment(HashMap::from([("TASK9_LAUNCH_TOKEN".into(), token.clone())]))
        .unwrap();
    assert!(
        profile
            .overlay_environment(HashMap::from([("TASK9_LAUNCH_TOKEN".into(), "other".into())]))
            .is_err()
    );
    assert!(profile.append_arguments(&[token.clone()]).is_err());
    profile.append_arguments(&["--config".into(), path.to_string_lossy().into()]).unwrap();
    let output = std::process::Command::new("python3")
        .args(["-c", "import os,sys; assert open(sys.argv[1]).read()==os.environ['TASK9_LAUNCH_TOKEN']; print('PRIVATE_LAUNCH_OK')"])
        .arg(&path).envs(&profile.config().env).output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "PRIVATE_LAUNCH_OK");
    assert!(std::env::var("TASK9_LAUNCH_TOKEN").is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::metadata(&directory).unwrap().permissions().mode() & 0o777, 0o700);
    }
    assert!(binding.accepts(&token, &identity("one")));
    assert!(!binding.accepts(&token, &identity("two")));
    drop(profile);
    assert!(!directory.exists());
    assert!(!binding.accepts(&token, &identity("one")));
    let restored = AcpLaunchProfile::new(&config, identity("one")).unwrap();
    assert_ne!(token, restored.binding.bearer_token());
    assert!(!restored.binding.accepts(&token, &identity("one")));
}

#[test]
fn acp_model_source_launch_profile_all_drivers_fail_closed_without_secret_errors() {
    let mut profile = AcpLaunchProfile::new(&Default::default(), identity("one")).unwrap();
    for key in [
        "auggie",
        "claude",
        "codex",
        "codebuddy",
        "goose",
        "vibe",
        "hermes",
        "snow",
        "copilot",
        "cursor",
        "droid",
        "gemini",
        "iflow",
        "kilocode",
        "kimi",
        "kiro",
        "opencode",
        "openclaw",
        "pi",
        "qoder",
        "qwen",
        "trae",
    ] {
        let error =
            drivers::configure(key, &mut profile, "http://127.0.0.1:1").unwrap_err().to_string();
        assert!(error.contains("has not been verified"));
        assert!(!error.contains(profile.binding.bearer_token()));
    }
}

struct ModelService;
#[async_trait::async_trait]
impl crate::model_relay::RelayBackend for ModelService {
    async fn execute(
        &self,
        _: &RelayIdentity,
        _: crate::model_relay::RelayRequest,
        _: tokio_util::sync::CancellationToken,
    ) -> Result<axum::response::Response, axum::http::StatusCode> {
        Ok(axum::response::Response::new(axum::body::Body::from("{\"ok\":true}")))
    }
}

#[tokio::test]
async fn acp_model_source_launch_profile_http_reissue_isolates_old_credentials() {
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
    crate::config::update_project_for_tenant(&tenant, root.path(), serde_json::json!({
        "providers": {"provider": {"api": "http://127.0.0.1:1/v1", "models": {"model": {"status": "active"}}}}
    })).await.unwrap();
    let context = instance::Context {
        directory: root.path().to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project: project.clone(),
    };
    let mut id = identity("http-owner");
    id.project = project.id;
    let mut profile = AcpLaunchProfile::new(&Default::default(), id.clone()).unwrap();
    let token = profile.binding.bearer_token().to_owned();
    let server = RelayServer::start(
        profile.binding.clone(),
        std::sync::Arc::new(ModelService),
        context.clone(),
    )
    .await
    .unwrap();
    let url = format!("http://{}/v1/chat/completions", server.address());
    profile.attach_server(server);
    let client = crate::model_relay::upstream_http_client().unwrap();
    let body =
        serde_json::json!({"model": "model", "messages": [{"role": "user", "content": "hello"}]});
    assert_eq!(
        client.post(&url).bearer_auth(&token).json(&body).send().await.unwrap().status(),
        200
    );
    let reference = profile.config().env[profile::REFERENCE_ENV].clone();
    drop(profile);
    let mut restored = AcpLaunchProfile::new(&Default::default(), id).unwrap();
    assert_eq!(reference, restored.config().env[profile::REFERENCE_ENV]);
    let server =
        RelayServer::start(restored.binding.clone(), std::sync::Arc::new(ModelService), context)
            .await
            .unwrap();
    let url = format!("http://{}/v1/chat/completions", server.address());
    restored.attach_server(server);
    assert_eq!(
        client.post(&url).bearer_auth(token).json(&body).send().await.unwrap().status(),
        401
    );
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(restored.binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
}

#[test]
fn acp_model_source_launch_profile_references_separate_every_identity_boundary() {
    let original = identity("one");
    let profile = AcpLaunchProfile::new(&Default::default(), original.clone()).unwrap();
    let reference = &profile.config().env[profile::REFERENCE_ENV];
    for boundary in 0..6 {
        let mut changed = original.clone();
        match boundary {
            0 => changed.session = "two".into(),
            1 => changed.project = "other-project".into(),
            2 => changed.provider = "other-provider".into(),
            3 => changed.model = "other-model".into(),
            4 => changed.generation += 1,
            _ => changed.tenant = vw_api_types::id::TenantScope::new("other", "tenant").unwrap(),
        }
        let other = AcpLaunchProfile::new(&Default::default(), changed).unwrap();
        assert_ne!(reference, &other.config().env[profile::REFERENCE_ENV]);
    }
}

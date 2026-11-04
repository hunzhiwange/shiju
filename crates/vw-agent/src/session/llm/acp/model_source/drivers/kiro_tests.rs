use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

fn profile(model: &str) -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: "kiro-cli".into(),
            args: vec!["acp".into()],
            ..Default::default()
        },
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "session:kiro".into(),
            provider: "provider".into(),
            model: model.into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    profile.model_api = Some(crate::provider::provider::ApiInfo {
        id: model.into(),
        url: "https://example.invalid/v1/chat/completions".into(),
        adapter: "openai-compatible".into(),
    });
    profile
}

#[test]
fn acp_model_source_kiro_child_reads_private_model_without_persisted_credentials() {
    let mut profile = profile("selected-model");
    configure(&mut profile, "http://127.0.0.1:12345").unwrap();
    let output = std::process::Command::new("python3")
        .args([
            "-c",
            "import os,json,pathlib; p=pathlib.Path(os.environ['KIRO_HOME'])/'settings/cli.json'; s=p.read_text(); c=json.loads(s); assert c['chat.defaultModel']=='selected-model'; assert c['acp.agentEngine']=='v3'; assert os.environ['KIRO_API_KEY'] not in s; assert os.environ['KIRO_KAS_ENDPOINT']=='http://127.0.0.1:12345'; assert os.environ['KIRO_KAS_CONTROL_PLANE_ENDPOINT']==os.environ['KIRO_KAS_ENDPOINT']; print('BOUND')",
        ])
        .envs(&profile.config().env)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"BOUND\n");
    assert!(!profile.config().args.iter().any(|arg| arg.contains(profile.binding.bearer_token())));
    let binding = profile.binding.clone();
    let token = binding.bearer_token().to_owned();
    drop(profile);
    assert!(!binding.accepts(&token, &binding.identity));
}

#[test]
fn acp_model_source_kiro_rotation_preserves_history_and_replaces_model() {
    let state = tempfile::tempdir().unwrap();
    let mut first = profile("first");
    first.state_root = Some(state.path().to_owned());
    configure(&mut first, "http://127.0.0.1:12345").unwrap();
    let home = std::path::PathBuf::from(&first.config().env["KIRO_HOME"]);
    std::fs::write(home.join("history-probe"), b"history").unwrap();
    let old_token = first.binding.bearer_token().to_owned();
    drop(first);
    let mut next = profile("second");
    next.state_root = Some(state.path().to_owned());
    configure(&mut next, "http://127.0.0.1:12346").unwrap();
    assert_eq!(next.config().env["KIRO_HOME"], home.to_string_lossy());
    assert_eq!(std::fs::read(home.join("history-probe")).unwrap(), b"history");
    let settings = std::fs::read_to_string(home.join("settings/cli.json")).unwrap();
    assert!(settings.contains("second"));
    assert!(!settings.contains(&old_token));
    assert!(!settings.contains(next.binding.bearer_token()));
    assert_ne!(old_token, next.binding.bearer_token());
}

#[test]
fn acp_model_source_kiro_conflicts_fail_before_publication() {
    for endpoint in ["https://example.invalid", "http://127.0.0.1:12345/path"] {
        let mut profile = profile("selected");
        assert!(configure(&mut profile, endpoint).is_err());
        assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
    }
    let mut profile = profile("selected");
    profile.append_arguments(&["--model".into(), "native".into()]).unwrap();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    let mut profile = self::profile("selected");
    profile
        .overlay_environment(std::collections::HashMap::from([(
            "KIRO_HOME".into(),
            "/native".into(),
        )]))
        .unwrap();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    let mut profile = self::profile("selected");
    profile.model_api = None;
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    let mut profile = self::profile("selected");
    profile.model_api.as_mut().unwrap().adapter = "google".into();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
}

#[test]
fn acp_model_source_kiro_control_is_authenticated_bound_and_revocable() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    use crate::{
        model_relay::{RelayBackend, RelayRequest, RelayServer},
        project::instance,
    };
    use axum::{body::Body, http::StatusCode, response::Response};
    use std::{sync::Arc, time::Duration};
    struct ExternalModel;
    #[async_trait::async_trait]
    impl RelayBackend for ExternalModel {
        async fn execute(
            &self,
            _: &RelayIdentity,
            request: RelayRequest,
            _: tokio_util::sync::CancellationToken,
        ) -> Result<Response, StatusCode> {
            assert_eq!(request.path, "/v1/responses");
            let value: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            assert!(value.get("origin").is_none());
            assert!(value.get("metadata").is_none());
            Ok(Response::new(Body::from("MODEL_BOUND")))
        }
    }
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = tempfile::tempdir().unwrap();
    struct EnvGuard(Option<std::ffi::OsString>);
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                match self.0.take() {
                    Some(value) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", value),
                    None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
                }
            }
        }
    }
    let _env = EnvGuard(std::env::var_os("VIBEWINDOW_CONFIG_DIR"));
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", temp.path().join("config"));
    }
    let tenant = vw_api_types::id::TenantScope::new("kiro", "control").unwrap();
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let (project, _) = crate::project::from_directory(&tenant, &root).await.unwrap();
    crate::config::update_project_for_tenant(&tenant, &root, serde_json::json!({"providers":{"kiro-fixture":{"api":"http://127.0.0.1:1/v1","models":{"selected":{"status":"active"}}}}})).await.unwrap();
    let binding = crate::model_relay::RelayBinding::new(
        RelayIdentity {
            tenant,
            project: project.id.clone(),
            session: "control:kiro".into(),
            provider: "kiro-fixture".into(),
            model: "selected".into(),
            generation: 0,
        },
        Duration::from_secs(60),
    )
    .unwrap();
    let server = RelayServer::start(
        binding.clone(),
        Arc::new(ExternalModel),
        instance::Context {
            directory: root.to_string_lossy().into(),
            worktree: project.worktree.clone(),
            project,
        },
    )
    .await
    .unwrap();
    let url = format!("http://{}", server.address());
    let client = crate::model_relay::upstream_http_client().unwrap();
    assert_eq!(client.post(&url).json(&serde_json::json!({})).send().await.unwrap().status(), 401);
    let response: serde_json::Value = client
        .post(&url)
        .bearer_auth(binding.bearer_token())
        .header("x-amz-target", "KiroControlPlaneBearerService.ListAvailableModels")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(response["models"].as_array().unwrap().len(), 1);
    assert_eq!(response["models"][0]["modelId"], "selected");
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .header("x-amz-target", "UnknownOperation")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .status(),
        422
    );
    let response = client.post(format!("{url}/v1/responses")).bearer_auth(binding.bearer_token())
        .json(&serde_json::json!({"model":"selected","input":"hi","origin":"AI_EDITOR","metadata":{"kiro:conversation_id":"local"}}))
        .send().await.unwrap();
    assert_eq!(response.text().await.unwrap(), "MODEL_BOUND");
    crate::config::update_project_for_tenant(&binding.identity.tenant, &root, serde_json::json!({"providers":{"kiro-fixture":{"models":{"selected":{"status":"disabled"}}}}})).await.unwrap();
    assert_eq!(client.post(&url).bearer_auth(binding.bearer_token())
        .header("x-amz-target","KiroControlPlaneBearerService.ListAvailableModels")
        .json(&serde_json::json!({})).send().await.unwrap().status(),403);
    binding.revoke();
    if let Ok(response) = client.post(&url).bearer_auth(binding.bearer_token())
        .header("x-amz-target","KiroControlPlaneBearerService.ListAvailableModels")
        .json(&serde_json::json!({})).send().await {
        assert_eq!(response.status(),401);
    }
    server.shutdown().await.unwrap();

        });
    }).unwrap().join().unwrap();
}

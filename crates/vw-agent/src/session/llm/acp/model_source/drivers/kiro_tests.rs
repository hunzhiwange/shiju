use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};



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
    let tenant = vw_api_transport::TenantScope::new("kiro", "control").unwrap();
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
    crate::config::update_project_for_tenant(&binding.identity().tenant, &root, serde_json::json!({"providers":{"kiro-fixture":{"models":{"selected":{"status":"disabled"}}}}})).await.unwrap();
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

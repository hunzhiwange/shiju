use super::*;
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
    llm_runtime::acp::acceptance_support::profile::http_reissue(id, |binding| {
        let context = context.clone();
        async move { RelayServer::start(binding, std::sync::Arc::new(ModelService), context).await.map(Into::into) }
    }).await;
}

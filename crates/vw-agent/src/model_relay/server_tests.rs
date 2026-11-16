use super::*;
use model_relay_http::relay_test_support::{self as checks, RelayHarness};
use std::{sync::Arc, path::PathBuf};
struct EnvGuard(Option<std::ffi::OsString>);
impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match self.0.take() {
                Some(v) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", v),
                None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
            }
        }
    }
}


struct HostHarness { context: crate::project::instance::Context, root: PathBuf, id: RelayIdentity }
#[async_trait::async_trait]
impl RelayHarness for HostHarness {
    fn identity(&self) -> RelayIdentity { self.id.clone() }
    fn scope_assertion(&self) -> fn(&RelayIdentity) { |identity| {
        assert_eq!(crate::session::ui_store::current_tenant_scope(), identity.tenant);
        assert_eq!(crate::project::instance::project().unwrap().id, identity.project);
    } }
    async fn start(&self, binding: Arc<RelayBinding>, backend: Arc<dyn RelayBackend>) -> model_relay_http::RelayServer {
        RelayServer::start(binding, backend, self.context.clone()).await.unwrap().into()
    }
    async fn configure(&self, patch: serde_json::Value) {
        crate::config::update_project_for_tenant(&self.id.tenant, &self.root, patch).await.unwrap();
    }
}
async fn harness(root: PathBuf, session: &str, namespace: &str) -> HostHarness {
    let tenant = vw_api_transport::TenantScope::new(namespace, if session == "flag:kiro" { "local-flag" } else { "one" }).unwrap();
    std::fs::create_dir(&root).unwrap();
    let (project, _) = crate::project::from_directory(&tenant, &root).await.unwrap();
    let context = crate::project::instance::Context {
        directory: root.to_string_lossy().into(), worktree: project.worktree.clone(), project: project.clone(),
    };
    let id = RelayIdentity { tenant, project: project.id, session: session.into(), provider: if session == "flag:kiro" { "missing-provider" } else { "relay-fixture" }.into(), model: if session == "flag:kiro" { "missing-model" } else { "model" }.into(), generation: 1 };
    let harness = HostHarness { context, root, id };
    if session != "flag:kiro" {
        harness.configure(serde_json::json!({"providers":{"relay-fixture":{"api":"http://127.0.0.1:1/v1","models":{"model":{"status":"active"}}}}})).await;
    }
    harness
}

#[tokio::test]
async fn kiro_protocol_flag_is_local_but_catalog_still_requires_selected_model() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let dir = tempfile::tempdir().unwrap();
    let _env = EnvGuard(std::env::var_os("VIBEWINDOW_CONFIG_DIR"));
    unsafe { std::env::set_var("VIBEWINDOW_CONFIG_DIR", dir.path().join("config")) };
    let host = harness(dir.path().join("project"), "flag:kiro", "kiro").await;
    checks::kiro_protocol_flag_is_local_but_catalog_still_requires_selected_model(&host).await;
}

#[tokio::test]
async fn acp_model_source_relay_binding_http_isolation_limits_and_lifecycle() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let dir = tempfile::tempdir().unwrap();
    let _env = EnvGuard(std::env::var_os("VIBEWINDOW_CONFIG_DIR"));
    unsafe { std::env::set_var("VIBEWINDOW_CONFIG_DIR", dir.path().join("config")) };
    let host = harness(dir.path().join("project"), "one", "relay").await;
    checks::acp_model_source_relay_binding_http_isolation_limits_and_lifecycle(&host).await;
}

#[tokio::test]
async fn acp_model_source_relay_expiry_cancels_an_established_http_stream() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let dir = tempfile::tempdir().unwrap();
    let _env = EnvGuard(std::env::var_os("VIBEWINDOW_CONFIG_DIR"));
    unsafe { std::env::set_var("VIBEWINDOW_CONFIG_DIR", dir.path().join("config")) };
    let host = harness(dir.path().join("project"), "one", "relay-expiry").await;
    checks::acp_model_source_relay_expiry_cancels_an_established_http_stream(&host).await;
}

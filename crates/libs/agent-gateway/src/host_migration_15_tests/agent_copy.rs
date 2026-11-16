//! Agent-copy domain tests use the real tenant configuration database.
use axum::{Json, extract::{Extension, Path, Query}, http::HeaderMap};
use gateway_misc::host::{MiscHost, HostFuture, InstanceQuery};
use gateway_misc::{AgentInfo, SkillInfo};
use gateway_types::ApiError;
use vw_api_transport::TenantScope;
struct CopyHost;
impl MiscHost for CopyHost {
    fn get_global(tenant: TenantScope) -> HostFuture<anyhow::Result<agent_config::runtime::Config>> {
        Box::pin(async move { agent_config::runtime::get_global_for_tenant(&tenant).await.map_err(Into::into) })
    }
    fn update_global(tenant: TenantScope, patch: serde_json::Value) -> HostFuture<anyhow::Result<()>> {
        Box::pin(async move { agent_config::runtime::update_global_for_tenant(&tenant, patch).await.map_err(Into::into) })
    }
    fn resolve_directory(_: &InstanceQuery, _: &HeaderMap) -> String { panic!("copy must not select project storage") }
    fn with_instance<T: Send + 'static>(_: String, _: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static) -> HostFuture<Result<T, ApiError>> { panic!("copy must not enter a project instance") }
    fn with_tenant_scope<T: Send + 'static>(_: TenantScope, _: impl std::future::Future<Output=T> + Send + 'static) -> HostFuture<T> { panic!("copy receives tenant explicitly") }
    fn commands() -> HostFuture<Vec<agent_commands::Info>> { panic!("copy does not list commands") }
    fn agents() -> HostFuture<Vec<AgentInfo>> { panic!("copy uses persisted config") }
    fn skills() -> HostFuture<Vec<SkillInfo>> { panic!("copy does not list skills") }
    fn get_current() -> HostFuture<anyhow::Result<agent_config::runtime::Config>> { panic!("copy uses explicit tenant config") }
}
fn query_for(path: &std::path::Path) -> Query<InstanceQuery> {
    Query(InstanceQuery { directory: Some(path.to_string_lossy().into()) })
}
struct ProfileEnvGuard(&'static str, Option<std::ffi::OsString>);
impl ProfileEnvGuard {
    fn set(key: &'static str, value: &std::path::Path) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value);
        }
        Self(key, previous)
    }
}
impl Drop for ProfileEnvGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.1 {
                Some(value) => std::env::set_var(self.0, value),
                None => std::env::remove_var(self.0),
            }
        }
    }
}

#[tokio::test]
async fn agent_profile_api_copy_has_unique_custom_identity_and_survives_reload() {
    let _env_lock = super::global_async_env_lock().lock().await;
    let home = tempfile::tempdir().unwrap();
    let _home = ProfileEnvGuard::set("HOME", home.path());
    let _config = ProfileEnvGuard::set("VIBEWINDOW_CONFIG_DIR", home.path());
    let tenant = vw_api_transport::TenantScope::new("profile-copy", "company").unwrap();
    let mut keys = std::collections::HashSet::new();
    for _ in 0..2 {
        let Json(copy) = gateway_misc::agent_copy::<CopyHost>(
            Extension(tenant.clone()),
            Path("acp-claude".into()),
            Query(InstanceQuery { directory: None }),
            HeaderMap::new(),
        )
        .await
        .unwrap();
        assert!(copy.key.starts_with("custom-"));
        assert!(keys.insert(copy.key.clone()));
        assert!(!copy.definition.builtin);
        let saved = agent_config::runtime::get_global_for_tenant(&tenant).await.unwrap();
        assert!(saved.agents.contains_key(&copy.key));
        assert_eq!(saved.agents[&copy.key].runtime, copy.definition.runtime);
    }
    assert!(
        agent_config::runtime::get_global_for_tenant(&vw_api_transport::TenantScope::default())
            .await
            .unwrap()
            .agents
            .keys()
            .all(|key| !keys.contains(key))
    );
}

#[tokio::test]
async fn agent_config_cleanup_catalog_copy_is_global_and_tenant_isolated() {
    let _lock = super::global_async_env_lock().lock().await;
    let home = tempfile::tempdir().unwrap();
    let _home = ProfileEnvGuard::set("HOME", home.path());
    let _config = ProfileEnvGuard::set("VIBEWINDOW_CONFIG_DIR", home.path());
    let tenant = vw_api_transport::TenantScope::new("cleanup", "copy").unwrap();
    let other = vw_api_transport::TenantScope::new("cleanup", "other").unwrap();
    agent_config::runtime::update_global_for_tenant(
        &tenant,
        serde_json::json!({"agents":{"main":{"description":"global"}}}),
    )
    .await
    .unwrap();
    let store = agent_config::runtime::SystemConfigStore::new(
        agent_database::GlobalDatabase::for_home(home.path()).unwrap(),
    )
    .unwrap();
    for name in ["a", "b"] {
        let project = home.path().join(name);
        std::fs::create_dir_all(&project).unwrap();
        let payload = serde_json::json!({"agents":{"main":{"description":name}}}).to_string();
        store.upsert_project(&tenant, project.to_str().unwrap(), &payload).unwrap();
        let Json(copy) = gateway_misc::agent_copy::<CopyHost>(
            Extension(tenant.clone()),
            Path("main".into()),
            query_for(&project),
            HeaderMap::new(),
        )
        .await
        .unwrap();
        assert_eq!(copy.definition.description.as_deref(), Some("global"));
        assert!(
            agent_config::runtime::get_global_for_tenant(&tenant)
                .await
                .unwrap()
                .agents
                .contains_key(&copy.key)
        );
        assert!(
            !agent_config::runtime::get_global_for_tenant(&other)
                .await
                .unwrap()
                .agents
                .contains_key(&copy.key)
        );
        assert_eq!(
            store.get_project(&tenant, project.to_str().unwrap()).unwrap().unwrap(),
            payload
        );
    }
}

//! Metadata and catalog contracts using the caller's real host services.
//! These are integration assertions, not simulated admission or standalone unit tests.
use crate::{command_list, agent_list, skill_list, AgentListQuery};
use crate::host::{MiscHost, InstanceQuery};
use axum::{Json, extract::{Extension, Query}, http::HeaderMap};
use vw_api_transport::TenantScope;
struct StoredProjectCleanup(std::path::PathBuf);

impl Drop for StoredProjectCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn query_for(path: &std::path::Path) -> Query<InstanceQuery> {
    Query(InstanceQuery { directory: Some(path.to_string_lossy().to_string()) })
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

pub async fn command_list_returns_named_command_metadata<H: MiscHost>() {
    let temp = tempfile::tempdir().expect("tempdir");
    let Json(commands) =
        command_list::<H>(query_for(temp.path()), HeaderMap::new()).await.expect("commands should list");
    assert!(commands.iter().all(|command| !command.name.trim().is_empty()));
}

pub async fn agent_list_returns_keyed_agent_metadata<H: MiscHost>(tenant: TenantScope) {
    let temp = tempfile::tempdir().expect("tempdir");
    let Json(agents) = agent_list::<H>(
        Extension(tenant),
        Query(AgentListQuery {
            directory: Some(temp.path().to_string_lossy().into()),
            include_disabled: false,
        }),
        HeaderMap::new(),
    )
    .await
    .expect("agents should list");
    assert!(agents.iter().all(|agent| !agent.key.trim().is_empty()));
}

pub async fn skill_list_returns_named_and_located_skill_metadata<H: MiscHost>() {
    let temp = tempfile::tempdir().expect("tempdir");
    let Json(skills) =
        skill_list::<H>(query_for(temp.path()), HeaderMap::new()).await.expect("skills should list");
    assert!(
        skills
            .iter()
            .all(|skill| !skill.name.trim().is_empty() && !skill.location.trim().is_empty())
    );
}

pub async fn list_handlers_ignore_legacy_invalid_project_json<H: MiscHost>(tenant: TenantScope, data_dir: std::path::PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let git_dir = temp.path().join(".git");
    std::fs::create_dir(&git_dir).expect("git dir");
    let project_id = format!("misc-invalid-project-{}", uuid::Uuid::new_v4());
    std::fs::write(git_dir.join("vibewindow"), &project_id).expect("cached project id");
    let storage_path = data_dir
        .join("storage")
        .join("project")
        .join(format!("{project_id}.json"));
    std::fs::create_dir_all(storage_path.parent().expect("project storage parent"))
        .expect("project storage dir");
    std::fs::write(&storage_path, "{").expect("invalid project json");
    let _cleanup = StoredProjectCleanup(storage_path);

    let query = || query_for(temp.path());
    let Json(commands) = command_list::<H>(query(), HeaderMap::new())
        .await
        .expect("command list should ignore legacy project storage");
    let Json(_agents) = agent_list::<H>(
        Extension(tenant),
        Query(AgentListQuery {
            directory: Some(temp.path().to_string_lossy().into()),
            include_disabled: false,
        }),
        HeaderMap::new(),
    )
    .await
    .expect("agent list should ignore legacy project storage");
    let Json(_skills) = skill_list::<H>(query(), HeaderMap::new())
        .await
        .expect("skill list should ignore legacy project storage");

    assert!(!commands.is_empty());
}

pub async fn agent_profile_api_catalog_includes_disabled_only_when_requested<H: MiscHost, F: std::future::Future<Output = bool>>(
    tenant: TenantScope, runtime_disabled: impl FnOnce(String) -> F,
) {
    let home = tempfile::tempdir().unwrap();
    let _home = ProfileEnvGuard::set("HOME", home.path());
    let _config = ProfileEnvGuard::set("VIBEWINDOW_CONFIG_DIR", home.path());
    agent_config::runtime::update_global_for_tenant(&vw_api_transport::TenantScope::default(), serde_json::json!({"agents":{"acp-claude":{"api_key":"profile-secret", "options":{"api_key":"profile-secret"}}}})).await.unwrap();
    let runtime_is_none = runtime_disabled(home.path().to_string_lossy().into_owned()).await;
    assert!(runtime_is_none, "disabled official entries must not be runnable");
    for include_disabled in [false, true] {
        let Json(agents) = agent_list::<H>(
            Extension(tenant.clone()),
            Query(AgentListQuery {
                directory: Some(home.path().to_string_lossy().into()),
                include_disabled,
            }),
            HeaderMap::new(),
        )
        .await
        .unwrap();
        assert_eq!(agents.iter().any(|agent| agent.key == "acp-snow"), include_disabled);
        assert!(agents.iter().all(|agent| agent.definition.api_key.is_none()));
        assert!(!serde_json::to_string(&agents).unwrap().contains("profile-secret"));
    }
}

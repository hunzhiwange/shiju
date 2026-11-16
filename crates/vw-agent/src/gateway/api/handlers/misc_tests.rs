// 覆盖命令、代理和技能列表等只读辅助路由。

use super::*;
use axum::body::Body;
use axum::extract::Query;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use tower::ServiceExt;

struct StoredProjectCleanup(std::path::PathBuf);

impl Drop for StoredProjectCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn query_for(path: &std::path::Path) -> Query<InstanceQuery> {
    Query(InstanceQuery { directory: Some(path.to_string_lossy().to_string()) })
}

async fn route_status(method: Method, uri: &str) -> StatusCode {
    router::<()>()
        .layer(Extension(vw_api_transport::TenantScope::default()))
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("route should respond")
        .status()
}

// 这些只读目录接口固定公开字段，且不依赖遗留项目文件是否可解析。
#[test]
fn router_builds_with_unit_state() {
    let _ = router::<()>();
}

#[tokio::test]
async fn router_dispatches_all_get_routes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let directory = temp.path().to_string_lossy();

    assert_eq!(route_status(Method::GET, "/doc").await, StatusCode::NOT_IMPLEMENTED);
    for path in ["command", "agent", "skill"] {
        let uri = format!("/{path}?directory={directory}");
        assert_eq!(route_status(Method::GET, &uri).await, StatusCode::OK, "{path}");
    }
}

#[tokio::test]
async fn router_rejects_non_get_methods_and_unknown_paths() {
    for path in ["/doc", "/command", "/agent", "/skill"] {
        assert_eq!(
            route_status(Method::POST, path).await,
            StatusCode::METHOD_NOT_ALLOWED,
            "{path}"
        );
    }
    assert_eq!(route_status(Method::GET, "/missing").await, StatusCode::NOT_FOUND);
}



#[tokio::test]
async fn command_list_returns_named_command_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let Json(commands) =
        command_list(query_for(temp.path()), HeaderMap::new()).await.expect("commands should list");
    assert!(commands.iter().all(|command| !command.name.trim().is_empty()));
}

#[tokio::test]
async fn agent_list_returns_keyed_agent_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let Json(agents) = agent_list(
        Extension(crate::session::ui_store::current_tenant_scope()),
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

#[tokio::test]
async fn skill_list_returns_named_and_located_skill_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let Json(skills) =
        skill_list(query_for(temp.path()), HeaderMap::new()).await.expect("skills should list");
    assert!(
        skills
            .iter()
            .all(|skill| !skill.name.trim().is_empty() && !skill.location.trim().is_empty())
    );
}



#[tokio::test]
async fn list_handlers_ignore_legacy_invalid_project_json() {
    let temp = tempfile::tempdir().expect("tempdir");
    let git_dir = temp.path().join(".git");
    std::fs::create_dir(&git_dir).expect("git dir");
    let project_id = format!("misc-invalid-project-{}", uuid::Uuid::new_v4());
    std::fs::write(git_dir.join("vibewindow"), &project_id).expect("cached project id");
    let storage_path = crate::app::agent::global::paths()
        .data
        .join("storage")
        .join("project")
        .join(format!("{project_id}.json"));
    std::fs::create_dir_all(storage_path.parent().expect("project storage parent"))
        .expect("project storage dir");
    std::fs::write(&storage_path, "{").expect("invalid project json");
    let _cleanup = StoredProjectCleanup(storage_path);

    let query = || query_for(temp.path());
    let Json(commands) = command_list(query(), HeaderMap::new())
        .await
        .expect("command list should ignore legacy project storage");
    let Json(_agents) = agent_list(
        Extension(crate::session::ui_store::current_tenant_scope()),
        Query(AgentListQuery {
            directory: Some(temp.path().to_string_lossy().into()),
            include_disabled: false,
        }),
        HeaderMap::new(),
    )
    .await
    .expect("agent list should ignore legacy project storage");
    let Json(_skills) = skill_list(query(), HeaderMap::new())
        .await
        .expect("skill list should ignore legacy project storage");

    assert!(!commands.is_empty());
}

#[tokio::test]
async fn agent_profile_api_catalog_includes_disabled_only_when_requested() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = tempfile::tempdir().unwrap();
    let _home = ProfileEnvGuard::set("HOME", home.path());
    let _config = ProfileEnvGuard::set("VIBEWINDOW_CONFIG_DIR", home.path());
    crate::config::update_global_for_tenant(&vw_api_transport::TenantScope::default(), serde_json::json!({"agents":{"acp-claude":{"api_key":"profile-secret", "options":{"api_key":"profile-secret"}}}})).await.unwrap();
    let runtime = with_instance(home.path().to_string_lossy().into_owned(), || {
        Box::pin(async { Ok(agent::get("acp-snow").await) })
    })
    .await
    .unwrap();
    assert!(runtime.is_none(), "disabled official entries must not be runnable");
    for include_disabled in [false, true] {
        let Json(agents) = agent_list(
            Extension(crate::session::ui_store::current_tenant_scope()),
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

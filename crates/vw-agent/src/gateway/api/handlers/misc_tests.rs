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
async fn not_implemented_returns_501() {
    let error = not_implemented().await.expect_err("endpoint is intentionally unavailable");

    assert_eq!(error.status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(error.message, "not implemented");
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
    let Json(agents) =
        agent_list(query_for(temp.path()), HeaderMap::new()).await.expect("agents should list");
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

#[test]
fn skill_info_serializes_all_public_response_fields() {
    let info = SkillInfo {
        name: "review".to_string(),
        description: "Review a change".to_string(),
        location: "/workspace/review/SKILL.md".to_string(),
        content: "# Review".to_string(),
    };

    assert_eq!(
        serde_json::to_value(info).expect("skill info should serialize"),
        serde_json::json!({
            "name": "review",
            "description": "Review a change",
            "location": "/workspace/review/SKILL.md",
            "content": "# Review",
        })
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
    let Json(_agents) = agent_list(query(), HeaderMap::new())
        .await
        .expect("agent list should ignore legacy project storage");
    let Json(_skills) = skill_list(query(), HeaderMap::new())
        .await
        .expect("skill list should ignore legacy project storage");

    assert!(!commands.is_empty());
}

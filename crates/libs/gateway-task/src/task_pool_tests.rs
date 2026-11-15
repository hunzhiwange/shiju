use super::*;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;

async fn post(path: &str, body: Value) -> (StatusCode, Value) {
    let app: axum::Router<()> = router();
    let response = app
        .oneshot(
            Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn task_pool_capacity_api_client_and_cli_keep_management_operations() {
    let root = tempfile::tempdir().unwrap();
    let id = format!("logical-pool-{}", uuid::Uuid::new_v4());
    let project = project::Info {
        id: id.clone(),
        worktree: root.path().to_string_lossy().into_owned(),
        vcs: None,
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    };
    project::store_local_test_project(&project).unwrap();
    let (status, maintained) =
        post("/task-pool/worktrees/maintain", json!({"project_id":id,"desired_capacity":5})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(maintained["target_idle_count"], 5);
    assert_eq!(maintained["idle_count"], 5);
    let (status, acquired) =
        post("/task-pool/worktrees/acquire", json!({"project_id":id,"task_id":"task"})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(acquired["slot"]["path"], "");
    assert_eq!(acquired["slot"]["branch"], "");
    let slot = acquired["slot"]["id"].as_str().unwrap();
    let (status, _) =
        post("/task-pool/worktrees/slots/recycle", json!({"project_id":id,"slot_id":slot})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    for (route, field) in [("release", "released"), ("acquire", "leased"), ("recycle", "recycled")]
    {
        let (status, result) = post(
            &format!("/task-pool/worktrees/{route}"),
            json!({"project_id":id,"task_id":"task"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(result[field], true);
    }
    let (status, result) =
        post("/task-pool/worktrees/slots/recycle", json!({"project_id":id,"slot_id":slot})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["recycled"], true);
    let (_, snapshot) = post("/task-pool/worktrees/snapshot", json!({"project_id":id})).await;
    assert_eq!(snapshot["idle_count"], 5);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    project::remove_local_test_project(&id).unwrap();
}

#[tokio::test]
async fn task_pool_capacity_missing_project_is_rejected() {
    let (status, _) = post(
        "/task-pool/worktrees/maintain",
        json!({"project_id":"missing-logical-project", "desired_capacity":4}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

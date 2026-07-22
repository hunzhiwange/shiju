use super::*;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;
use tower::ServiceExt;
use uuid::Uuid;
use vw_api_types::task::{
    TaskPoolWorktreeAcquireResponse, TaskPoolWorktreeMaintenanceResponse,
    TaskPoolWorktreeMergeLockResponse, TaskPoolWorktreeMergeUnlockResponse,
    TaskPoolWorktreeRecycleResponse, TaskPoolWorktreeReleaseResponse,
    TaskPoolWorktreeSlotRecycleResponse, TaskPoolWorktreeSlotStateDto,
    TaskPoolWorktreeSnapshotResponse,
};

use crate::app::agent::project;

#[tokio::test]
async fn task_pool_worktree_snapshot_handler_returns_pool() {
    let fixture = GatewayPoolFixture::new().await;

    let response = post_worktree_json(
        "/task-pool/worktrees/snapshot",
        json!({ "project_id": fixture.project_id }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: TaskPoolWorktreeSnapshotResponse = response_body(response).await;
    assert_eq!(body.project_id.as_ref(), fixture.project_id);
    assert_eq!(canonical(&body.repo_root), canonical(&fixture.repo));
    assert_eq!(body.base_branch, "main");
    assert_eq!(body.idle_count, 0);
    assert!(body.pool_root.contains("task-worktrees"));

    fixture.cleanup().await;
}

#[tokio::test]
async fn task_pool_worktree_acquire_handler_prepares_slot() {
    let fixture = GatewayPoolFixture::new().await;
    let task_id = format!("task-{}", Uuid::new_v4());

    let response = post_worktree_json(
        "/task-pool/worktrees/acquire",
        json!({ "project_id": fixture.project_id, "task_id": task_id }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: TaskPoolWorktreeAcquireResponse = response_body(response).await;
    let slot = body.slot.expect("leased slot");
    assert!(body.leased);
    assert_eq!(slot.state, TaskPoolWorktreeSlotStateDto::Busy);
    assert_eq!(slot.leased_task_id.as_ref().expect("leased task").as_ref(), task_id);
    assert_eq!(current_branch(Path::new(&slot.path)), format!("vw/task/{task_id}"));

    remove_worktree(&fixture.repo, Path::new(&slot.path));
    fixture.cleanup().await;
}

#[tokio::test]
async fn task_pool_worktree_maintain_handler_creates_idle_slots() {
    let fixture = GatewayPoolFixture::new().await;

    let response = post_worktree_json(
        "/task-pool/worktrees/maintain",
        json!({ "project_id": fixture.project_id, "desired_capacity": 2 }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: TaskPoolWorktreeMaintenanceResponse = response_body(response).await;
    assert_eq!(body.idle_count, 2);
    assert_eq!(body.target_idle_count, 2);

    let snapshot = body.snapshot.expect("maintenance snapshot");
    assert_eq!(snapshot.slots.len(), 2);
    assert!(snapshot.slots.iter().all(|slot| slot.state == TaskPoolWorktreeSlotStateDto::Idle));
    remove_snapshot_worktrees(&fixture.repo, &snapshot);
    fixture.cleanup().await;
}

#[tokio::test]
async fn task_pool_worktree_release_handler_returns_busy_slot_to_idle() {
    let fixture = GatewayPoolFixture::new().await;
    let task_id = format!("task-release-{}", Uuid::new_v4());
    let acquire = acquire_task_slot(&fixture.project_id, &task_id).await;
    let acquired_slot = acquire.slot.expect("leased slot");

    let response = post_worktree_json(
        "/task-pool/worktrees/release",
        json!({ "project_id": fixture.project_id, "task_id": task_id }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: TaskPoolWorktreeReleaseResponse = response_body(response).await;
    assert!(body.released);

    let snapshot = snapshot_project(&fixture.project_id).await;
    let slot =
        snapshot.slots.iter().find(|slot| slot.id == acquired_slot.id).expect("released slot");
    assert_eq!(slot.state, TaskPoolWorktreeSlotStateDto::Idle);
    assert!(slot.leased_task_id.is_none());

    remove_worktree(&fixture.repo, Path::new(&acquired_slot.path));
    fixture.cleanup().await;
}

#[tokio::test]
async fn task_pool_worktree_recycle_handler_taints_recycled_task_slot_with_reason() {
    let fixture = GatewayPoolFixture::new().await;
    let task_id = format!("task-recycle-{}", Uuid::new_v4());
    let acquire = acquire_task_slot(&fixture.project_id, &task_id).await;
    let acquired_slot = acquire.slot.expect("leased slot");

    let response = post_worktree_json(
        "/task-pool/worktrees/recycle",
        json!({
            "project_id": fixture.project_id,
            "task_id": task_id,
            "taint_reason": "command failed"
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: TaskPoolWorktreeRecycleResponse = response_body(response).await;
    assert!(body.recycled);

    let snapshot = snapshot_project(&fixture.project_id).await;
    let slot =
        snapshot.slots.iter().find(|slot| slot.id == acquired_slot.id).expect("recycled slot");
    assert_eq!(slot.state, TaskPoolWorktreeSlotStateDto::Tainted);
    assert!(slot.leased_task_id.is_none());
    assert_eq!(slot.taint_reason.as_deref(), Some("command failed"));

    remove_worktree(&fixture.repo, Path::new(&acquired_slot.path));
    fixture.cleanup().await;
}

#[tokio::test]
async fn task_pool_worktree_slot_recycle_handler_recycles_idle_slot() {
    let fixture = GatewayPoolFixture::new().await;
    let maintenance = post_worktree_json(
        "/task-pool/worktrees/maintain",
        json!({ "project_id": fixture.project_id, "desired_capacity": 1 }),
    )
    .await;
    assert_eq!(maintenance.status(), StatusCode::OK);
    let maintenance: TaskPoolWorktreeMaintenanceResponse = response_body(maintenance).await;
    let snapshot = maintenance.snapshot.expect("maintenance snapshot");
    let slot = snapshot.slots.first().expect("idle slot");

    let response = post_worktree_json(
        "/task-pool/worktrees/slots/recycle",
        json!({ "project_id": fixture.project_id, "slot_id": slot.id }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: TaskPoolWorktreeSlotRecycleResponse = response_body(response).await;
    assert!(body.recycled);

    let snapshot = snapshot_project(&fixture.project_id).await;
    let recycled =
        snapshot.slots.iter().find(|candidate| candidate.id == slot.id).expect("recycled slot");
    assert_eq!(recycled.state, TaskPoolWorktreeSlotStateDto::Idle);
    assert!(recycled.taint_reason.is_none());

    remove_worktree(&fixture.repo, Path::new(&slot.path));
    fixture.cleanup().await;
}

#[tokio::test]
async fn task_pool_worktree_lock_handler_reports_holder() {
    let fixture = GatewayPoolFixture::new().await;
    let task_a = format!("task-a-{}", Uuid::new_v4());
    let task_b = format!("task-b-{}", Uuid::new_v4());

    let first = post_worktree_json(
        "/task-pool/worktrees/merge-lock",
        json!({ "project_id": fixture.project_id, "task_id": task_a, "target_branch": "main" }),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);

    let second = post_worktree_json(
        "/task-pool/worktrees/merge-lock",
        json!({ "project_id": fixture.project_id, "task_id": task_b, "target_branch": "main" }),
    )
    .await;

    assert_eq!(second.status(), StatusCode::OK);
    let body: TaskPoolWorktreeMergeLockResponse = response_body(second).await;
    assert!(!body.locked);
    assert_eq!(body.holder_task_id.as_ref().expect("holder").as_ref(), task_a);

    fixture.cleanup().await;
}

#[tokio::test]
async fn task_pool_worktree_merge_unlock_handler_reports_released_for_holder_only() {
    let fixture = GatewayPoolFixture::new().await;
    let task_a = format!("task-a-{}", Uuid::new_v4());
    let task_b = format!("task-b-{}", Uuid::new_v4());

    let first_lock = post_worktree_json(
        "/task-pool/worktrees/merge-lock",
        json!({ "project_id": fixture.project_id, "task_id": task_a, "target_branch": " main " }),
    )
    .await;
    assert_eq!(first_lock.status(), StatusCode::OK);

    let non_holder_unlock = post_worktree_json(
        "/task-pool/worktrees/merge-unlock",
        json!({ "project_id": fixture.project_id, "task_id": task_b, "target_branch": "main" }),
    )
    .await;
    assert_eq!(non_holder_unlock.status(), StatusCode::OK);
    let body: TaskPoolWorktreeMergeUnlockResponse = response_body(non_holder_unlock).await;
    assert!(!body.released);

    let holder_unlock = post_worktree_json(
        "/task-pool/worktrees/merge-unlock",
        json!({ "project_id": fixture.project_id, "task_id": task_a, "target_branch": "main" }),
    )
    .await;
    assert_eq!(holder_unlock.status(), StatusCode::OK);
    let body: TaskPoolWorktreeMergeUnlockResponse = response_body(holder_unlock).await;
    assert!(body.released);

    fixture.cleanup().await;
}

#[tokio::test]
async fn task_pool_worktree_handlers_reject_missing_project() {
    let response = post_worktree_json(
        "/task-pool/worktrees/snapshot",
        json!({ "project_id": format!("missing-{}", Uuid::new_v4()) }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body: serde_json::Value = response_body(response).await;
    assert_eq!(body["error"], "project not found");
}

#[tokio::test]
async fn task_pool_mutation_handlers_reject_missing_project() {
    let project_id = format!("missing-{}", Uuid::new_v4());
    let requests = [
        (
            "/task-pool/worktrees/maintain",
            json!({ "project_id": project_id, "desired_capacity": 2 }),
        ),
        ("/task-pool/worktrees/acquire", json!({ "project_id": project_id, "task_id": "task-1" })),
        ("/task-pool/worktrees/release", json!({ "project_id": project_id, "task_id": "task-1" })),
        (
            "/task-pool/worktrees/recycle",
            json!({ "project_id": project_id, "task_id": "task-1", "taint_reason": "failed" }),
        ),
        (
            "/task-pool/worktrees/slots/recycle",
            json!({ "project_id": project_id, "slot_id": "slot-1" }),
        ),
        (
            "/task-pool/worktrees/merge-lock",
            json!({ "project_id": project_id, "task_id": "task-1", "target_branch": "main" }),
        ),
        (
            "/task-pool/worktrees/merge-unlock",
            json!({ "project_id": project_id, "task_id": "task-1", "target_branch": "main" }),
        ),
    ];

    for (path, body) in requests {
        let response = post_worktree_json(path, body).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "path: {path}");
        let body: serde_json::Value = response_body(response).await;
        assert_eq!(body["error"], "project not found", "path: {path}");
    }
}

#[test]
fn task_pool_error_mapping_distinguishes_missing_slots() {
    let missing =
        map_task_worktree_pool_error(anyhow::anyhow!("task worktree slot missing: slot-1"));
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert_eq!(missing.message, "task worktree slot missing: slot-1");

    let invalid = map_task_worktree_pool_error(anyhow::anyhow!("target branch is empty"));
    assert_eq!(invalid.status, StatusCode::BAD_REQUEST);
    assert_eq!(invalid.message, "target branch is empty");
}

async fn post_worktree_json(path: &str, body: serde_json::Value) -> axum::response::Response {
    let app: axum::Router<()> = router();
    app.oneshot(
        Request::builder()
            .method(Method::POST)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request should build"),
    )
    .await
    .expect("route should respond")
}

async fn acquire_task_slot(project_id: &str, task_id: &str) -> TaskPoolWorktreeAcquireResponse {
    let response = post_worktree_json(
        "/task-pool/worktrees/acquire",
        json!({ "project_id": project_id, "task_id": task_id }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response_body(response).await
}

async fn snapshot_project(project_id: &str) -> TaskPoolWorktreeSnapshotResponse {
    let response =
        post_worktree_json("/task-pool/worktrees/snapshot", json!({ "project_id": project_id }))
            .await;
    assert_eq!(response.status(), StatusCode::OK);
    response_body(response).await
}

async fn response_body<T>(response: axum::response::Response) -> T
where
    T: serde::de::DeserializeOwned,
{
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    serde_json::from_slice(&bytes).expect("body should be json")
}

struct GatewayPoolFixture {
    _temp: tempfile::TempDir,
    project_id: String,
    repo: PathBuf,
}

impl GatewayPoolFixture {
    async fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let repo = temp.path().join("repo");
        init_repo(&repo);
        let project_id = format!("handler-task-pool-worktree-{}", Uuid::new_v4());
        let info = project::Info {
            id: project_id.clone(),
            worktree: repo.to_string_lossy().to_string(),
            vcs: Some(project::Vcs::Git),
            name: None,
            icon: None,
            commands: None,
            time: project::TimeInfo { created: 0, updated: 0, initialized: None },
            sandboxes: Vec::new(),
        };
        project::store_local_test_project(&info).expect("project write");
        Self { _temp: temp, project_id, repo }
    }

    async fn cleanup(&self) {
        project::remove_local_test_project(&self.project_id).expect("project cleanup");
    }
}

fn init_repo(path: &Path) {
    run(Command::new("git").arg("init").arg("-b").arg("main").arg(path));
    run(Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("config")
        .arg("user.email")
        .arg("test@example.com"));
    run(Command::new("git").arg("-C").arg(path).arg("config").arg("user.name").arg("Test User"));
    std::fs::write(path.join("README.md"), "demo\n").expect("write readme");
    run(Command::new("git").arg("-C").arg(path).arg("add").arg("README.md"));
    run(Command::new("git").arg("-C").arg(path).arg("commit").arg("-m").arg("init"));
}

fn current_branch(path: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output()
        .expect("git branch");
    assert!(
        output.status.success(),
        "command failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn remove_worktree(repo: &Path, path: &Path) {
    run(Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("worktree")
        .arg("remove")
        .arg("--force")
        .arg(path));
}

fn remove_snapshot_worktrees(repo: &Path, snapshot: &TaskPoolWorktreeSnapshotResponse) {
    for slot in &snapshot.slots {
        remove_worktree(repo, Path::new(&slot.path));
    }
}

fn canonical(path: impl AsRef<Path>) -> PathBuf {
    std::fs::canonicalize(path).expect("canonical path")
}

fn run(command: &mut Command) {
    let output = command.output().expect("run command");
    assert!(
        output.status.success(),
        "command failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

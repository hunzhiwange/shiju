use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use std::{path::PathBuf, process::Command};
use agent_project as project;
use task_scheduler::{domain::{TaskId, RunId}, runner::local::{LocalAgentExecutor, LocalAgentWorkspaceRequest}, store::AgentSchedulerStorageContext};
use crate::agent_scheduler::runner::gateway_executor::GatewayLocalAgentExecutor;
#[tokio::test]
async fn agent_scheduler_routes_require_header_tenant_scope() {
    let app = router::<()>()
        .layer(axum::middleware::from_fn(
            crate::app::agent::gateway::middleware::tenant_scope_middleware,
        ))
        .with_state(());
    let old = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/agent-scheduler/platforms/platform-a/companies/company-a/projects/project-a/tasks")
                .header(vw_api_transport::PLATFORM_ID_HEADER, "platform-a")
                .header(vw_api_transport::COMPANY_ID_HEADER, "company-a")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(old.status(), StatusCode::NOT_FOUND);

    let missing_tenant = app
        .oneshot(
            Request::builder()
                .uri("/agent-scheduler/projects/project-a/tasks")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(missing_tenant.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn task_pool_capacity_gateway_runtime_projects_logical_station() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-local-worktree");
    init_merge_test_repo(&project_path, "feature/local-worktree-base");
    let (project_id, project_info) = register_merge_test_project(&project_path).await;
    let task_id = TaskId::new("task-local-worktree").expect("task id");
    let request = LocalAgentWorkspaceRequest {
        agent: None,
        project_path: project_path.clone(),
        task_id: task_id.clone(),
        run_id: RunId::new("run-local-worktree").expect("run id"),
    };

    let workspace = tokio::task::spawn_blocking(move || {
        GatewayLocalAgentExecutor::default().lease_workspace(request)
    })
    .await
    .expect("lease join")
    .expect("lease workspace");

    assert_eq!(PathBuf::from(&workspace.path), project_path.canonicalize().unwrap());
    assert_eq!(workspace.branch, None);
    assert_eq!(workspace.slot_id, None);
    let pool = crate::app::agent::gateway::api::handlers::task_pool::task_worktree_pool_service();
    let snapshot = pool.snapshot(&project_info).await.expect("logical snapshot");
    assert_eq!(snapshot.busy_count, 1);
    assert_eq!(snapshot.slots[0].run_id.as_deref(), Some("run-local-worktree"));
    assert!(snapshot.slots[0].path.is_empty());
    assert!(snapshot.slots[0].branch.is_empty());
    let release_path = project_path.clone();
    tokio::task::spawn_blocking(move || {
        GatewayLocalAgentExecutor::default().release_workspace(
            crate::agent_scheduler::runner::local::LocalAgentWorkspaceReleaseRequest {
                project_path: release_path,
                task_id,
                run_id: RunId::new("run-local-worktree").unwrap(),
                workspace,
            },
        )
    })
    .await
    .expect("release join")
    .expect("release logical station");
    assert_eq!(pool.snapshot(&project_info).await.unwrap().busy_count, 0);
    cleanup_merge_test_project(&project_id).await;
}

fn scheduler_project_id(project_path: &std::path::Path) -> String {
    let project_id = project_path
        .storage_scope()
        .expect("scheduler test scope")
        .project_scope()
        .project_id()
        .as_ref()
        .to_owned();
    project::store_local_test_project(&project::Info {
        id: project_id.clone(),
        worktree: project_path.to_string_lossy().into_owned(),
        vcs: None,
        name: Some("Agent Scheduler test project".to_owned()),
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    })
    .expect("register scheduler test project");
    project_id
}

fn init_merge_test_repo(project_path: &std::path::Path, source_branch: &str) {
    std::fs::create_dir_all(project_path).expect("project dir");
    run_test_git(project_path, &["init"]);
    run_test_git(project_path, &["config", "user.email", "test@example.com"]);
    run_test_git(project_path, &["config", "user.name", "Test User"]);
    run_test_git(project_path, &["checkout", "-b", "main"]);
    std::fs::write(project_path.join("base.txt"), "base\n").expect("base file");
    run_test_git(project_path, &["add", "base.txt"]);
    run_test_git(project_path, &["commit", "-m", "initial"]);
    run_test_git(project_path, &["checkout", "-b", source_branch]);
    std::fs::write(project_path.join("feature.txt"), "feature\n").expect("feature file");
    run_test_git(project_path, &["add", "feature.txt"]);
    run_test_git(project_path, &["commit", "-m", "feature change"]);
    run_test_git(project_path, &["checkout", "main"]);
}

async fn register_merge_test_project(project_path: &std::path::Path) -> (String, project::Info) {
    let project_id = scheduler_project_id(project_path);
    let info = project::Info {
        id: project_id.clone(),
        worktree: project_path.to_string_lossy().to_string(),
        vcs: Some(project::Vcs::Git),
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    };
    project::store_local_test_project(&info).expect("project write");
    (project_id, info)
}

async fn cleanup_merge_test_project(project_id: &str) {
    project::remove_local_test_project(project_id).expect("project cleanup");
}

fn run_test_git(repo: &std::path::Path, args: &[&str]) {
    let output = Command::new("git").current_dir(repo).args(args).output().expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout={}\nstderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

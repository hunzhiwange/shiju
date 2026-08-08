//! `task_plan_run` 网关 handler 校验与路由接线测试。

use super::*;
use axum::Json;
use axum::http::StatusCode;
use vw_api_types::id::TenantScope;
use vw_api_types::task::{TaskPlanRunRequest, TaskPlanRunStatus, TaskPlanRunStreamEventDto};

fn sample_request() -> TaskPlanRunRequest {
    TaskPlanRunRequest {
        query: "build a plan".to_string(),
        root: "/tmp/workspace".to_string(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
    }
}

fn default_tenant() -> Extension<TenantScope> {
    Extension(TenantScope::default())
}

#[tokio::test]
async fn task_plan_run_rejects_empty_root() {
    let err = task_plan_run_stream(
        default_tenant(),
        HeaderMap::new(),
        Json(TaskPlanRunRequest { root: "   ".to_string(), ..sample_request() }),
    )
    .await
    .expect_err("empty root must be rejected before SSE");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.to_string().contains("root"), "message={}", err);
}

#[tokio::test]
async fn task_plan_run_rejects_empty_query() {
    let err = task_plan_run_stream(
        default_tenant(),
        HeaderMap::new(),
        Json(TaskPlanRunRequest { query: " ".to_string(), ..sample_request() }),
    )
    .await
    .expect_err("empty query must be rejected before SSE");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.to_string().contains("query"), "message={}", err);
}

#[test]
fn task_plan_run_router_is_wired() {
    let _router = router::<()>();
}

#[test]
fn task_plan_run_stream_path_matches_desktop_client() {
    assert_eq!(TASK_PLAN_RUN_STREAM_ROUTE, "/task-plan-run/stream");
    assert_eq!(format!("/v1{TASK_PLAN_RUN_STREAM_ROUTE}"), "/v1/task-plan-run/stream");
    assert_eq!(
        TASK_PLAN_RUN_STATUS_ROUTE,
        "/task-plan-run/{task_id}/status"
    );
    assert_eq!(TASK_PLAN_RUN_STOP_ROUTE, "/task-plan-run/{task_id}/stop");
}

#[tokio::test]
async fn task_plan_run_control_routes_are_tenant_scoped_and_stop_is_idempotent() {
    let owner = TenantScope::new("platform-route-owner", "company-route-owner").expect("owner");
    let outsider =
        TenantScope::new("platform-route-outsider", "company-route-outsider").expect("outsider");
    let task_id = format!("route-control-{}", uuid::Uuid::new_v4());
    let session_id = format!("{task_id}-plan");
    let registry = global_plan_run_control_registry();
    let cancellation =
        registry.register(owner.clone(), &task_id).expect("register route control test");
    registry
        .observe_event(
            &owner,
            &task_id,
            &TaskPlanRunStreamEventDto::PlanningStarted {
                task_id: task_id.clone(),
                plan_dir: "/repo/.plans/control".to_string(),
                session_id: session_id.clone(),
            },
        )
        .expect("observe planning");

    let Json(status) = task_plan_run_status(
        Extension(owner.clone()),
        Path(task_id.clone()),
    )
    .await
    .expect("owner status");
    assert_eq!(status.status, TaskPlanRunStatus::Running);
    assert_eq!(status.current_session_id.as_deref(), Some(session_id.as_str()));

    let cross_tenant_status =
        task_plan_run_status(Extension(outsider.clone()), Path(task_id.clone()))
            .await
            .expect_err("cross-tenant status must be hidden");
    assert_eq!(cross_tenant_status.status, StatusCode::NOT_FOUND);
    let cross_tenant_stop = task_plan_run_stop(Extension(outsider), Path(task_id.clone()))
        .await
        .expect_err("cross-tenant stop must be hidden");
    assert_eq!(cross_tenant_stop.status, StatusCode::NOT_FOUND);
    assert!(!cancellation.is_cancelled());

    let Json(stopped) =
        task_plan_run_stop(Extension(owner.clone()), Path(task_id.clone()))
            .await
            .expect("owner stop");
    assert_eq!(stopped.status, TaskPlanRunStatus::Cancelled);
    assert!(cancellation.is_cancelled());

    let Json(stopped_again) = task_plan_run_stop(Extension(owner), Path(task_id))
        .await
        .expect("idempotent stop");
    assert_eq!(stopped_again.status, TaskPlanRunStatus::Cancelled);
}

#[test]
fn panic_message_reads_str_and_string_payloads() {
    assert_eq!(panic_message(&"boom"), "boom");
    assert_eq!(panic_message(&"owned".to_string()), "owned");
    assert_eq!(panic_message(&42u32), "unknown panic");
}

#[tokio::test]
async fn task_plan_run_rejects_reused_request_id_with_different_body() {
    let request_id = format!("plan-conflict-{}", uuid::Uuid::new_v4());
    let registered_request =
        TaskPlanRunRequest { query: "first request".to_string(), ..sample_request() };
    let signature = request_signature(
        &serde_json::to_vec(&registered_request).expect("serialize registered request"),
    );
    global_stream_store()
        .open(TenantScope::default(), StreamKind::TaskPlanRun, request_id.clone(), signature, 0)
        .expect("register first request");
    let mut headers = HeaderMap::new();
    headers.insert(
        crate::app::agent::gateway::api::handlers::resumable_stream::STREAM_REQUEST_ID_HEADER,
        request_id.parse().expect("valid request id header"),
    );

    let error = task_plan_run_stream(default_tenant(), headers, Json(sample_request()))
        .await
        .expect_err("different request body must conflict");

    assert_eq!(error.status, StatusCode::CONFLICT);
}

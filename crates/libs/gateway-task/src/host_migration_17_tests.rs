//! Input rejection must precede all execution host operations.
use crate::task_plan_run::*;
use crate::PlanRunHost;
use axum::{Extension, Json, http::{HeaderMap, StatusCode}};
use gateway_resumable_stream::{global_stream_store, request_signature, StreamKind};
use vw_api_transport::TenantScope;
use vw_api_task::{TaskPlanRunRequest, TaskPlanRunStatusResponse, TaskPlanRunStreamEventDto};
use tokio_util::sync::CancellationToken;
struct Host;
impl PlanRunHost for Host {
 fn status(_: &TenantScope, _: &str) -> Option<TaskPlanRunStatusResponse> { panic!("unexpected host access") }
 fn stop(_: &TenantScope, _: &str) -> Result<Option<TaskPlanRunStatusResponse>, String> { panic!("unexpected host access") }
 fn register(_: TenantScope, _: &str) -> Result<CancellationToken, String> { panic!("unexpected host access") }
 fn observe_event(_: &TenantScope, _: &str, _: &TaskPlanRunStreamEventDto) -> Result<bool, String> { panic!("unexpected host access") }
 fn run<'a>(_: TenantScope, _: String, _: TaskPlanRunRequest, _: CancellationToken,
  _: Box<dyn FnMut(TaskPlanRunStreamEventDto) -> bool + Send + 'a>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> { panic!("unexpected host access") }
}
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
    let err = task_plan_run_stream::<Host>(
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
    let err = task_plan_run_stream::<Host>(
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
fn task_plan_run_stream_path_matches_desktop_client() {
    assert_eq!(TASK_PLAN_RUN_STREAM_ROUTE, "/task-plan-run/stream");
    assert_eq!(format!("/v1{TASK_PLAN_RUN_STREAM_ROUTE}"), "/v1/task-plan-run/stream");
    assert_eq!(TASK_PLAN_RUN_STATUS_ROUTE, "/task-plan-run/{task_id}/status");
    assert_eq!(TASK_PLAN_RUN_STOP_ROUTE, "/task-plan-run/{task_id}/stop");
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
        gateway_resumable_stream::STREAM_REQUEST_ID_HEADER,
        request_id.parse().expect("valid request id header"),
    );

    let error = task_plan_run_stream::<Host>(default_tenant(), headers, Json(sample_request()))
        .await
        .expect_err("different request body must conflict");

    assert_eq!(error.status, StatusCode::CONFLICT);
}

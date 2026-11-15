//! 聊天工作流执行（task-plan-run）网关 SSE 路由。

use std::panic::AssertUnwindSafe;

use axum::{
    Extension, Json, Router,
    extract::Path,
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
};
use futures_util::FutureExt;
use vw_api_transport::id::TenantScope;
use vw_api_task::{
    TaskPlanRunRequest, TaskPlanRunStatusResponse, TaskPlanRunStreamEventDto,
};

use gateway_types::ApiError;
use gateway_resumable_stream::{
    StreamKind, StreamOpenError, global_stream_store, last_event_id, request_signature,
    sse_response, stream_request_id,
};
use crate::PlanRunHost;

/// 相对 `/v1` 的路由路径，与桌面客户端 `/v1/task-plan-run/stream` 对齐。
pub const TASK_PLAN_RUN_STREAM_ROUTE: &str = "/task-plan-run/stream";
pub const TASK_PLAN_RUN_STREAM_STOP_ROUTE: &str = "/task-plan-run/stream/{request_id}/stop";
pub const TASK_PLAN_RUN_STATUS_ROUTE: &str = "/task-plan-run/{task_id}/status";
pub const TASK_PLAN_RUN_STOP_ROUTE: &str = "/task-plan-run/{task_id}/stop";

pub fn router<S, H: PlanRunHost>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(TASK_PLAN_RUN_STREAM_ROUTE, post(task_plan_run_stream::<H>))
        .route(TASK_PLAN_RUN_STREAM_STOP_ROUTE, post(task_plan_run_stream_stop))
        .route(TASK_PLAN_RUN_STATUS_ROUTE, get(task_plan_run_status::<H>))
        .route(TASK_PLAN_RUN_STOP_ROUTE, post(task_plan_run_stop::<H>))
}

async fn task_plan_run_stream_stop(
    Extension(tenant): Extension<TenantScope>,
    Path(request_id): Path<String>,
) -> Json<serde_json::Value> {
    let cancelled = global_stream_store().cancel(tenant, StreamKind::TaskPlanRun, &request_id);
    Json(serde_json::json!({ "cancelled": cancelled }))
}

pub async fn task_plan_run_status<H: PlanRunHost>(
    Extension(tenant): Extension<TenantScope>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskPlanRunStatusResponse>, ApiError> {
    H::status(&tenant, &task_id)
        .map(Json)
        .ok_or_else(|| ApiError::not_found("task-plan-run 不存在"))
}

pub async fn task_plan_run_stop<H: PlanRunHost>(
    Extension(tenant): Extension<TenantScope>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskPlanRunStatusResponse>, ApiError> {
    H::stop(&tenant, &task_id)
        .map_err(ApiError::internal)?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("task-plan-run 不存在"))
}

pub async fn task_plan_run_stream<H: PlanRunHost>(
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(request): Json<TaskPlanRunRequest>,
) -> Result<Response, ApiError> {
    validate_task_plan_run_request(&request)?;
    task_plan_run_streaming_response::<H>(tenant, headers, request)
}

fn validate_task_plan_run_request(request: &TaskPlanRunRequest) -> Result<(), ApiError> {
    if request.root.trim().is_empty() {
        return Err(ApiError::bad_request("task-plan-run 需要非空 root"));
    }
    if request.query.trim().is_empty() {
        return Err(ApiError::bad_request("task-plan-run 需要非空 query"));
    }
    Ok(())
}

fn task_plan_run_streaming_response<H: PlanRunHost>(
    tenant: TenantScope,
    headers: HeaderMap,
    request: TaskPlanRunRequest,
) -> Result<Response, ApiError> {
    let request_id = stream_request_id(&headers)?;
    let after_event_id = last_event_id(&headers)?;
    let serialized_request = serde_json::to_vec(&request)
        .map_err(|error| ApiError::internal(format!("task-plan-run 请求签名失败: {error}")))?;
    let signature = request_signature(&serialized_request);
    let mut opened = global_stream_store()
        .open(tenant.clone(), StreamKind::TaskPlanRun, request_id, signature, after_event_id)
        .map_err(|error| match error {
            StreamOpenError::RequestConflict => ApiError {
                status: StatusCode::CONFLICT,
                message: "相同流请求标识对应了不同 task-plan-run 请求".to_string(),
            },
            StreamOpenError::ResumeUnavailable => ApiError {
                status: StatusCode::GONE,
                message: "task-plan-run replay is unavailable".to_string(),
            },
        })?;
    let task_id = uuid::Uuid::new_v4().to_string();
    if let Some(producer) = opened.producer.take() {
        let cancellation =
            H::register(tenant.clone(), &task_id).map_err(ApiError::internal)?;
        let cancellation_tenant = tenant.clone();
        let cancellation_task_id = task_id.clone();
        producer.install_cancellation(move || {
            if let Err(error) = H::stop(
                &cancellation_tenant,
                &cancellation_task_id,
            ) {
                tracing::warn!(
                    target: "vw_agent::task_plan_run",
                    task_id = %cancellation_task_id,
                    error = %error,
                    "按流请求停止 plan-run 失败"
                );
            }
        });
        tokio::spawn(async move {
            // H::run 必须显式恢复租户作用域，不能继承请求中间件的任务局部状态。
            let run = AssertUnwindSafe(H::run(
                tenant.clone(),
                task_id.clone(),
                request,
                cancellation,
                Box::new(|event| {
                    if let Err(error) =
                        H::observe_event(&tenant, &task_id, &event)
                    {
                        tracing::warn!(
                            target: "vw_agent::task_plan_run",
                            task_id = %task_id,
                            error = %error,
                            "更新 plan-run 控制状态失败"
                        );
                    }
                    let terminal = matches!(
                        event,
                        TaskPlanRunStreamEventDto::Finished { .. }
                            | TaskPlanRunStreamEventDto::Error { .. }
                            | TaskPlanRunStreamEventDto::Blocked { .. }
                            | TaskPlanRunStreamEventDto::Cancelled { .. }
                    );
                    let data = serde_json::to_string(&event)
                        .expect("TaskPlanRunStreamEventDto must serialize");
                    producer.publish(data, terminal)
                }),
            ));
            if let Err(panic) = run.catch_unwind().await {
                let message =
                    format!("task-plan-run 内部异常: {}", panic_message(panic.as_ref()));
                let event = TaskPlanRunStreamEventDto::Error {
                    task_id: Some(task_id.clone()),
                    message,
                };
                let _ = H::observe_event(&tenant, &task_id, &event);
                let data = serde_json::to_string(&event)
                    .expect("TaskPlanRunStreamEventDto must serialize");
                let _ = producer.publish(data, true);
            }

            if !producer.is_terminal() {
                let event = TaskPlanRunStreamEventDto::Error {
                    task_id: Some(task_id.clone()),
                    message: "task-plan-run 异常结束：未发出终态事件".to_string(),
                };
                let _ = H::observe_event(&tenant, &task_id, &event);
                let data = serde_json::to_string(&event)
                    .expect("TaskPlanRunStreamEventDto must serialize");
                let _ = producer.publish(data, true);
            }
        });
    }

    Ok(sse_response(opened.subscription))
}

pub fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic".to_string()
    }
}

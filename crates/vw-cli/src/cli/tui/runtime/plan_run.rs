//! TUI 计划/目标长任务的 Gateway 宿主与所有权校验。

use std::path::Path;

use vw_gateway_client::vw_api_types::task::TaskPlanRunRequest;
use vw_gateway_client::{GatewayClient, GatewayStreamAttemptResult, GatewayStreamResume};

use super::gateway::{GatewayUiRuntime, block_on_gateway};
use super::stream_adapter::{UiRuntimeEvent, adapt_task_plan_run_event};

/// 构造真实 task-plan-run 请求；计划与持续目标共享 Gateway 的目标运行语义。
pub(crate) fn plan_run_request(
    query: &str,
    root: &Path,
    model: Option<String>,
    agent: Option<String>,
    acp_agent: Option<String>,
    parent_session_id: Option<String>,
) -> Result<TaskPlanRunRequest, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("plan/goal run requires a non-empty request".to_string());
    }
    let root =
        root.to_str().ok_or_else(|| "plan/goal workspace root is not valid UTF-8".to_string())?;
    Ok(TaskPlanRunRequest {
        query: query.to_string(),
        root: root.to_string(),
        model,
        agent,
        acp_agent,
        parent_session_id,
    })
}

/// 在后台线程中运行一次带稳定 request ID 的 plan-run SSE，并适配到统一事件管线。
pub(crate) fn stream_plan_run_resumable_blocking(
    runtime: &GatewayUiRuntime,
    request: &TaskPlanRunRequest,
    request_id: &str,
    mut on_event: impl FnMut(UiRuntimeEvent) -> bool,
) -> Result<GatewayStreamAttemptResult, String> {
    let resume = GatewayStreamResume { request_id: request_id.to_string(), last_event_id: None };
    let mut consumer_stopped = false;
    block_on_gateway(GatewayClient::stream_task_plan_run_resumable(
        runtime.endpoint(),
        request,
        &resume,
        |event| {
            for event in adapt_task_plan_run_event(event.event) {
                if !on_event(event) {
                    consumer_stopped = true;
                    return false;
                }
            }
            true
        },
    ))
    .map(
        |result| {
            if consumer_stopped { GatewayStreamAttemptResult::CallerStopped } else { result }
        },
    )
}

/// 校验停止目标同时匹配当前 TUI 保存的 request ID 与 task ID。
pub(crate) fn validate_owned_plan_run(
    owned_request_id: Option<&str>,
    owned_task_id: Option<&str>,
    request_id: &str,
    task_id: &str,
) -> Result<(), String> {
    if request_id.trim().is_empty() || task_id.trim().is_empty() {
        return Err("plan-run stop requires non-empty request and task IDs".to_string());
    }
    if owned_request_id != Some(request_id) || owned_task_id != Some(task_id) {
        return Err("plan-run stop target is not owned by the active TUI run".to_string());
    }
    Ok(())
}

/// 精确停止当前 TUI 拥有的流请求与其后台任务。
pub(crate) fn stop_owned_plan_run_blocking(
    runtime: &GatewayUiRuntime,
    owned_request_id: Option<&str>,
    owned_task_id: Option<&str>,
    request_id: &str,
    task_id: &str,
) -> Result<bool, String> {
    validate_owned_plan_run(owned_request_id, owned_task_id, request_id, task_id)?;
    let stream_cancelled =
        block_on_gateway(runtime.client().task_plan_run_stream_stop(request_id))?;
    let status = block_on_gateway(runtime.client().task_plan_run_stop(task_id))?;
    Ok(stream_cancelled || status.status.is_terminal())
}

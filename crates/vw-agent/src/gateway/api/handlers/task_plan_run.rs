//! 任务计划路由组合与现有会话执行器适配。
use gateway_task::PlanRunHost;
use crate::task_plan_run::{global_plan_run_control_registry, GatewaySessionRunner, run_task_plan_with_cancellation};
use crate::session::child_agent::coordinator::global_child_agent_coordinator;
use tokio_util::sync::CancellationToken;
use vw_api_transport::TenantScope;
use vw_api_task::{TaskPlanRunRequest, TaskPlanRunStatusResponse, TaskPlanRunStreamEventDto};
use std::{future::Future, pin::Pin};

struct Host;
impl PlanRunHost for Host {
    fn status(tenant: &TenantScope, task_id: &str) -> Option<TaskPlanRunStatusResponse> {
        global_plan_run_control_registry().status(tenant, task_id)
    }
    fn stop(tenant: &TenantScope, task_id: &str) -> Result<Option<TaskPlanRunStatusResponse>, String> {
        let outcome = global_plan_run_control_registry().stop(tenant, task_id, &global_child_agent_coordinator())?;
        if let Some(outcome) = &outcome {
            tracing::debug!(target: "vw_agent::task_plan_run", task_id = %task_id,
                cancelled_descendants = outcome.cancelled_descendants,
                already_terminal = outcome.already_terminal, "处理 plan-run 停止请求");
        }
        Ok(outcome.map(|outcome| outcome.status))
    }
    fn register(tenant: TenantScope, task_id: &str) -> Result<CancellationToken, String> {
        global_plan_run_control_registry().register(tenant, task_id)
    }
    fn observe_event(tenant: &TenantScope, task_id: &str, event: &TaskPlanRunStreamEventDto) -> Result<bool, String> {
        global_plan_run_control_registry().observe_event(tenant, task_id, event)
    }
    fn run<'a>(tenant: TenantScope, task_id: String, request: TaskPlanRunRequest,
        cancellation: CancellationToken,
        on_event: Box<dyn FnMut(TaskPlanRunStreamEventDto) -> bool + Send + 'a>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(crate::session::ui_store::with_tenant_scope(tenant.clone(), async move {
            let runner = GatewaySessionRunner::new(tenant, request.parent_session_id.clone());
            run_task_plan_with_cancellation(&task_id, &request, &runner, cancellation, on_event).await
        }))
    }
}

pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> axum::Router<S> {
    gateway_task::task_plan_run::router::<S, Host>()
}

#[cfg(test)]
#[path = "task_plan_run_tests.rs"]
mod task_plan_run_tests;

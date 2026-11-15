//! 任务管理 HTTP 路由；会话执行由宿主注入。
#![cfg(not(target_arch = "wasm32"))]

pub mod task_lifecycle;
pub mod task_pool;
pub mod task_plan_run;

use std::{future::Future, pin::Pin};
use tokio_util::sync::CancellationToken;
use vw_api_transport::TenantScope;
use vw_api_task::{TaskPlanRunRequest, TaskPlanRunStatusResponse, TaskPlanRunStreamEventDto};

/// 宿主计划执行与原控制注册表的窄接口；实现必须保留显式租户作用域。
pub trait PlanRunHost: Send + Sync + 'static {
    fn status(tenant: &TenantScope, task_id: &str) -> Option<TaskPlanRunStatusResponse>;
    fn stop(tenant: &TenantScope, task_id: &str) -> Result<Option<TaskPlanRunStatusResponse>, String>;
    fn register(tenant: TenantScope, task_id: &str) -> Result<CancellationToken, String>;
    fn observe_event(tenant: &TenantScope, task_id: &str, event: &TaskPlanRunStreamEventDto) -> Result<bool, String>;
    fn run<'a>(
        tenant: TenantScope,
        task_id: String,
        request: TaskPlanRunRequest,
        cancellation: CancellationToken,
        on_event: Box<dyn FnMut(TaskPlanRunStreamEventDto) -> bool + Send + 'a>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}

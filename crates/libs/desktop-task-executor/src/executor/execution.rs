//! 任务线程与工作树租约的执行生命周期。
use super::{Sender, TaskLogStream};
use desktop_task_model::Task;
/// 公开的 execute_task_async 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub async fn execute_task_async(
    endpoint: fn() -> vw_gateway_client::GatewayEndpoint,
    gateway_client: fn() -> Result<vw_gateway_client::GatewayClient, String>,
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let task_id = task.id.clone();
        tokio::task::spawn_blocking(move || {
            execute_task_in_workspace(
                endpoint,
                gateway_client,
                task,
                project_path,
                log_sender,
                GatewayPromptSessionMode::Fresh,
            )
        })
        .await
        .unwrap_or_else(|error| (task_id, Err(format!("任务执行线程异常: {}", error))))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (project_path, log_sender);
        (task.id, Err("Task execution not supported on Web".to_string()))
    }
}

/// 公开的 execute_task_retry_async 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub async fn execute_task_retry_async(
    endpoint: fn() -> vw_gateway_client::GatewayEndpoint,
    gateway_client: fn() -> Result<vw_gateway_client::GatewayClient, String>,
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let task_id = task.id.clone();
        tokio::task::spawn_blocking(move || {
            execute_task_in_workspace(
                endpoint,
                gateway_client,
                task,
                project_path,
                log_sender,
                GatewayPromptSessionMode::ContinueExisting,
            )
        })
        .await
        .unwrap_or_else(|error| (task_id, Err(format!("任务执行线程异常: {}", error))))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (project_path, log_sender);
        (task.id, Err("Task execution not supported on Web".to_string()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
use super::runner::GatewayPromptSessionMode;
#[cfg(not(target_arch = "wasm32"))]
fn execute_task_in_workspace(
    endpoint: fn() -> vw_gateway_client::GatewayEndpoint,
    gateway_client: fn() -> Result<vw_gateway_client::GatewayClient, String>,
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
    mode: GatewayPromptSessionMode,
) -> (String, Result<String, String>) {
    // Preserve home validation before acquiring the worktree claim.
    if super::programs::user_home_dir().is_none() {
        return super::runner::execute_task_blocking_in_mode(
            endpoint,
            project_path.clone(),
            task,
            project_path,
            log_sender,
            mode,
        );
    }
    let (workspace, _claim_guard) = match super::worktree_admin::resolve_task_execution_workspace(
        gateway_client,
        &task,
        &project_path,
        log_sender.as_ref(),
    ) {
        Ok(result) => result,
        Err(error) => return (task.id, Err(error)),
    };
    super::runner::execute_task_blocking_in_mode(
        endpoint,
        workspace.execution_path,
        task,
        project_path,
        log_sender,
        mode,
    )
}

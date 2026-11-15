//! 桌面网关配置与工作区生命周期适配。
use super::*;
pub use desktop_task_executor::executor::runner::{TaskPlanGenerationOutcome, TaskPlanSubTask, write_task_plan_files};
pub async fn execute_task_plan_async(task: Task, project_path: String, log_sender: Option<Sender<TaskLogStream>>) -> (String, Result<TaskPlanGenerationOutcome, String>) {
 desktop_task_executor::executor::runner::execute_task_plan_async(crate::app::config::gateway_client_endpoint, task, project_path, log_sender).await
}
pub async fn execute_task_review_async(task: Task, project_path: String, log_sender: Option<Sender<TaskLogStream>>) -> (String, Result<String, String>) {
 desktop_task_executor::executor::runner::execute_task_review_async(crate::app::config::gateway_client_endpoint, task, project_path, log_sender).await
}
pub async fn execute_task_merge_async(task: Task, project_path: String, log_sender: Option<Sender<TaskLogStream>>) -> (String, Result<String, String>) {
 desktop_task_executor::executor::runner::execute_task_merge_async(crate::app::config::gateway_client_endpoint, task, project_path, log_sender).await
}
pub fn execute_gateway_prompt_with_streaming(session_id: &str, execution_path: &str, model: &str, prompt: &str, agent: Option<String>, acp_agent: Option<String>, sender: Option<&Sender<TaskLogStream>>) -> Result<String, String> {
 desktop_task_executor::executor::runner::execute_gateway_prompt_with_streaming(crate::app::config::gateway_client_endpoint, session_id, execution_path, model, prompt, agent, acp_agent, sender)
}
/// 公开的 execute_task_async 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub async fn execute_task_async(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let task_id = task.id.clone();
        tokio::task::spawn_blocking(move || {
            execute_task_blocking_in_mode(
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
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let task_id = task.id.clone();
        tokio::task::spawn_blocking(move || {
            execute_task_blocking_in_mode(
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
use desktop_task_executor::executor::runner::GatewayPromptSessionMode;
#[cfg(not(target_arch = "wasm32"))]
fn execute_task_blocking_in_mode(task: Task, project_path: String, log_sender: Option<Sender<TaskLogStream>>, mode: GatewayPromptSessionMode) -> (String, Result<String, String>) {
    // Preserve home validation before acquiring the worktree claim.
    if super::programs::user_home_dir().is_none() {
        return desktop_task_executor::executor::runner::execute_task_blocking_in_mode(crate::app::config::gateway_client_endpoint, project_path.clone(), task, project_path, log_sender, mode);
    }
    let (workspace, _claim_guard) = match super::worktree_admin::resolve_task_execution_workspace(&task, &project_path, log_sender.as_ref()) {
        Ok(result) => result,
        Err(error) => return (task.id, Err(error)),
    };
    desktop_task_executor::executor::runner::execute_task_blocking_in_mode(crate::app::config::gateway_client_endpoint, workspace.execution_path, task, project_path, log_sender, mode)
}

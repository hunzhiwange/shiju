//! 桌面网关配置与工作区生命周期适配。
use super::*;
pub use desktop_task_executor::executor::runner::{
    TaskPlanGenerationOutcome, TaskPlanSubTask, write_task_plan_files,
};
pub async fn execute_task_plan_async(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<TaskPlanGenerationOutcome, String>) {
    desktop_task_executor::executor::runner::execute_task_plan_async(
        crate::app::config::gateway_client_endpoint,
        task,
        project_path,
        log_sender,
    )
    .await
}
pub async fn execute_task_review_async(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    desktop_task_executor::executor::runner::execute_task_review_async(
        crate::app::config::gateway_client_endpoint,
        task,
        project_path,
        log_sender,
    )
    .await
}
pub async fn execute_task_merge_async(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    desktop_task_executor::executor::runner::execute_task_merge_async(
        crate::app::config::gateway_client_endpoint,
        task,
        project_path,
        log_sender,
    )
    .await
}
pub fn execute_gateway_prompt_with_streaming(
    session_id: &str,
    execution_path: &str,
    model: &str,
    prompt: &str,
    agent: Option<String>,
    acp_agent: Option<String>,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<String, String> {
    desktop_task_executor::executor::runner::execute_gateway_prompt_with_streaming(
        crate::app::config::gateway_client_endpoint,
        session_id,
        execution_path,
        model,
        prompt,
        agent,
        acp_agent,
        sender,
    )
}
pub async fn execute_task_async(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    desktop_task_executor::executor::execution::execute_task_async(
        crate::app::config::gateway_client_endpoint,
        crate::app::config::gateway_client,
        task,
        project_path,
        log_sender,
    )
    .await
}
pub async fn execute_task_retry_async(
    task: Task,
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> (String, Result<String, String>) {
    desktop_task_executor::executor::execution::execute_task_retry_async(
        crate::app::config::gateway_client_endpoint,
        crate::app::config::gateway_client,
        task,
        project_path,
        log_sender,
    )
    .await
}

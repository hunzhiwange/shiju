//! 定时Agent任务投递到现有SQLite任务池。
use agent_config::runtime::Config;
use crate::CronJob;
pub async fn deliver_agent_job_to_task_pool(
    config: &Config,
    job: &CronJob,
    prompt: &str,
    model: String,
) -> anyhow::Result<String> {
    let project_path = config.workspace_dir.to_string_lossy().to_string();
    let prompt = prompt.to_string();
    let crate::execution::TaskPoolAgents { agent, acp_agent } =
        crate::execution::task_pool_agents(job);

    let task = tokio::task::spawn_blocking(move || {
        let mut task = task_file_store::bootstrap_task::Task::new(999);
        task.prompt = prompt;
        task.model = model;
        task.agent = Some(agent);
        task.acp_agent = acp_agent;
        task_file_store::bootstrap_task::create_task(&project_path, task)
    })
    .await
    .map_err(|err| anyhow::anyhow!("task pool create task join failed: {err}"))??;

    Ok(format!("agent job delivered to task pool: {}", task.id))
}

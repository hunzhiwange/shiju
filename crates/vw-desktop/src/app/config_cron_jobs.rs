//! Desktop endpoint resolution for cron configuration.

use vw_gateway_client::{CronJobDto, CronRunDto};

pub async fn load_cron_jobs_async() -> Result<Vec<CronJobDto>, String> {
    desktop_config::config_cron_jobs::load_cron_jobs_async(super::gateway::gateway_client()).await
}

pub async fn load_cron_history_async()
-> Result<(vw_gateway_client::CronHistoryResponse, Vec<CronJobDto>), String> {
    desktop_config::config_cron_jobs::load_cron_history_async(super::gateway::gateway_client()).await
}

pub async fn load_cron_job_runs_async(job_id: String) -> Result<Vec<CronRunDto>, String> {
    desktop_config::config_cron_jobs::load_cron_job_runs_async(super::gateway::gateway_client(), job_id).await
}

pub async fn add_cron_job_async(
    name: String,
    job_type: String,
    schedule_kind: String,
    schedule: String,
    at: String,
    every_ms: String,
    command: String,
    prompt: String,
    session_target: String,
    agent: String,
    acp_agent: String,
    project_path: String,
    wake: bool,
    model: String,
    fallbacks: String,
    full_access: bool,
    task_pool: bool,
    delivery_enabled: bool,
    delivery_channel: String,
    delivery_to: String,
    delivery_best_effort: bool,
    delete_after_run: bool,
    time_rule: Option<vw_gateway_client::CronPreviewRequest>,
) -> Result<(), String> {
    desktop_config::config_cron_jobs::add_cron_job_async(super::gateway::gateway_client(), name, job_type, schedule_kind, schedule, at, every_ms, command, prompt, session_target, agent, acp_agent, project_path, wake, model, fallbacks, full_access, task_pool, delivery_enabled, delivery_channel, delivery_to, delivery_best_effort, delete_after_run, time_rule).await
}

pub async fn update_cron_job_async(
    job_id: String,
    name: String,
    job_type: String,
    schedule_kind: String,
    schedule: String,
    at: String,
    every_ms: String,
    command: String,
    prompt: String,
    session_target: String,
    agent: String,
    acp_agent: String,
    project_path: String,
    wake: bool,
    model: String,
    fallbacks: String,
    full_access: bool,
    task_pool: bool,
    delivery_enabled: bool,
    delivery_channel: String,
    delivery_to: String,
    delivery_best_effort: bool,
    delete_after_run: bool,
    time_rule: Option<vw_gateway_client::CronPreviewRequest>,
) -> Result<(), String> {
    desktop_config::config_cron_jobs::update_cron_job_async(super::gateway::gateway_client(), job_id, name, job_type, schedule_kind, schedule, at, every_ms, command, prompt, session_target, agent, acp_agent, project_path, wake, model, fallbacks, full_access, task_pool, delivery_enabled, delivery_channel, delivery_to, delivery_best_effort, delete_after_run, time_rule).await
}

pub async fn set_cron_job_enabled_async(job_id: String, enabled: bool) -> Result<(), String> {
    desktop_config::config_cron_jobs::set_cron_job_enabled_async(super::gateway::gateway_client(), job_id, enabled).await
}

pub async fn set_cron_jobs_enabled_async(
    job_ids: Vec<String>,
    enabled: bool,
) -> Result<(), String> {
    desktop_config::config_cron_jobs::set_cron_jobs_enabled_async(super::gateway::gateway_client(), job_ids, enabled).await
}

pub async fn delete_cron_job_async(job_id: String) -> Result<(), String> {
    desktop_config::config_cron_jobs::delete_cron_job_async(super::gateway::gateway_client(), job_id).await
}

pub async fn delete_cron_jobs_async(job_ids: Vec<String>) -> Result<(), String> {
    desktop_config::config_cron_jobs::delete_cron_jobs_async(super::gateway::gateway_client(), job_ids).await
}

pub async fn preview_cron_schedule_async(
    rule: vw_gateway_client::CronPreviewRequest,
) -> Result<vw_gateway_client::CronPreviewResponse, String> {
    desktop_config::config_cron_jobs::preview_cron_schedule_async(super::gateway::gateway_client(), rule).await
}

#[cfg(test)]
#[path = "config_cron_jobs_tests.rs"]
mod tests;

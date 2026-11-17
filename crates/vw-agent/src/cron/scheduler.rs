//! # 定时任务调度器模块
//!
//! 本模块实现了 VibeWindow 代理系统的定时任务（Cron Job）调度核心逻辑。
//!
//! ## 主要功能
//!
//! - **调度执行**：定期轮询数据库中的到期任务，并按配置的并发度执行
//! - **任务类型支持**：支持两种任务类型
//!   - Shell 命令任务：执行系统命令或脚本
//!   - Agent 任务：触发代理执行特定的提示词任务
//! - **安全策略集成**：所有任务执行前都会通过安全策略验证
//! - **重试机制**：失败的任务支持自动重试，采用指数退避策略
//! - **结果投递**：支持将任务执行结果投递到多种通道（Telegram、Discord、Slack等）
//! - **健康监控**：持续向健康检查系统报告调度器状态
//!
//! ## 架构设计
//!
//! 调度器采用以下设计模式：
//! - 使用 tokio interval 实现定时轮询
//! - 使用 futures buffer_unordered 控制并发度
//! - 与安全策略模块深度集成，确保所有操作符合安全约束
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use vibe_agent::app::agent::config::Config;
//! use vibe_agent::app::agent::cron::scheduler;
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = Config::load().unwrap();
//!     scheduler::run(config).await.unwrap();
//! }
//! ```
//!
//! ## 注意事项
//!
//! - WASM 目标平台不支持 Shell 命令执行和部分通道功能
//! - 高频 Agent 任务（间隔小于5分钟）会触发警告
//! - 一次性任务执行成功后会自动删除，失败则会被禁用

use super::CronJob;
use crate::app::agent::config::Config;
use anyhow::Result;
#[cfg(test)]
use agent_security::SecurityPolicy;
#[cfg(test)]
use chrono::{DateTime, Utc};
#[cfg(test)]
use std::sync::Arc;
// Execution rule tests live in agent-cron.
// Shell command and pure scheduling tests live in agent-cron.
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use agent_cron::scheduler::notify_schedule_changed;
#[cfg(not(target_arch = "wasm32"))]
use agent_cron::scheduler::{cron_gateway_agent_request, format_gateway_agent_output};
// Gateway request options are tested by their agent-cron owner.

pub(crate) struct Host;
#[async_trait::async_trait]
impl agent_cron::scheduler::SchedulerHost for Host {
    #[cfg(not(target_arch = "wasm32"))]
    async fn run_gateway_agent_job(&self, config: &Config, job: &CronJob, prompt: &str, model_override: Option<String>) -> Result<String> {
        run_gateway_agent_job(config, job, prompt, model_override).await
    }
    async fn deliver_announcement(&self, config: &Config, channel: &str, target: &str, output: &str) -> Result<()> {
        deliver_announcement(config, channel, target, output).await
    }
}
#[cfg(not(target_arch = "wasm32"))]
async fn run_gateway_agent_job(
    config: &Config,
    job: &CronJob,
    prompt: &str,
    model_override: Option<String>,
) -> anyhow::Result<String> {
    let (session_id, request) = cron_gateway_agent_request(config, job, prompt, model_override);

    let collected =
        crate::app::agent::gateway::api::handlers::session::stream::collect_background_gateway_chat_stream(
            config.workspace_dir.to_string_lossy().to_string(),
            request,
            job.full_access,
            |_| {},
        )
        .await
        .map_err(|err| anyhow::anyhow!("{err}"))?;

    Ok(format_gateway_agent_output(&session_id, &collected))
}



pub(crate) async fn deliver_announcement(config: &Config, channel: &str, target: &str, output: &str) -> Result<()> {
    agent_cron::announcement::deliver_announcement(&Host, config, channel, target, output).await
}

pub async fn run(config: Config) -> Result<()> {
    agent_cron::scheduler::run(&Host, config).await
}

pub async fn execute_job_now(config: &Config, job: &CronJob) -> (bool, String) {
    agent_cron::scheduler::execute_job_now(&Host, config, job).await
}

#[cfg(test)]
async fn process_due_jobs(
    config: &Config,
    security: &Arc<SecurityPolicy>,
    jobs: Vec<CronJob>,
    component: &str,
) {
    agent_cron::scheduler::process_due_jobs(&Host, config, security, jobs, component).await
}

#[cfg(test)]
async fn run_agent_job(
    config: &Config,
    security: &SecurityPolicy,
    job: &CronJob,
) -> (bool, String) {
    agent_cron::scheduler::run_agent_job(&Host, config, security, job).await
}

#[cfg(test)]
async fn persist_job_result(
    config: &Config,
    job: &CronJob,
    success: bool,
    output: &str,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
) -> bool {
    agent_cron::scheduler::persist_job_result(&Host, config, job, success, output, started_at, finished_at).await
}

#[cfg(test)]
#[path = "tests/scheduler.rs"]
mod tests;

#[cfg(test)]
#[path = "scheduler_tests.rs"]
mod scheduler_tests;

#[async_trait::async_trait]
impl agent_cron::delivery::DeliveryHost for Host {
    #[cfg(not(target_arch = "wasm32"))]
    fn validate_webhook(&self, url: &str) -> Result<()> {
        crate::channels::wecom::webhook::WeComWebhook::new(url).map(|_| ())
    }
    #[cfg(not(target_arch = "wasm32"))]
    async fn latest_config(&self, config: &Config) -> Result<Config> {
        super::delivery::latest_config(config).await
    }
    #[cfg(not(target_arch = "wasm32"))]
    async fn send_validated(&self, config: &Config, channel: &str, target: &str, output: &str) -> Result<()> {
        super::delivery::send_validated(config, channel, target, output).await
    }
}

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

#[cfg(not(target_arch = "wasm32"))]
use super::super::channels::{
    Channel, DiscordChannel, EmailChannel, MattermostChannel, QQChannel, SendMessage,
    TelegramChannel,
};
use super::CronJob;
use crate::app::agent::config::Config;
use anyhow::Result;
#[cfg(test)]
use agent_security::SecurityPolicy;
#[cfg(test)]
use chrono::{DateTime, Utc};
#[cfg(test)]
use std::sync::Arc;
#[cfg(test)]
use agent_cron::execution::*;
#[cfg(test)]
use agent_cron::scheduler::{run_job_command, run_job_command_with_timeout, cron_agent_prompt, cron_full_access_config, is_one_shot_auto_delete, warn_if_high_frequency_agent_job, is_high_frequency_agent_job, wait_for_schedule_scan};
pub(crate) use agent_cron::scheduler::notify_schedule_changed;
#[cfg(not(target_arch = "wasm32"))]
use agent_cron::scheduler::{cron_gateway_agent_request, format_gateway_agent_output};
#[cfg(all(test, not(target_arch = "wasm32")))]
use agent_cron::scheduler::cron_gateway_options;

pub(crate) struct Host;
#[async_trait::async_trait]
impl agent_cron::scheduler::SchedulerHost for Host {
    async fn deliver_agent_job_to_task_pool(&self, config: &Config, job: &CronJob, prompt: &str, model: String) -> Result<String> {
        deliver_agent_job_to_task_pool(config, job, prompt, model).await
    }
    #[cfg(not(target_arch = "wasm32"))]
    async fn run_gateway_agent_job(&self, config: &Config, job: &CronJob, prompt: &str, model_override: Option<String>) -> Result<String> {
        run_gateway_agent_job(config, job, prompt, model_override).await
    }
    async fn deliver_announcement(&self, config: &Config, channel: &str, target: &str, output: &str) -> Result<()> {
        deliver_announcement(config, channel, target, output).await
    }
}
async fn deliver_agent_job_to_task_pool(
    config: &Config,
    job: &CronJob,
    prompt: &str,
    model: String,
) -> anyhow::Result<String> {
    let project_path = config.workspace_dir.to_string_lossy().to_string();
    let prompt = prompt.to_string();
    let agent = job
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("main")
        .to_string();
    let acp_agent = job
        .acp_agent
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let task = tokio::task::spawn_blocking(move || {
        let mut task = crate::app::task::Task::new(999);
        task.prompt = prompt;
        task.model = model;
        task.agent = Some(agent);
        task.acp_agent = acp_agent;
        crate::app::task::create_task(&project_path, task)
    })
    .await
    .map_err(|err| anyhow::anyhow!("task pool create task join failed: {err}"))??;

    Ok(format!("agent job delivered to task pool: {}", task.id))
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



#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn deliver_announcement(
    config: &Config,
    channel: &str,
    target: &str,
    output: &str,
) -> Result<()> {
    let channel = channel.to_ascii_lowercase();
    if super::delivery::is_wecom(&channel) {
        return super::delivery::send(config, &channel, target, output).await;
    }
    if config.channels_config.unavailable_channels.contains(&format!("{channel}:default")) {
        anyhow::bail!(
            "channel credentials unavailable; repair channel configuration before delivery"
        );
    }
    // 根据通道类型路由到对应的实现
    match channel.as_str() {
        "telegram" => {
            // 获取 Telegram 配置
            let tg = config
                .channels_config
                .telegram
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("telegram channel not configured"))?;
            // 创建 Telegram 通道实例
            let channel = TelegramChannel::new(
                tg.bot_token.clone(),
                tg.allowed_users.clone(),
                tg.mention_only,
            )
            .with_workspace_dir(config.workspace_dir.clone());
            // 发送消息
            channel.send(&SendMessage::new(output, target)).await?;
        }
        "discord" => {
            // 获取 Discord 配置
            let dc = config
                .channels_config
                .discord
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("discord channel not configured"))?;
            // 创建 Discord 通道实例
            let channel = DiscordChannel::new(
                dc.bot_token.clone(),
                dc.guild_id.clone(),
                dc.allowed_users.clone(),
                dc.listen_to_bots,
                dc.mention_only,
            )
            .with_workspace_dir(config.workspace_dir.clone());
            // 发送消息
            channel.send(&SendMessage::new(output, target)).await?;
        }
        "slack" => {
            // 获取 Slack 配置
            let sl = config
                .channels_config
                .slack
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("slack channel not configured"))?;
            // 创建 Slack 通道实例
            let channel = channel_slack::SlackChannel::new(
                sl.bot_token.clone(),
                sl.channel_id.clone(),
                sl.allowed_users.clone(),
            );
            // 发送消息
            channel.send(&SendMessage::new(output, target)).await?;
        }
        "mattermost" => {
            // 获取 Mattermost 配置
            let mm = config
                .channels_config
                .mattermost
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("mattermost channel not configured"))?;
            // 创建 Mattermost 通道实例
            let channel = MattermostChannel::new(
                mm.url.clone(),
                mm.bot_token.clone(),
                mm.channel_id.clone(),
                mm.allowed_users.clone(),
                mm.thread_replies.unwrap_or(true),
                mm.mention_only.unwrap_or(false),
            );
            // 发送消息
            channel.send(&SendMessage::new(output, target)).await?;
        }
        "qq" => {
            // 获取 QQ 配置
            let qq = config
                .channels_config
                .qq
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("qq channel not configured"))?;
            // 创建 QQ 通道实例
            let channel =
                QQChannel::new(qq.app_id.clone(), qq.app_secret.clone(), qq.allowed_users.clone());
            // 发送消息
            channel.send(&SendMessage::new(output, target)).await?;
        }
        "email" => {
            // 获取邮件配置
            let email = config
                .channels_config
                .email
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("email channel not configured"))?;
            // 创建邮件通道实例
            let channel = EmailChannel::new(email.clone());
            // 发送邮件
            channel.send(&SendMessage::new(output, target)).await?;
        }
        other => anyhow::bail!("unsupported delivery channel: {other}"),
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn deliver_announcement(
    _config: &Config,
    _channel: &str,
    _target: &str,
    _output: &str,
) -> Result<()> {
    if super::delivery::is_wecom(_channel) {
        anyhow::bail!("企微投递不支持 WASM，请使用原生 daemon");
    }
    Ok(())
}

pub async fn run(config: Config) -> Result<()> {
    agent_cron::scheduler::run(&Host, config).await
}

pub async fn execute_job_now(config: &Config, job: &CronJob) -> (bool, String) {
    agent_cron::scheduler::execute_job_now(&Host, config, job).await
}

#[cfg(test)]
async fn execute_job_with_retry(
    config: &Config,
    security: &SecurityPolicy,
    job: &CronJob,
) -> (bool, String) {
    agent_cron::scheduler::execute_job_with_retry(&Host, config, security, job).await
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
async fn execute_and_persist_job(
    config: &Config,
    security: &SecurityPolicy,
    job: &CronJob,
    component: &str,
) -> (String, bool, String) {
    agent_cron::scheduler::execute_and_persist_job(&Host, config, security, job, component).await
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

pub(super) async fn deliver_if_configured(
    config: &Config,
    job: &CronJob,
    output: &str,
) -> Result<()> {
    agent_cron::scheduler::deliver_if_configured(&Host, config, job, output).await
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

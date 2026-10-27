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
    Channel, DiscordChannel, EmailChannel, MattermostChannel, QQChannel, SendMessage, SlackChannel,
    TelegramChannel,
};
use super::super::security::{AutonomyLevel, SecurityPolicy};
use super::{
    CronJob, CronJobPatch, DeliveryConfig, JobType, Schedule, due_jobs, next_run_for_schedule,
    record_last_run, record_run, remove_job, reschedule_after_run, update_job,
};
use crate::app::agent::config::Config;
use anyhow::Result;
use chrono::{DateTime, Utc};
use futures_util::{StreamExt, stream};
use serde_json::json;
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::process::Stdio;
use std::sync::{Arc, OnceLock};
#[cfg(not(target_arch = "wasm32"))]
use tokio::process::Command;
use tokio::time::{self, Duration};

/// 调度器轮询间隔的最小值（秒）
///
/// 用于防止用户配置过短的轮询间隔导致系统资源浪费
const MIN_POLL_SECONDS: u64 = 5;

/// Shell 任务执行的超时时间（秒）
///
/// 当 Shell 命令执行超过此时间后会被强制终止
const SHELL_JOB_TIMEOUT_SECS: u64 = 120;

/// 调度器组件在健康检查系统中的标识名称
const SCHEDULER_COMPONENT: &str = "scheduler";

const CRON_AUTOMATION_SYSTEM_PROMPT: &str =
    "我是自动化脚本，无法回答你的问题，你可以自己作出你认为最好的决定。";

static SCHEDULER_REFRESH: OnceLock<tokio::sync::Notify> = OnceLock::new();

fn scheduler_refresh() -> &'static tokio::sync::Notify {
    SCHEDULER_REFRESH.get_or_init(tokio::sync::Notify::new)
}

pub(crate) fn notify_schedule_changed() {
    scheduler_refresh().notify_one();
}

pub(crate) async fn wait_for_schedule_scan(interval: &mut time::Interval) {
    tokio::select! {
        _ = interval.tick() => {}
        _ = scheduler_refresh().notified() => {}
    }
}

/// 启动定时任务调度器的主循环
///
/// 该函数是调度器的核心入口，会无限循环执行以下操作：
/// 1. 按配置的轮询间隔定期检查数据库
/// 2. 查询所有到期的定时任务
/// 3. 并发执行到期的任务（受 max_concurrent 限制）
/// 4. 持续向健康检查系统报告调度器状态
///
/// # 参数
///
/// * `config` - 应用配置，包含调度器参数、安全策略、通道配置等
///
/// # 返回值
///
/// 返回 `Result<()>`，理论上该函数不会正常返回（无限循环），
/// 仅在初始化失败或不可恢复错误时返回 Err
///
/// # 示例
///
/// ```rust,no_run
/// use vibe_agent::app::agent::config::Config;
///
/// #[tokio::main]
/// async fn main() {
///     let config = Config::load().unwrap();
///     // 这将阻塞当前线程
///     vibe_agent::app::agent::cron::scheduler::run(config).await.unwrap();
/// }
/// ```
///
/// # 错误处理
///
/// - 轮询失败时会记录警告并继续下一轮循环
/// - 单个任务执行失败不影响其他任务
/// - 所有错误都会通过 tracing 记录
///
/// # 并发控制
///
/// 使用 `buffer_unordered` 控制并发度，由 `config.scheduler.max_concurrent` 配置
pub async fn run(config: Config) -> Result<()> {
    let poll_secs = config.reliability.scheduler_poll_secs.max(MIN_POLL_SECONDS);
    let mut interval = time::interval(Duration::from_secs(poll_secs));
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
    let security = Arc::new(SecurityPolicy::from_configs(
        &config.autonomy,
        &config.security,
        &config.workspace_dir,
    ));

    crate::app::agent::health::mark_component_ok(SCHEDULER_COMPONENT);

    loop {
        wait_for_schedule_scan(&mut interval).await;
        // Keep scheduler liveness fresh even when there are no due jobs.
        crate::app::agent::health::mark_component_ok(SCHEDULER_COMPONENT);

        let jobs = match due_jobs(&config, Utc::now()) {
            Ok(jobs) => jobs,
            Err(e) => {
                crate::app::agent::health::mark_component_error(SCHEDULER_COMPONENT, e.to_string());
                tracing::warn!("Scheduler query failed: {e}");
                continue;
            }
        };

        process_due_jobs(&config, &security, jobs, SCHEDULER_COMPONENT).await;
    }
}

/// 立即执行指定的定时任务（跳过调度）
///
/// 该函数用于手动触发任务执行，不经过调度器的轮询机制。
/// 主要用于以下场景：
/// - 手动测试定时任务
/// - 通过 API 触发即时执行
/// - 调试和故障排查
///
/// # 参数
///
/// * `config` - 应用配置引用
/// * `job` - 要执行的定时任务定义
///
/// # 返回值
///
/// 返回元组 `(bool, String)`：
/// - `bool`: 执行是否成功（true = 成功，false = 失败）
/// - `String`: 执行输出或错误信息
///
/// # 示例
///
/// ```rust,no_run
/// use vibe_agent::app::agent::config::Config;
/// use vibe_agent::app::agent::cron::CronJob;
///
/// async fn trigger_job(config: &Config, job: &CronJob) {
///     let (success, output) =
///         vibe_agent::app::agent::cron::scheduler::execute_job_now(config, job).await;
///
///     if success {
///         println!("任务执行成功: {}", output);
///     } else {
///         eprintln!("任务执行失败: {}", output);
///     }
/// }
/// ```
///
/// # 注意事项
///
/// - 该函数会应用重试机制（参见 `execute_job_with_retry`）
/// - 会进行完整的安全策略检查
/// - 不会自动更新任务的下一次执行时间
pub async fn execute_job_now(config: &Config, job: &CronJob) -> (bool, String) {
    let security =
        SecurityPolicy::from_configs(&config.autonomy, &config.security, &config.workspace_dir);
    execute_job_with_retry(config, &security, job).await
}

/// 执行任务并实现自动重试机制
///
/// 该函数封装了任务执行的核心逻辑，提供指数退避重试功能：
/// - 根据 `config.reliability.scheduler_retries` 配置重试次数
/// - 使用指数退避策略（初始退避时间来自 `provider_backoff_ms`）
/// - 添加随机抖动避免重试风暴
/// - 安全策略违规导致的失败不重试（确定性错误）
///
/// # 参数
///
/// * `config` - 应用配置
/// * `security` - 安全策略实例
/// * `job` - 要执行的任务
///
/// # 返回值
///
/// 返回 `(bool, String)`：
/// - 第一个元素表示最终执行结果（成功/失败）
/// - 第二个元素是最后一次执行的输出
///
/// # 重试策略
///
/// - 退避时间：初始值为 `provider_backoff_ms`（最小200ms），每次翻倍，最大30秒
/// - 抖动：0-250ms 的随机延迟
/// - 不重试条件：输出以 "blocked by security policy:" 开头
async fn execute_job_with_retry(
    config: &Config,
    security: &SecurityPolicy,
    job: &CronJob,
) -> (bool, String) {
    let run_config = effective_job_config(config, job);
    let run_security;
    let security = if run_config.workspace_dir != config.workspace_dir {
        run_security = SecurityPolicy::from_configs(
            &run_config.autonomy,
            &run_config.security,
            &run_config.workspace_dir,
        );
        &run_security
    } else {
        security
    };

    let mut last_output = String::new();
    let retries = run_config.reliability.scheduler_retries;
    // 初始退避时间，确保最小值为 200ms
    let mut backoff_ms = run_config.reliability.provider_backoff_ms.max(200);

    // 执行重试循环：0 表示首次尝试，1..=retries 表示重试
    for attempt in 0..=retries {
        // 根据任务类型选择执行器
        let (success, output) = match effective_job_type(job) {
            JobType::Shell => run_job_command(&run_config, security, job).await,
            JobType::Agent => run_agent_job(&run_config, security, job).await,
        };
        last_output = output;

        // 执行成功，立即返回
        if success {
            return (true, last_output);
        }

        // 安全策略违规导致的失败是确定性的，重试无意义
        if last_output.starts_with("blocked by security policy:") {
            return (false, last_output);
        }

        // 如果还有重试机会，等待后继续
        if attempt < retries {
            // 添加随机抖动（0-250ms）避免重试风暴
            let jitter_ms = u64::from(Utc::now().timestamp_subsec_millis() % 250);
            time::sleep(Duration::from_millis(backoff_ms + jitter_ms)).await;
            // 指数退避，最大 30 秒
            backoff_ms = (backoff_ms.saturating_mul(2)).min(30_000);
        }
    }

    (false, last_output)
}

fn effective_job_config(config: &Config, job: &CronJob) -> Config {
    let Some(project_path) =
        job.project_path.as_deref().map(str::trim).filter(|value| !value.is_empty())
    else {
        return config.clone();
    };

    let mut next = config.clone();
    next.workspace_dir = PathBuf::from(project_path);
    next
}

fn effective_job_type(job: &CronJob) -> JobType {
    if matches!(job.job_type, JobType::Shell)
        && job.command.trim().is_empty()
        && job.prompt.as_deref().map(str::trim).is_some_and(|value| !value.is_empty())
    {
        return JobType::Agent;
    }
    job.job_type.clone()
}

fn invalid_job_payload(job: &CronJob) -> Option<String> {
    match effective_job_type(job) {
        JobType::Shell if job.command.trim().is_empty() => {
            Some("invalid cron job: shell command is empty; job disabled".to_string())
        }
        JobType::Agent
            if job.prompt.as_deref().map(str::trim).is_none_or(|value| value.is_empty()) =>
        {
            Some("invalid cron job: agent prompt is empty; job disabled".to_string())
        }
        _ => None,
    }
}

fn disable_invalid_job(config: &Config, job: &CronJob, output: &str, timestamp: DateTime<Utc>) {
    if let Err(err) = record_run(config, &job.id, timestamp, timestamp, "error", Some(output), 0) {
        tracing::warn!("Failed to record invalid cron job run: {err}");
    }
    if let Err(err) = record_last_run(config, &job.id, timestamp, false, output) {
        tracing::warn!("Failed to record invalid cron job status: {err}");
    }
    if let Err(err) = update_job(
        config,
        &job.id,
        CronJobPatch { enabled: Some(false), ..CronJobPatch::default() },
    ) {
        tracing::warn!("Failed to disable invalid cron job: {err}");
    }
}

/// 并发处理所有到期的定时任务
///
/// 该函数接收一批到期的任务，使用 futures 流处理器并发执行。
/// 并发度由 `config.scheduler.max_concurrent` 控制。
///
/// # 参数
///
/// * `config` - 应用配置
/// * `security` - 安全策略（Arc 包装，用于跨任务共享）
/// * `jobs` - 到期的任务列表
/// * `component` - 健康检查组件名称
///
/// # 执行流程
///
/// 1. 更新健康检查状态
/// 2. 将任务转换为异步执行流
/// 3. 使用 buffer_unordered 控制并发执行
/// 4. 收集并记录执行结果
///
/// # 并发模型
///
/// 使用 `stream::iter` + `buffer_unordered` 实现：
/// - 任务按到达顺序开始执行
/// - 最多同时运行 `max_concurrent` 个任务
/// - 任务完成顺序不确定
async fn process_due_jobs(
    config: &Config,
    security: &Arc<SecurityPolicy>,
    jobs: Vec<CronJob>,
    component: &str,
) {
    // 刷新调度器健康状态
    crate::app::agent::health::mark_component_ok(component);

    // 获取最大并发数，确保至少为 1
    let max_concurrent = config.scheduler.max_concurrent.max(1);
    // 创建并发执行的流
    let mut in_flight =
        stream::iter(
            jobs.into_iter().map(|job| {
                // 克隆配置和安全策略用于异步闭包
                let config = config.clone();
                let security = Arc::clone(security);
                let component = component.to_owned();
                async move {
                    execute_and_persist_job(&config, security.as_ref(), &job, &component).await
                }
            }),
        )
        .buffer_unordered(max_concurrent);

    // 等待所有任务完成，记录失败的任务
    while let Some((job_id, success, output)) = in_flight.next().await {
        if !success {
            tracing::warn!("Scheduler job '{job_id}' failed: {output}");
        }
    }
}

/// 执行单个任务并持久化执行结果
///
/// 该函数是任务执行的完整生命周期管理器，负责：
/// 1. 记录任务开始时间
/// 2. 执行任务（含重试）
/// 3. 可选的结果投递
/// 4. 记录执行历史到数据库
/// 5. 更新任务调度状态
///
/// # 参数
///
/// * `config` - 应用配置
/// * `security` - 安全策略
/// * `job` - 要执行的任务
/// * `component` - 健康检查组件名称
///
/// # 返回值
///
/// 返回元组 `(String, bool, String)`：
/// - `String`: 任务 ID
/// - `bool`: 执行是否成功
/// - `String`: 执行输出或错误信息
///
/// # 副作用
///
/// - 写入执行历史记录到数据库
/// - 可能发送投递消息到外部通道
/// - 对于一次性任务，可能删除或禁用任务
async fn execute_and_persist_job(
    config: &Config,
    security: &SecurityPolicy,
    job: &CronJob,
    component: &str,
) -> (String, bool, String) {
    // 更新健康检查状态
    crate::app::agent::health::mark_component_ok(component);
    if let Some(output) = invalid_job_payload(job) {
        disable_invalid_job(config, job, &output, Utc::now());
        return (job.id.clone(), false, output);
    }
    // 检查并警告高频 Agent 任务
    warn_if_high_frequency_agent_job(job);

    // 记录执行时间用于统计
    let started_at = Utc::now();
    // 执行任务（含重试逻辑）
    let (success, output) = execute_job_with_retry(config, security, job).await;
    let finished_at = Utc::now();
    // 持久化执行结果
    let success = persist_job_result(config, job, success, &output, started_at, finished_at).await;

    (job.id.clone(), success, output)
}

/// 执行 Agent 类型的定时任务
///
/// 该函数通过调用代理核心来执行 AI 驱动的任务。
/// 任务的 prompt 会被自动添加前缀以标识其来源。
///
/// # 参数
///
/// * `config` - 应用配置
/// * `security` - 安全策略实例
/// * `job` - Agent 任务定义
///
/// # 返回值
///
/// 返回 `(bool, String)`：
/// - `bool`: 执行是否成功
/// - `String`: 执行结果描述或错误信息
///
/// # 安全检查
///
/// 执行前会依次检查：
/// 1. `can_act()` - 是否允许自主操作
/// 2. `is_rate_limited()` - 是否超出速率限制
/// 3. `record_action()` - 是否有剩余操作配额
///
/// # Prompt 处理
///
/// 任务 prompt 会被格式化为：`[cron:{job_id} {name}] {original_prompt}`
/// 这样可以在日志和代理会话中清晰地追踪任务来源
///
/// # 平台限制
///
/// - WASM 平台不支持 Agent 任务，会直接返回成功但不执行
async fn run_agent_job(
    config: &Config,
    security: &SecurityPolicy,
    job: &CronJob,
) -> (bool, String) {
    if !job.full_access {
        // 安全策略检查：是否允许自主操作
        if !security.can_act() {
            return (false, "blocked by security policy: autonomy is read-only".to_string());
        }

        // 安全策略检查：是否超出速率限制
        if security.is_rate_limited() {
            return (false, "blocked by security policy: rate limit exceeded".to_string());
        }

        // 安全策略检查：是否还有操作配额
        if !security.record_action() {
            return (false, "blocked by security policy: action budget exhausted".to_string());
        }
    }

    let run_config = if job.full_access { cron_full_access_config(config) } else { config.clone() };
    // 准备任务参数
    let name = job.name.clone().unwrap_or_else(|| "cron-job".to_string());
    let prompt = job.prompt.clone().unwrap_or_default();
    // 添加任务标识前缀，便于追踪
    let prefixed_prompt = cron_agent_prompt(job, &name, &prompt);
    let primary_model = job.model.clone();

    let delegate_agent_key = job.agent.as_deref().map(str::trim).filter(|value| !value.is_empty());
    if let Some(agent_key) = delegate_agent_key {
        let Some(agent_config) = run_config.agents.get(agent_key) else {
            return (false, format!("agent job failed: delegate agent '{agent_key}' not found"));
        };
        if !agent_config.enabled {
            return (false, format!("agent job failed: delegate agent '{agent_key}' is disabled"));
        }
    }

    if job.task_pool {
        let model = primary_model
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "auto".to_string());
        return match deliver_agent_job_to_task_pool(&run_config, job, &prefixed_prompt, model).await
        {
            Ok(output) => (true, output),
            Err(err) => (false, format!("agent task pool delivery failed: {err}")),
        };
    }

    let model_candidates = agent_model_candidates(primary_model, &job.fallbacks);

    let mut last_error = None;
    for model_override in model_candidates {
        let run_result: anyhow::Result<String> = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                run_gateway_agent_job(&run_config, job, &prefixed_prompt, model_override).await
            }
            #[cfg(target_arch = "wasm32")]
            {
                let _ = model_override;
                // WASM 平台不支持 Agent 任务
                Ok("agent job executed".to_string())
            }
        };

        match run_result {
            Ok(output) => {
                return (true, normalize_agent_job_success_output(&output));
            }
            Err(err) => {
                last_error = Some(err);
                continue;
            }
        }
    }

    let run_result: anyhow::Result<()> = match last_error {
        Some(err) => Err(err),
        None => Ok(()),
    };

    // 返回执行结果
    match run_result {
        Ok(_) => (true, "agent job executed".to_string()),
        Err(e) => (false, format!("agent job failed: {e}")),
    }
}

fn normalize_agent_job_success_output(output: &str) -> String {
    let output = output.trim();
    if output.is_empty() { "agent job executed".to_string() } else { output.to_string() }
}

fn agent_model_candidates(
    primary_model: Option<String>,
    fallbacks: &[String],
) -> Vec<Option<String>> {
    let mut model_candidates = Vec::new();
    if let Some(model) = primary_model.filter(|value| !value.trim().is_empty()) {
        model_candidates.push(Some(model));
    } else {
        model_candidates.push(None);
    }
    for fallback in fallbacks {
        let fallback = fallback.trim();
        if !fallback.is_empty()
            && !model_candidates.iter().any(|candidate| candidate.as_deref() == Some(fallback))
        {
            model_candidates.push(Some(fallback.to_string()));
        }
    }
    model_candidates
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
        crate::app::agent::gateway::api::handlers::session::stream::collect_gateway_chat_stream(
            config.workspace_dir.to_string_lossy().to_string(),
            request,
        )
        .await
        .map_err(|err| anyhow::anyhow!("{err}"))?;

    Ok(format_gateway_agent_output(&session_id, &collected))
}

#[cfg(not(target_arch = "wasm32"))]
fn cron_gateway_agent_request(
    config: &Config,
    job: &CronJob,
    prompt: &str,
    model_override: Option<String>,
) -> (String, vw_api_types::chat::GatewayChatStreamRequest) {
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
    let session_id = format!("cron-{}", job.id);

    let options = cron_gateway_options(config, job, acp_agent.as_deref());

    let request = vw_api_types::chat::GatewayChatStreamRequest {
        session_id: Some(session_id.clone().into()),
        messages: vec![json!({ "role": "user", "content": prompt })],
        system: None,
        model: model_override,
        reasoning_effort: None,
        agent: Some(agent),
        allowed_tools: None,
        acp_agent,
        acp_allowed_tools: None,
        options: (!options.is_empty()).then_some(serde_json::Value::Object(options)),
    };

    (session_id, request)
}

#[cfg(not(target_arch = "wasm32"))]
fn format_gateway_agent_output(
    session_id: &str,
    collected: &crate::app::agent::gateway::api::handlers::session::stream::GatewayChatStreamCollected,
) -> String {
    let mut output = collected.output.trim().to_string();
    if output.is_empty() {
        output = "agent job executed".to_string();
    }
    if let Some(message_id) = collected.message_id.as_deref() {
        output.push_str(&format!("\n[session:{session_id} message:{message_id}]"));
    } else if let Some(parent_message_id) = collected.parent_message_id.as_deref() {
        output.push_str(&format!("\n[session:{session_id} parent:{parent_message_id}]"));
    } else {
        output.push_str(&format!("\n[session:{session_id}]"));
    }
    output
}

#[cfg(not(target_arch = "wasm32"))]
fn cron_gateway_options(
    config: &Config,
    job: &CronJob,
    acp_agent: Option<&str>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut options = serde_json::Map::new();
    if job.full_access || acp_agent.is_some() {
        options.insert("full_access".to_string(), serde_json::Value::Bool(true));
    }
    if let Some(acp_agent) = acp_agent {
        options.insert("acp_test".to_string(), serde_json::Value::Bool(true));
        options.insert("acp_agent".to_string(), serde_json::Value::String(acp_agent.to_string()));
        options.insert("acp_force_new_session".to_string(), serde_json::Value::Bool(true));
        options.insert("acp_close_client_after_request".to_string(), serde_json::Value::Bool(true));
        options.insert(
            "cwd".to_string(),
            serde_json::Value::String(config.workspace_dir.to_string_lossy().to_string()),
        );
        options.insert(
            "acp_permission_mode".to_string(),
            serde_json::Value::String("approve-all".to_string()),
        );
    }
    options
}

fn cron_agent_prompt(job: &CronJob, name: &str, prompt: &str) -> String {
    if job.full_access {
        format!("[cron:{} {name}]\n{CRON_AUTOMATION_SYSTEM_PROMPT}\n\n{prompt}", job.id)
    } else {
        format!("[cron:{} {name}] {prompt}", job.id)
    }
}

fn cron_full_access_config(config: &Config) -> Config {
    let mut run_config = config.clone();
    run_config.autonomy.level = AutonomyLevel::Full;
    run_config.autonomy.workspace_only = false;
    run_config.autonomy.allowed_commands = vec!["*".to_string()];
    run_config.autonomy.forbidden_paths.clear();
    run_config.autonomy.max_actions_per_hour = u32::MAX;
    run_config.autonomy.require_approval_for_medium_risk = false;
    run_config.autonomy.block_high_risk_commands = false;
    run_config.autonomy.auto_approve = vec!["*".to_string()];
    run_config.autonomy.always_ask.clear();
    run_config.autonomy.allowed_roots = vec!["/".to_string()];
    run_config.autonomy.non_cli_excluded_tools.clear();
    run_config.autonomy.allow_unsafe_shell_patterns = true;
    run_config
}

/// 持久化任务执行结果到数据库
///
/// 该函数负责将任务执行结果写入持久化存储，包括：
/// - 可选的结果投递到外部通道
/// - 记录执行历史（开始时间、结束时间、状态、输出）
/// - 处理一次性任务的特殊逻辑
/// - 更新任务的下次执行时间
///
/// # 参数
///
/// * `config` - 应用配置
/// * `job` - 执行的任务
/// * `success` - 初始执行状态（可能因投递失败而改变）
/// * `output` - 执行输出
/// * `started_at` - 任务开始时间
/// * `finished_at` - 任务结束时间
///
/// # 返回值
///
/// 返回最终的执行状态（`true` = 成功，`false` = 失败）
/// 注意：即使任务本身成功，投递失败也可能导致最终状态为失败
///
/// # 投递逻辑
///
/// 如果配置了投递（delivery.mode = "announce"）：
/// - 投递失败时，根据 `best_effort` 决定是否将整体状态标记为失败
/// - `best_effort = true`: 投递失败仅记录警告
/// - `best_effort = false`: 投递失败会将任务标记为失败
///
/// # 一次性任务处理
///
/// 对于 `delete_after_run = true` 且 `Schedule::At` 类型的任务：
/// - 成功：删除任务记录
/// - 失败：记录失败并禁用任务（不删除）
async fn persist_job_result(
    config: &Config,
    job: &CronJob,
    mut success: bool,
    output: &str,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
) -> bool {
    // 计算执行时长（毫秒）
    let duration_ms = (finished_at - started_at).num_milliseconds();

    // 尝试投递结果到配置的通道
    if let Err(e) = deliver_if_configured(config, job, output).await {
        if job.delivery.best_effort {
            // best_effort 模式：投递失败不影响整体状态
            tracing::warn!("Cron delivery failed (best_effort): {e}");
        } else {
            // 非 best_effort 模式：投递失败会导致任务失败
            success = false;
            tracing::warn!("Cron delivery failed: {e}");
        }
    }

    // 记录执行历史到数据库
    let _ = record_run(
        config,
        &job.id,
        started_at,
        finished_at,
        if success { "ok" } else { "error" },
        Some(output),
        duration_ms,
    );

    // 处理一次性任务的特殊逻辑
    if is_one_shot_auto_delete(job) {
        if success {
            // 一次性任务成功：删除任务
            if let Err(e) = remove_job(config, &job.id) {
                tracing::warn!("Failed to remove one-shot cron job after success: {e}");
            }
        } else {
            // 一次性任务失败：记录失败并禁用（保留任务以便调试）
            let _ = record_last_run(config, &job.id, finished_at, false, output);
            if let Err(e) = update_job(
                config,
                &job.id,
                CronJobPatch { enabled: Some(false), ..CronJobPatch::default() },
            ) {
                tracing::warn!("Failed to disable failed one-shot cron job: {e}");
            }
        }
        return success;
    }

    // 对于周期性任务：更新下次执行时间
    if let Err(e) = reschedule_after_run(config, job, success, output) {
        tracing::warn!("Failed to persist scheduler run result: {e}");
    }

    success
}

/// 判断任务是否为一次性自动删除任务
///
/// 一次性任务是指：
/// - `delete_after_run = true`：执行后需要清理
/// - `Schedule::At`：指定了具体的执行时间点（非周期性）
///
/// 这类任务在成功执行后会被自动删除，失败后会被禁用。
///
/// # 参数
///
/// * `job` - 任务定义
///
/// # 返回值
///
/// 如果是一次性自动删除任务返回 `true`，否则返回 `false`
fn is_one_shot_auto_delete(job: &CronJob) -> bool {
    job.delete_after_run && matches!(job.schedule, Schedule::At { .. })
}

/// 检查并警告高频 Agent 任务
///
/// Agent 任务通常涉及 AI 模型调用，执行成本较高。
/// 如果任务调度频率过高（间隔小于5分钟），会输出警告日志。
///
/// 该函数用于防止以下情况：
/// - 意外配置了过于频繁的 Agent 任务
/// - 导致不必要的 API 调用和成本
/// - 影响系统整体性能
///
/// # 参数
///
/// * `job` - 任务定义
///
/// # 频率判断逻辑
///
/// - `Schedule::Every`: 直接检查 `every_ms` 是否小于 5 分钟
/// - `Schedule::Cron`: 计算两个连续时间点的下次执行时间，判断间隔
/// - `Schedule::At`: 一次性任务，不算高频
fn warn_if_high_frequency_agent_job(job: &CronJob) {
    if is_high_frequency_agent_job(job) {
        tracing::warn!(
            "Cron agent job '{}' is scheduled more frequently than every 5 minutes",
            job.id
        );
    }
}

fn is_high_frequency_agent_job(job: &CronJob) -> bool {
    // 只检查 Agent 类型任务
    if !matches!(effective_job_type(job), JobType::Agent) {
        return false;
    }

    // 判断任务是否过于频繁
    match &job.schedule {
        Schedule::Every { every_ms } => *every_ms < 5 * 60 * 1000,
        Schedule::Cron { .. } => {
            // 对于 cron 表达式，比较连续两次实际执行时间。
            let now = Utc::now();
            match next_run_for_schedule(&job.schedule, now) {
                Ok(first) => match next_run_for_schedule(&job.schedule, first) {
                    Ok(second) => second - first < chrono::Duration::minutes(5),
                    Err(_) => false,
                },
                _ => false,
            }
        }
        Schedule::At { .. } => false,
    }
}

/// 根据配置投递任务执行结果
///
/// 如果任务配置了 `delivery.mode = "announce"`，会将执行结果
/// 投递到指定的通道和目标。
///
/// # 参数
///
/// * `config` - 应用配置
/// * `job` - 任务定义（包含投递配置）
/// * `output` - 要投递的执行结果
///
/// # 返回值
///
/// - `Ok(())`: 投递成功或无需投递
/// - `Err(e)`: 投递配置错误或投递失败
///
/// # 投递条件
///
/// 只有当 `delivery.mode` 为 "announce"（不区分大小写）时才执行投递
async fn deliver_if_configured(config: &Config, job: &CronJob, output: &str) -> Result<()> {
    let delivery: &DeliveryConfig = &job.delivery;
    // 检查是否启用了投递模式
    if !delivery.mode.eq_ignore_ascii_case("announce") {
        return Ok(());
    }

    // 验证必需的投递配置
    let channel = delivery
        .channel
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("delivery.channel is required for announce mode"))?;
    let target = delivery
        .to
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("delivery.to is required for announce mode"))?;

    deliver_announcement(config, channel, target, output).await
}

/// 投递公告消息到指定的通道
///
/// 该函数根据通道类型将消息投递到对应的外部系统。
/// 支持多种主流通信平台的集成。
///
/// # 参数
///
/// * `config` - 应用配置（包含各通道的配置信息）
/// * `channel` - 目标通道类型（不区分大小写）
/// * `target` - 目标接收者（如频道 ID、用户 ID、邮箱地址等）
/// * `output` - 要发送的消息内容
///
/// # 返回值
///
/// - `Ok(())`: 消息发送成功
/// - `Err(e)`: 通道未配置、配置错误或发送失败
///
/// # 支持的通道
///
/// - `telegram`: Telegram 机器人
/// - `discord`: Discord 机器人
/// - `slack`: Slack 应用
/// - `mattermost`: Mattermost 机器人
/// - `qq`: QQ 机器人
/// - `email`: 电子邮件
///
/// # 平台限制
///
/// 该函数仅在非 WASM 平台可用（需要网络访问）
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn deliver_announcement(
    config: &Config,
    channel: &str,
    target: &str,
    output: &str,
) -> Result<()> {
    let channel = channel.to_ascii_lowercase();
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
            let channel = SlackChannel::new(
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

/// 执行 Shell 命令任务（使用默认超时）
///
/// 该函数是 `run_job_command_with_timeout` 的便捷包装，
/// 使用 `SHELL_JOB_TIMEOUT_SECS` 作为默认超时时间。
///
/// # 参数
///
/// * `config` - 应用配置
/// * `security` - 安全策略
/// * `job` - 包含要执行命令的任务
///
/// # 返回值
///
/// 返回 `(bool, String)`：
/// - `bool`: 执行是否成功
/// - `String`: 命令输出（包含 stdout、stderr 和退出状态）
async fn run_job_command(
    config: &Config,
    security: &SecurityPolicy,
    job: &CronJob,
) -> (bool, String) {
    run_job_command_with_timeout(config, security, job, Duration::from_secs(SHELL_JOB_TIMEOUT_SECS))
        .await
}

/// Shell 命令执行的 WASM 存根（空实现）
///
/// WASM 平台不支持执行系统命令，该函数始终返回失败。
/// 这是为了保持 API 一致性的占位实现。
///
/// # 参数
///
/// * `_config` - 未使用
/// * `_security` - 未使用
/// * `_job` - 未使用
/// * `_timeout` - 未使用
///
/// # 返回值
///
/// 始终返回 `(false, "Shell commands not supported on WASM")`
#[cfg(target_arch = "wasm32")]
async fn run_job_command_with_timeout(
    _config: &Config,
    _security: &SecurityPolicy,
    _job: &CronJob,
    _timeout: Duration,
) -> (bool, String) {
    (false, "Shell commands not supported on WASM".to_string())
}

/// 执行 Shell 命令任务（带超时控制）
///
/// 该函数是 Shell 命令执行的核心实现，包含完整的安全检查和超时控制。
///
/// # 执行流程
///
/// 1. 应用 Shell 重定向策略（安全过滤）
/// 2. 检查自主操作权限（`can_act`）
/// 3. 检查速率限制（`is_rate_limited`）
/// 4. 验证命令是否在允许列表中
/// 5. 检查命令参数中的禁止路径
/// 6. 记录操作配额消耗
/// 7. 在工作目录中执行命令
/// 8. 等待命令完成或超时
///
/// # 参数
///
/// * `config` - 应用配置
/// * `security` - 安全策略实例
/// * `job` - 包含要执行命令的任务
/// * `timeout` - 命令执行的最大等待时间
///
/// # 返回值
///
/// 返回 `(bool, String)`：
/// - `bool`: 执行是否成功（命令退出码为 0）
/// - `String`: 包含退出状态、stdout 和 stderr 的组合输出
///
/// # 命令执行环境
///
/// - 使用 `sh -lc` 执行命令（支持 shell 特性）
/// - 工作目录：`config.workspace_dir`
/// - stdin: 关闭（null）
/// - stdout/stderr: 捕获
/// - `kill_on_drop`: 进程会在句柄丢弃时被终止
///
/// # 安全策略
///
/// 任何安全检查失败都会立即返回错误，不会执行命令
#[cfg(not(target_arch = "wasm32"))]
async fn run_job_command_with_timeout(
    config: &Config,
    security: &SecurityPolicy,
    job: &CronJob,
    timeout: Duration,
) -> (bool, String) {
    // 应用安全策略中的 shell 重定向过滤
    let effective_command = security.apply_shell_redirect_policy(&job.command);

    // 安全检查 1: 是否允许自主操作
    if !security.can_act() {
        return (false, "blocked by security policy: autonomy is read-only".to_string());
    }

    // 安全检查 2: 是否超出速率限制
    if security.is_rate_limited() {
        return (false, "blocked by security policy: rate limit exceeded".to_string());
    }

    // 安全检查 3: 命令是否在允许列表中
    if !security.is_command_allowed(&effective_command) {
        return (
            false,
            format!("blocked by security policy: command not allowed: {}", job.command),
        );
    }

    // 安全检查 4: 命令参数中是否包含禁止的路径
    if let Some(path) = security.forbidden_path_argument(&effective_command) {
        return (false, format!("blocked by security policy: forbidden path argument: {path}"));
    }

    // 安全检查 5: 记录操作并检查配额
    if !security.record_action() {
        return (false, "blocked by security policy: action budget exhausted".to_string());
    }

    // 启动子进程执行命令
    let child = match Command::new("sh")
        .arg("-lc")
        .arg(&effective_command)
        .current_dir(&config.workspace_dir)
        .stdin(Stdio::null()) // 不接受输入
        .stdout(Stdio::piped()) // 捕获标准输出
        .stderr(Stdio::piped()) // 捕获标准错误
        .kill_on_drop(true) // 句柄丢弃时终止进程
        .spawn()
    {
        Ok(child) => child,
        Err(e) => return (false, format!("spawn error: {e}")),
    };

    // 等待命令完成，应用超时
    match time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(output)) => {
            // 命令正常完成，组合输出
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!(
                "status={}\nstdout:\n{}\nstderr:\n{}",
                output.status,
                stdout.trim(),
                stderr.trim()
            );
            (output.status.success(), combined)
        }
        Ok(Err(e)) => (false, format!("spawn error: {e}")),
        Err(_) => (false, format!("job timed out after {}s", timeout.as_secs_f64())),
    }
}

/// 消息投递的 WASM 存根（空实现）
///
/// WASM 平台不支持网络请求到外部通道，该函数为空操作。
/// 这是为了保持 API 一致性的占位实现。
///
/// # 参数
///
/// * `_config` - 未使用
/// * `_channel` - 未使用
/// * `_target` - 未使用
/// * `_output` - 未使用
///
/// # 返回值
///
/// 始终返回 `Ok(())`
#[cfg(target_arch = "wasm32")]
pub(crate) async fn deliver_announcement(
    _config: &Config,
    _channel: &str,
    _target: &str,
    _output: &str,
) -> Result<()> {
    Ok(())
}

#[cfg(test)]
#[path = "tests/scheduler.rs"]
mod tests;

#[cfg(test)]
#[path = "scheduler_tests.rs"]
mod scheduler_tests;

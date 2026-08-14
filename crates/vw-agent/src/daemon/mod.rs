//! 守护进程管理模块
//!
//! 本模块提供 VibeWindow 代理的守护进程运行时管理功能，负责：
//! - 启动和监督各个子系统组件（网关、通道、心跳、调度器）
//! - 处理优雅关闭和信号管理
//! - 维护守护进程状态文件
//! - 组件故障时的自动重启和退避策略
//!
//! # 架构概述
//!
//! 守护进程采用监督者模式运行多个组件：
//! - **网关（Gateway）**: HTTP/WebSocket API 服务
//! - **通道（Channels）**: 实时通信通道（Telegram、Discord、Slack 等）
//! - **心跳（Heartbeat）**: 定期执行代理任务并报告状态
//! - **调度器（Scheduler）**: 基于 cron 的任务调度
//!
//! # 关闭流程
//!
//! 守护进程支持优雅关闭：
//! 1. 接收到 SIGINT（Ctrl+C）或 SIGTERM 信号
//! 2. 标记守护进程为错误状态
//! 3. 给予各组件宽限期完成清理
//! 4. 超时后强制中止未完成的任务

use crate::agent_scheduler::domain::AgentSchedulerContext;
use crate::agent_scheduler::runner::matrix::{
    ConfiguredMatrixWorkerComponent, MATRIX_WORKER_RUNTIME, configured_matrix_worker_from_stored,
    matrix_worker_desired_state_is_running,
};
use crate::agent_scheduler::store::AgentSchedulerStore;
use crate::agent_scheduler::store::workers::AgentWorkerFilter;
use crate::app::agent::config::Config;
use crate::app::agent::config::schema::ChannelsConfigExt;
use anyhow::Result;
use chrono::Utc;
use std::future::Future;
use std::io::ErrorKind;
use std::path::PathBuf;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

/// 状态文件刷新间隔（秒）
///
/// 守护进程每隔此间隔将当前状态快照写入磁盘，
/// 用于持久化组件健康状态和时间戳信息。
const STATUS_FLUSH_SECONDS: u64 = 5;

const GATEWAY_PROBE_TIMEOUT_MILLIS: u64 = 500;

/// 关闭信号类型枚举
///
/// 表示触发守护进程关闭的不同信号来源。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShutdownSignal {
    /// 用户按下 Ctrl+C 触发的中断信号（SIGINT）
    CtrlC,
    /// 系统发送的终止信号（SIGTERM），通常来自进程管理器
    SigTerm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExistingGatewayStatus {
    Missing,
    Compatible,
    Stale,
}

/// 获取关闭信号的人类可读原因描述
///
/// # 参数
///
/// - `signal`: 关闭信号类型
///
/// # 返回值
///
/// 返回描述关闭原因的静态字符串
fn shutdown_reason(signal: ShutdownSignal) -> &'static str {
    match signal {
        ShutdownSignal::CtrlC => "shutdown requested (SIGINT)",
        ShutdownSignal::SigTerm => "shutdown requested (SIGTERM)",
    }
}

/// 获取平台相关的关闭提示文本
///
/// 在 Unix 系统上提示支持 Ctrl+C 和 SIGTERM，
/// 在非 Unix 系统上仅提示 Ctrl+C。
///
/// # 返回值
///
/// 返回适合当前平台的关闭提示字符串
#[cfg(unix)]
fn shutdown_hint() -> &'static str {
    "Ctrl+C or SIGTERM to stop"
}

#[cfg(not(unix))]
fn shutdown_hint() -> &'static str {
    "Ctrl+C to stop"
}

/// 异步等待关闭信号
///
/// 根据操作系统平台监听相应的关闭信号：
/// - **Unix 系统**: 同时监听 SIGINT（Ctrl+C）和 SIGTERM
/// - **非 Unix 系统**: 仅监听 SIGINT（Ctrl+C）
///
/// # 返回值
///
/// 成功时返回接收到的关闭信号类型，失败时返回错误
///
/// # 错误
///
/// - 在 Unix 系统上，如果 SIGTERM 信号流意外关闭，返回错误
/// - 信号注册失败时返回错误
async fn wait_for_shutdown_signal() -> Result<ShutdownSignal> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm = signal(SignalKind::terminate())?;
        tokio::select! {
            // 监听 Ctrl+C 信号
            ctrl_c = tokio::signal::ctrl_c() => {
                ctrl_c?;
                Ok(ShutdownSignal::CtrlC)
            }
            // 监听 SIGTERM 信号
            sigterm_result = sigterm.recv() => match sigterm_result {
                Some(()) => Ok(ShutdownSignal::SigTerm),
                None => anyhow::bail!("SIGTERM signal stream unexpectedly closed"),
            },
        }
    }
    #[cfg(not(unix))]
    {
        // 非 Unix 系统仅支持 Ctrl+C
        tokio::signal::ctrl_c().await?;
        Ok(ShutdownSignal::CtrlC)
    }
}

/// 启动守护进程主运行循环
///
/// 这是守护进程的主入口点，负责：
/// 1. 初始化健康状态和心跳文件
/// 2. 启动状态写入器
/// 3. 启动并监督各个组件（网关、通道、心跳、调度器）
/// 4. 等待关闭信号
/// 5. 优雅关闭所有组件
///
/// # 参数
///
/// - `config`: 代理配置对象
/// - `host`: 网关监听的主机地址
/// - `port`: 网关监听的端口号
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误
///
/// # 示例
///
/// ```no_run
/// use vibe_agent::app::agent::config::Config;
/// use vibe_agent::app::agent::daemon;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let config = Config::load("config.toml")?;
///     daemon::run(config, "0.0.0.0".to_string(), 8080).await
/// }
/// ```
pub async fn run(config: Config, host: String, port: u16) -> Result<()> {
    // 从配置中获取退避参数，确保最小值为 1 秒
    let initial_backoff = config.reliability.channel_initial_backoff_secs.max(1);
    let max_backoff = config.reliability.channel_max_backoff_secs.max(initial_backoff);

    // 标记守护进程组件为健康状态
    crate::app::agent::health::mark_component_ok("daemon");

    // 如果启用了心跳功能，确保心跳文件存在
    if config.heartbeat.enabled {
        let _ = crate::app::agent::heartbeat::engine::HeartbeatEngine::ensure_heartbeat_file(
            &config.workspace_dir,
        )
        .await;
    }

    // 收集所有组件的任务句柄
    let mut handles: Vec<JoinHandle<()>> = vec![spawn_state_writer(config.clone())];
    let existing_gateway_status = existing_gateway_status(&host, port).await;
    let gateway_already_running = existing_gateway_status != ExistingGatewayStatus::Missing;

    // 启动网关组件监督器
    if existing_gateway_status == ExistingGatewayStatus::Compatible {
        crate::app::agent::health::mark_component_ok("gateway");
        tracing::warn!(
            host = %host,
            port,
            "Gateway already running on requested address; daemon will not bind a duplicate gateway"
        );
    } else if existing_gateway_status == ExistingGatewayStatus::Stale {
        crate::app::agent::health::mark_component_error(
            "gateway",
            "existing gateway is missing cron history API",
        );
        tracing::error!(
            host = %host,
            port,
            "Existing gateway is healthy but missing cron history API; stop the old gateway process and restart daemon to load the updated gateway"
        );
    } else {
        let gateway_cfg = config.clone();
        let gateway_host = host.clone();
        handles.push(spawn_component_supervisor(
            "gateway",
            initial_backoff,
            max_backoff,
            move || {
                let cfg = gateway_cfg.clone();
                let host = gateway_host.clone();
                async move { crate::app::agent::gateway::run_gateway(&host, port, cfg).await }
            },
        ));
    }

    // 启动通道组件监督器（如果配置了实时通道）
    {
        if has_supervised_channels(&config) {
            let channels_cfg = config.clone();
            handles.push(tokio::spawn(async move {
                match crate::app::agent::channels::start_channels(channels_cfg).await {
                    Ok(()) => {
                        crate::app::agent::health::mark_component_ok("channels");
                        tracing::info!("Optional channels component stopped");
                    }
                    Err(error) => {
                        crate::app::agent::health::mark_component_error(
                            "channels",
                            error.to_string(),
                        );
                        tracing::warn!(
                            "Optional channels component is unavailable and will remain disabled: {error}"
                        );
                    }
                }
            }));
        } else {
            // 没有配置实时通道，标记为健康状态并记录日志
            crate::app::agent::health::mark_component_ok("channels");
            tracing::info!("No real-time channels configured; channel supervisor disabled");
        }
    }

    // 启动心跳组件监督器（如果启用了心跳功能）
    if config.heartbeat.enabled {
        let heartbeat_cfg = config.clone();
        handles.push(spawn_component_supervisor(
            "heartbeat",
            initial_backoff,
            max_backoff,
            move || {
                let cfg = heartbeat_cfg.clone();
                async move { Box::pin(run_heartbeat_worker(cfg)).await }
            },
        ));
    }

    // 启动调度器组件监督器（如果启用了 cron）
    if config.cron.enabled {
        let scheduler_cfg = config.clone();
        handles.push(spawn_component_supervisor(
            "scheduler",
            initial_backoff,
            max_backoff,
            move || {
                let cfg = scheduler_cfg.clone();
                async move { crate::app::agent::cron::scheduler::run(cfg).await }
            },
        ));
    } else {
        // Cron 未启用，标记为健康状态并记录日志
        crate::app::agent::health::mark_component_ok("scheduler");
        tracing::info!("Cron disabled; scheduler supervisor not started");
    }

    let matrix_worker_components = configured_matrix_worker_components(&config).await?;
    if matrix_worker_components.is_empty() {
        crate::app::agent::health::mark_component_ok("matrix-workers");
        tracing::info!(
            "No desired Running Matrix workers configured; matrix worker supervisors disabled"
        );
    } else {
        for component in matrix_worker_components {
            let worker_cfg = config.clone();
            let component_name = matrix_worker_component_name(&component);
            handles.push(spawn_component_supervisor(
                component_name,
                initial_backoff,
                max_backoff,
                move || {
                    let cfg = worker_cfg.clone();
                    let project_path = component.project_path.clone();
                    let worker_id = component.worker_id.clone();
                    async move {
                        crate::agent_scheduler::runner::matrix::run_configured_matrix_worker(
                            cfg,
                            project_path,
                            worker_id,
                        )
                        .await
                    }
                },
            ));
        }
    }

    // 输出启动信息到控制台
    println!("💡 VibeWindow daemon started");
    if gateway_already_running {
        println!("   Gateway:  http://{host}:{port} (already running)");
    } else {
        println!("   Gateway:  http://{host}:{port}");
    }
    println!("   Components: gateway, channels, heartbeat, scheduler, matrix-workers");
    println!("   {}", shutdown_hint());

    // 等待关闭信号
    let signal = wait_for_shutdown_signal().await?;
    crate::app::agent::health::mark_component_error("daemon", shutdown_reason(signal));

    // 组件没有共享关闭令牌，收到进程级停止信号后应立即终止后台任务。
    let stopped = shutdown_handles(handles).await;
    if stopped > 0 {
        tracing::info!(stopped, "Stopped daemon background tasks");
    }

    Ok(())
}

async fn configured_matrix_worker_components(
    config: &Config,
) -> Result<Vec<ConfiguredMatrixWorkerComponent>> {
    if !config.channels_config.matrix_enabled {
        return Ok(Vec::new());
    }

    let store = AgentSchedulerStore::new();
    let context = matrix_worker_scheduler_context(config).await?;
    let workers = store.list_workers(
        &context,
        AgentWorkerFilter {
            runtime: Some(MATRIX_WORKER_RUNTIME.to_string()),
            ..AgentWorkerFilter::default()
        },
    )?;
    workers
        .into_iter()
        .filter(matrix_worker_desired_state_is_running)
        .map(|worker| configured_matrix_worker_from_stored(&config.workspace_dir, worker))
        .collect()
}

async fn matrix_worker_scheduler_context(config: &Config) -> Result<AgentSchedulerContext> {
    let tenant_scope = TenantScope::default();
    let project =
        crate::app::agent::project::from_directory(&tenant_scope, &config.workspace_dir).await?.0;
    let project_scope = ProjectScope::new(tenant_scope, ProjectId::from(project.id.as_str()))?;
    AgentSchedulerContext::new(
        AgentSchedulerScope::new(project_scope),
        config.workspace_dir.clone(),
    )
}

fn matrix_worker_component_name(component: &ConfiguredMatrixWorkerComponent) -> String {
    format!("matrix-worker:{}", component.worker_id.as_str())
}

async fn existing_gateway_status(host: &str, port: u16) -> ExistingGatewayStatus {
    let health_url = gateway_health_url(host, port);
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_millis(GATEWAY_PROBE_TIMEOUT_MILLIS))
        .build()
    else {
        return ExistingGatewayStatus::Missing;
    };
    let Ok(response) = client.get(health_url).send().await else {
        return ExistingGatewayStatus::Missing;
    };
    if !response.status().is_success() {
        return ExistingGatewayStatus::Missing;
    }
    let Ok(body) = response.json::<serde_json::Value>().await else {
        return ExistingGatewayStatus::Missing;
    };
    let is_vibewindow = body.get("status").and_then(serde_json::Value::as_str) == Some("ok")
        && body.get("runtime").is_some();
    if !is_vibewindow {
        return ExistingGatewayStatus::Missing;
    }

    let history_url = gateway_cron_history_probe_url(host, port);
    let Ok(response) = client.get(history_url).send().await else {
        return ExistingGatewayStatus::Stale;
    };
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        ExistingGatewayStatus::Stale
    } else {
        ExistingGatewayStatus::Compatible
    }
}

fn gateway_health_url(host: &str, port: u16) -> String {
    format!("{}/v1/health", gateway_base_url(host, port))
}

fn gateway_cron_history_probe_url(host: &str, port: u16) -> String {
    format!("{}/v1/cron/runs/__probe__", gateway_base_url(host, port))
}

fn gateway_base_url(host: &str, port: u16) -> String {
    let probe_host = match host.trim() {
        "" => "127.0.0.1",
        "0.0.0.0" => "127.0.0.1",
        "::" => "::1",
        other => other,
    };

    let host_part = if probe_host.contains(':') && !probe_host.starts_with('[') {
        format!("[{probe_host}]")
    } else {
        probe_host.to_string()
    };
    format!("http://{host_part}:{port}")
}

/// 停止所有后台任务句柄。
///
/// # 参数
///
/// - `handles`: 需要关闭的任务句柄列表
/// # 返回值
///
/// 返回被停止的任务数量。
async fn shutdown_handles(handles: Vec<JoinHandle<()>>) -> usize {
    let mut stopped = 0usize;
    for handle in &handles {
        if !handle.is_finished() {
            handle.abort();
            stopped += 1;
        }
    }

    for handle in handles {
        let _ = handle.await;
    }
    stopped
}

/// 获取守护进程状态文件的路径
///
/// 状态文件位于配置文件所在目录下的 `daemon_state.json` 文件中。
///
/// # 参数
///
/// - `config`: 代理配置对象
///
/// # 返回值
///
/// 返回状态文件的完整路径
///
/// # 路径解析规则
///
/// - 如果配置文件有父目录，状态文件位于该目录下
/// - 如果配置文件没有父目录（当前目录），状态文件位于当前目录
pub fn state_file_path(config: &Config) -> PathBuf {
    config
        .config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join("daemon_state.json")
}

/// 生成状态写入器任务
///
/// 创建一个后台任务，定期将守护进程状态快照写入磁盘文件。
/// 状态文件包含所有组件的健康状态和时间戳信息。
///
/// # 参数
///
/// - `config`: 代理配置对象
///
/// # 返回值
///
/// 返回状态写入器任务的句柄
///
/// # 写入内容
///
/// - 所有组件的健康状态快照
/// - 写入时间戳（ISO 8601 格式）
fn spawn_state_writer(config: Config) -> JoinHandle<()> {
    tokio::spawn(async move {
        let path = state_file_path(&config);

        // 确保状态文件的父目录存在
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        // 创建定时器，每隔 STATUS_FLUSH_SECONDS 秒触发一次
        let mut interval = tokio::time::interval(Duration::from_secs(STATUS_FLUSH_SECONDS));
        loop {
            interval.tick().await;

            // 获取健康状态快照并添加时间戳
            let mut json = crate::app::agent::health::snapshot_json();
            if let Some(obj) = json.as_object_mut() {
                obj.insert("written_at".into(), serde_json::json!(Utc::now().to_rfc3339()));
            }

            // 序列化并写入文件
            let data = serde_json::to_vec_pretty(&json).unwrap_or_else(|_| b"{}".to_vec());
            let _ = tokio::fs::write(&path, data).await;
        }
    })
}

/// 生成组件监督器任务
///
/// 创建一个监督器任务，负责启动、监控和重启指定的组件。
/// 当组件失败或意外退出时，使用指数退避策略进行重启。
///
/// # 参数
///
/// - `name`: 组件名称，用于健康状态跟踪和日志记录
/// - `initial_backoff_secs`: 初始退避时间（秒）
/// - `max_backoff_secs`: 最大退避时间（秒）
/// - `run_component`: 组件运行函数，返回 `Future<Output = Result<()>>`
///
/// # 返回值
///
/// 返回监督器任务的句柄
///
/// # 退避策略
///
/// - 初始退避时间至少为 1 秒
/// - 每次失败后，退避时间翻倍，直到达到最大值
/// - 组件正常退出时，重置退避时间为初始值
/// - 退避时间在睡眠后更新，首次错误使用初始退避时间
fn spawn_component_supervisor<F, Fut>(
    name: impl Into<String>,
    initial_backoff_secs: u64,
    max_backoff_secs: u64,
    mut run_component: F,
) -> JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    let name = name.into();
    tokio::spawn(async move {
        // 确保退避时间参数有效
        let mut backoff = initial_backoff_secs.max(1);
        let max_backoff = max_backoff_secs.max(backoff);

        loop {
            // 标记组件为健康状态
            crate::app::agent::health::mark_component_ok(&name);
            match run_component().await {
                Ok(()) => {
                    // 组件正常退出（但守护进程期望它持续运行）
                    crate::app::agent::health::mark_component_error(
                        &name,
                        "component exited unexpectedly",
                    );
                    tracing::warn!("Daemon component '{name}' exited unexpectedly");
                    // 正常退出，重置退避时间
                    backoff = initial_backoff_secs.max(1);
                }
                Err(e) => {
                    // 组件失败
                    crate::app::agent::health::mark_component_error(&name, e.to_string());
                    if is_non_retryable_component_error(&name, &e) {
                        tracing::error!(
                            "Daemon component '{name}' failed with a non-retryable error: {e}"
                        );
                        break;
                    }
                    tracing::error!("Daemon component '{name}' failed: {e}");
                }
            }

            // 记录组件重启次数并等待退避时间
            crate::app::agent::health::bump_component_restart(&name);
            tokio::time::sleep(Duration::from_secs(backoff)).await;
            // 在睡眠后更新退避时间，这样首次错误使用初始退避时间
            backoff = backoff.saturating_mul(2).min(max_backoff);
        }
    })
}

fn is_non_retryable_component_error(name: &str, error: &anyhow::Error) -> bool {
    name == "gateway"
        && error.chain().any(|cause| {
            cause
                .downcast_ref::<std::io::Error>()
                .is_some_and(|io_error| io_error.kind() == ErrorKind::AddrInUse)
        })
}

/// 运行心跳工作器
///
/// 定期执行心跳任务，收集代理的任务列表并运行代理处理这些任务。
/// 任务执行结果可以通过配置的通道（Telegram、Discord、Slack 等）发送。
///
/// # 参数
///
/// - `config`: 代理配置对象
///
/// # 返回值
///
/// 成功时循环运行直到被外部中止，失败时返回错误
///
/// # 工作流程
///
/// 1. 创建观察者引擎用于心跳任务收集
/// 2. 定期触发（间隔由配置决定，至少 5 分钟）
/// 3. 收集心跳任务文件中的任务
/// 4. 对每个任务调用代理执行
/// 5. 将执行结果通过配置的通道发送
async fn run_heartbeat_worker(config: Config) -> Result<()> {
    // 创建观察者用于心跳任务收集
    let observer: std::sync::Arc<dyn crate::app::agent::observability::Observer> =
        std::sync::Arc::from(crate::app::agent::observability::create_observer(
            &config.observability,
        ));
    let engine = crate::app::agent::heartbeat::engine::HeartbeatEngine::new(
        config.heartbeat.clone(),
        config.workspace_dir.clone(),
        observer,
    );

    // 获取心跳投递目标配置
    let delivery = heartbeat_delivery_target(&config)?;

    // 设置心跳间隔（至少 5 分钟）
    let interval_mins = config.heartbeat.interval_minutes.max(5);
    let mut interval = tokio::time::interval(Duration::from_secs(u64::from(interval_mins) * 60));

    loop {
        interval.tick().await;

        // 收集心跳任务文件中的任务
        let file_tasks = engine.collect_tasks().await?;
        let tasks = heartbeat_tasks_for_tick(file_tasks, config.heartbeat.message.as_deref());
        if tasks.is_empty() {
            continue;
        }

        // 执行每个任务
        for task in tasks {
            let prompt = format!("[Heartbeat Task] {task}");
            let temp = config.default_temperature;
            match crate::app::agent::agent::run(config.clone(), Some(prompt), None, None, temp)
                .await
            {
                Ok(output) => {
                    // 任务执行成功
                    crate::app::agent::health::mark_component_ok("heartbeat");
                    let announcement = if output.trim().is_empty() {
                        "heartbeat task executed".to_string()
                    } else {
                        output
                    };

                    // 如果配置了投递目标，发送执行结果
                    if let Some((channel, target)) = &delivery {
                        if let Err(e) = crate::app::agent::cron::scheduler::deliver_announcement(
                            &config,
                            channel,
                            target,
                            &announcement,
                        )
                        .await
                        {
                            crate::app::agent::health::mark_component_error(
                                "heartbeat",
                                format!("delivery failed: {e}"),
                            );
                            tracing::warn!("Heartbeat delivery failed: {e}");
                        }
                    }
                }
                Err(e) => {
                    // 任务执行失败
                    crate::app::agent::health::mark_component_error("heartbeat", e.to_string());
                    tracing::warn!("Heartbeat task failed: {e}");
                }
            }
        }
    }
}

/// 为当前心跳周期准备任务列表
///
/// 从文件任务列表或配置的回退消息中生成要执行的任务。
/// 优先使用文件中的任务，如果文件为空则使用配置的消息。
///
/// # 参数
///
/// - `file_tasks`: 从心跳任务文件中收集的任务列表
/// - `fallback_message`: 配置中的回退消息（当文件任务为空时使用）
///
/// # 返回值
///
/// 返回要执行的任务列表，可能为空
fn heartbeat_tasks_for_tick(
    file_tasks: Vec<String>,
    fallback_message: Option<&str>,
) -> Vec<String> {
    // 优先使用文件中的任务
    if !file_tasks.is_empty() {
        return file_tasks;
    }

    // 文件为空时使用回退消息
    fallback_message
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(|message| vec![message.to_string()])
        .unwrap_or_default()
}

/// 获取心跳投递目标配置
///
/// 从配置中提取心跳结果的投递目标（通道和接收者），
/// 并验证配置的完整性和有效性。
///
/// # 参数
///
/// - `config`: 代理配置对象
///
/// # 返回值
///
/// 成功时返回 `Some((channel, target))` 元组，如果未配置则返回 `None`
///
/// # 错误
///
/// - 仅配置了通道或接收者中的一个（必须同时配置或都不配置）
/// - 配置的通道在 `channels_config` 中未启用
/// - 配置了不支持的通道类型
fn heartbeat_delivery_target(config: &Config) -> Result<Option<(String, String)>> {
    // 提取并清理配置值
    let channel =
        config.heartbeat.target.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let target = config.heartbeat.to.as_deref().map(str::trim).filter(|value| !value.is_empty());

    match (channel, target) {
        (None, None) => Ok(None),
        (Some(_), None) => anyhow::bail!("heartbeat.to is required when heartbeat.target is set"),
        (None, Some(_)) => anyhow::bail!("heartbeat.target is required when heartbeat.to is set"),
        (Some(channel), Some(target)) => {
            // 验证通道配置是否有效
            validate_heartbeat_channel_config(config, channel)?;
            Ok(Some((channel.to_string(), target.to_string())))
        }
    }
}

/// 验证心跳通道配置
///
/// 检查配置的心跳通道在 `channels_config` 中是否已正确配置。
///
/// # 参数
///
/// - `config`: 代理配置对象
/// - `channel`: 通道名称（如 "telegram"、"discord" 等）
///
/// # 返回值
///
/// 验证通过时返回 `Ok(())`，否则返回错误
///
/// # 支持的通道
///
/// - telegram
/// - discord
/// - slack
/// - mattermost
fn validate_heartbeat_channel_config(config: &Config, channel: &str) -> Result<()> {
    match channel.to_ascii_lowercase().as_str() {
        "telegram" => {
            if config.channels_config.telegram.is_none() {
                anyhow::bail!(
                    "heartbeat.target is set to telegram but channels_config.telegram is not configured"
                );
            }
        }
        "discord" => {
            if config.channels_config.discord.is_none() {
                anyhow::bail!(
                    "heartbeat.target is set to discord but channels_config.discord is not configured"
                );
            }
        }
        "slack" => {
            if config.channels_config.slack.is_none() {
                anyhow::bail!(
                    "heartbeat.target is set to slack but channels_config.slack is not configured"
                );
            }
        }
        "mattermost" => {
            if config.channels_config.mattermost.is_none() {
                anyhow::bail!(
                    "heartbeat.target is set to mattermost but channels_config.mattermost is not configured"
                );
            }
        }
        other => anyhow::bail!("unsupported heartbeat.target channel: {other}"),
    }

    Ok(())
}

/// 检查是否配置了需要监督的实时通道
///
/// 判断是否存在任何非 webhook 的通道已启用，
/// 用于决定是否启动通道监督器。
///
/// # 参数
///
/// - `config`: 代理配置对象
///
/// # 返回值
///
/// 如果存在至少一个启用的非 webhook 通道，返回 `true`，否则返回 `false`
fn has_supervised_channels(config: &Config) -> bool {
    config.channels_config.channels_except_webhook().iter().any(|(_, ok)| *ok)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

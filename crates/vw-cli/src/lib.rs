//! # VibeWindow 代理 CLI 主入口模块
//!
//! 本模块是 `vibe-agent` 命令行工具的核心入口点，提供以下主要功能：
//!
//! - **代理运行**：启动 AI 代理进行任务执行和代码操作
//! - **网关服务**：启动 HTTP/WebSocket 网关提供 API 访问
//! - **守护进程**：作为后台服务运行
//! - **任务管理**：创建、读取和管理项目任务
//! - **配置管理**：查看和管理系统配置
//! - **安全控制**：紧急停止（E-stop）和安全策略管理
//! - **通道管理**：管理多平台消息通道（Telegram、Slack、Discord 等）
//! - **诊断工具**：系统诊断和健康检查
//!
//! ## 使用示例
//!
//! ```bash
//! # 启动交互式代理
//! vibe-agent agent
//!
//! # 启动网关服务
//! vibe-agent gateway --port 8080
//!
//! # 创建新任务
//! vibe-agent task create --project-dir /path/to/project --prompt "实现新功能"
//! ```

//!
//! ## 架构说明
//!
//! 本模块遵循 trait + 工厂架构，通过以下方式扩展功能：
//! - 在 `cli` 模块中定义命令
//! - 在 `handlers` 模块中实现命令处理逻辑
//! - 通过重新导出保持命令枚举的单一真实来源

#![warn(clippy::all, clippy::pedantic)]
// #![forbid(unsafe_code)]
#![allow(
    clippy::assigning_clones,
    clippy::bool_to_int_with_if,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::cast_possible_wrap,
    clippy::doc_markdown,
    clippy::field_reassign_with_default,
    clippy::float_cmp,
    clippy::implicit_clone,
    clippy::items_after_statements,
    clippy::map_unwrap_or,
    clippy::manual_let_else,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::large_futures,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::needless_raw_string_hashes,
    clippy::redundant_closure_for_method_calls,
    clippy::result_large_err,
    clippy::similar_names,
    clippy::single_match_else,
    clippy::struct_field_names,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::unused_self,
    clippy::cast_precision_loss,
    clippy::unnecessary_cast,
    clippy::unnecessary_lazy_evaluations,
    clippy::unnecessary_literal_bound,
    clippy::unnecessary_map_or,
    clippy::unnecessary_wraps,
    dead_code
)]

use anyhow::{Context, Result, bail};
use clap::Parser;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use vw_agent::agent_scheduler::domain::TaskId;
use vw_agent::agent_scheduler::store::AgentSchedulerStore;
use vw_agent::agent_scheduler::task_board::{
    create_native_task, list_native_tasks, load_native_task,
};
use vw_agent::channels::ChannelCommands;
use vw_agent::cron;
use vw_agent::integrations;
use vw_agent::provider::provider;
use vw_agent::{
    channels, config, daemon, doctor, gateway, initialize_runtime_build_id, memory, observability,
    plugins, security, service, skills,
};
use vw_shared::task::{SubTask, Task, TaskExecutorBackend, TaskSplitMode, TaskStatus};

use config::Config;
use config::schema::ChannelsConfigExt;
use config::schema::ConfigExt;

mod acp;
#[path = "cli.rs"]
mod cli;
#[cfg(test)]
#[path = "cli_tests.rs"]
mod cli_tests;
mod handlers;

pub(crate) mod session {
    pub(crate) use vw_shared::session::ui_types;
}

pub(crate) mod app {
    pub(crate) mod agent {
        pub(crate) use vw_agent::{
            approval, channels, config, id, memory, observability, project, providers, runtime,
            security, shell, skills, tools,
        };

        pub(crate) mod session {
            pub(crate) use vw_agent::session::{processor, session, title};
        }

        #[allow(clippy::module_inception)]
        pub(crate) mod agent {
            pub(crate) mod loop_ {
                pub(crate) use vw_agent::agent::loop_::{context, core, instructions};
            }
        }
    }
}

use cli::{Cli, Commands, ConfigCommands, DoctorCommands, TaskCommands};

/// 重新导出命令枚举，确保二进制模块可以使用 `crate::<CommandEnum>`
/// 同时保持命令定义的单一真实来源。
/// 解析并验证温度参数
///
/// 温度参数用于控制 AI 模型输出的随机性，取值范围为 0.0 到 2.0：
/// - 0.0：最确定性，输出最一致
/// - 1.0：平衡的随机性
/// - 2.0：最高随机性，输出最多样化
///
/// # 参数
///
/// * `s` - 温度值的字符串表示
///
/// # 返回值
///
/// - `Ok(f64)` - 解析成功的温度值（0.0 到 2.0 之间）
/// - `Err(String)` - 解析失败或值超出范围时的错误信息
///
/// # 示例
///
/// ```ignore
/// let temp = parse_temperature("0.7").unwrap(); // 返回 0.7
/// let err = parse_temperature("3.0"); // 返回错误，超出范围
/// ```
fn parse_temperature(s: &str) -> std::result::Result<f64, String> {
    let t: f64 = s.parse().map_err(|e| format!("{e}"))?;
    if !(0.0..=2.0).contains(&t) {
        return Err("temperature must be between 0.0 and 2.0".to_string());
    }
    Ok(t)
}

/// 解析并验证项目目录路径
///
/// 将用户提供的项目目录路径转换为规范的绝对路径，并进行以下验证：
/// 1. 路径不能为空（去除空白后）
/// 2. 路径必须存在且可访问
/// 3. 路径必须指向一个目录而非文件
///
/// # 参数
///
/// * `project_dir` - 用户提供的项目目录路径字符串
///
/// # 返回值
///
/// - `Ok(String)` - 规范化后的绝对路径字符串
/// - `Err(anyhow::Error)` - 路径为空、不存在或不是目录时的错误
///
/// # 错误
///
/// - 路径为空或仅包含空白字符
/// - 路径无法解析（权限不足或不存在）
/// - 路径指向的不是目录
///
/// # 示例
///
/// ```ignore
/// let path = resolve_project_dir("./my-project").unwrap();
/// // 返回类似 "/Users/username/projects/my-project" 的绝对路径
/// ```
fn resolve_project_dir(project_dir: &str) -> Result<String> {
    let trimmed = project_dir.trim();
    if trimmed.is_empty() {
        bail!("--project-dir cannot be empty");
    }
    let path = std::fs::canonicalize(trimmed)
        .with_context(|| format!("Failed to resolve project directory: {trimmed}"))?;
    if !path.is_dir() {
        bail!("project directory is not a folder: {}", path.display());
    }
    Ok(path.to_string_lossy().to_string())
}

/// 提取文本内容中的第一个非空行
///
/// 遍历文本的所有行，跳过空白行，返回第一个包含内容的行。
/// 如果所有行都为空，则返回空字符串。
///
/// # 参数
///
/// * `content` - 要处理的文本内容
///
/// # 返回值
///
/// 第一个非空行的字符串，如果不存在则返回空字符串
///
/// # 示例
///
/// ```ignore
/// let text = "\n  \n第一行内容\n第二行内容";
/// let first = first_non_empty_line(text); // 返回 "第一行内容"
/// ```
fn first_non_empty_line(content: &str) -> String {
    content.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default().to_string()
}

/// 以 JSON 格式打印单个任务信息
///
/// 将任务对象序列化为格式化的 JSON 字符串并输出到标准输出。
/// 使用 `serde_json::to_string_pretty` 进行美化格式化，便于人类阅读。
///
/// # 参数
///
/// * `task` - 要打印的任务对象引用
///
/// # 返回值
///
/// - `Ok(())` - 打印成功
/// - `Err(anyhow::Error)` - JSON 序列化失败
///
/// # 示例
///
/// ```ignore
/// let task = Task::new(1);
/// print_task_json(&task)?; // 输出格式化的 JSON
/// ```
fn print_task_json(task: &Task) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(task)?);
    Ok(())
}

/// 以 JSON 格式打印任务列表信息
///
/// 将任务对象数组序列化为格式化的 JSON 字符串并输出到标准输出。
/// 使用 `serde_json::to_string_pretty` 进行美化格式化，便于人类阅读。
///
/// # 参数
///
/// * `tasks` - 要打印的任务对象数组切片
///
/// # 返回值
///
/// - `Ok(())` - 打印成功
/// - `Err(anyhow::Error)` - JSON 序列化失败
///
/// # 示例
///
/// ```ignore
/// let tasks = vec![Task::new(1), Task::new(2)];
/// print_tasks_json(&tasks)?; // 输出格式化的 JSON 数组
/// ```
fn print_tasks_json(tasks: &[Task]) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(tasks)?);
    Ok(())
}

/// CLI 主入口函数
///
/// 这是 `vibe-agent` 命令行工具的异步主函数，负责：
/// 1. 初始化 TLS 加密提供者
/// 2. 解析命令行参数
/// 3. 设置配置目录（如果指定）
/// 4. 处理 shell 补全命令（不初始化日志）
/// 5. 初始化日志系统（根据模式调整输出目标）
/// 6. 加载并应用配置
/// 7. 初始化可观测性和安全模块
/// 8. 路由到具体命令处理器
///
/// # 命令路由
///
/// 支持的命令包括：
/// - `agent`: 启动 AI 代理进行任务执行
/// - `gateway`: 启动 HTTP/WebSocket 网关服务
/// - `daemon`: 启动后台守护进程
/// - `status`: 显示系统状态信息
/// - `estop`: 紧急停止控制
/// - `security`: 安全策略管理
/// - `cron`: 定时任务管理
/// - `providers`: 列出支持的 AI 提供者
/// - `service`: 系统服务管理
/// - `doctor`: 系统诊断工具
/// - `channel`: 消息通道管理
/// - `integrations`: 集成管理
/// - `skills`: 技能管理
/// - `task`: 任务管理
/// - `memory`: 记忆系统管理
/// - `config`: 配置管理
///
/// # 返回值
///
/// - `Ok(())` - 命令执行成功
/// - `Err(anyhow::Error)` - 命令执行失败
///
/// # 错误处理
///
/// 函数会在以下情况下返回错误：
/// - 配置加载失败
/// - 命令参数无效
/// - 命令执行过程中的任何错误
#[allow(clippy::too_many_lines)]
pub async fn run() -> Result<()> {
    // 安装 Rustls TLS 的默认加密提供者
    // 这可以防止当同时有多个加密库可用时出现的错误：
    // "could not automatically determine the process-level CryptoProvider"
    if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
        eprintln!("Warning: Failed to install default crypto provider: {e:?}");
    }

    // 解析命令行参数
    let cli = Cli::parse();
    if matches!(&cli.command, Commands::Gateway { .. } | Commands::Daemon { .. }) {
        initialize_runtime_build_id().map_err(anyhow::Error::msg)?;
    }

    // 设置自定义配置目录（如果通过 --config-dir 指定）
    if let Some(config_dir) = &cli.config_dir {
        if config_dir.trim().is_empty() {
            bail!("--config-dir cannot be empty");
        }
        // 注意：这里使用了 unsafe 代码块来设置环境变量
        // 在多线程环境下可能有竞争条件，但在这里是安全的
        unsafe {
            std::env::set_var("VIBEWINDOW_CONFIG_DIR", config_dir);
        }
    }

    // 处理 shell 补全命令
    // 补全命令必须保持仅输出到标准输出，不加载配置或初始化日志
    // 这可以避免警告/日志行破坏被 source 的补全脚本
    if let Commands::Completions { shell } = &cli.command {
        let mut stdout = std::io::stdout().lock();
        cli::write_shell_completion(*shell, &mut stdout)?;
        return Ok(());
    }

    // 检测是否为交互式代理模式（全屏 TUI 模式）
    let acp_mode = matches!(&cli.command, Commands::Acp);
    let interactive_agent_mode = matches!(&cli.command, Commands::Agent { message: None, .. });

    init_tracing(interactive_agent_mode, acp_mode);

    // 加载配置（所有其他命令都需要先加载配置）
    let mut config = Box::pin(Config::load_or_init()).await?;
    config.apply_env_overrides();

    if acp_mode {
        return acp::run_stdio().await;
    }

    // 从配置初始化运行时追踪
    observability::runtime_trace::init_from_config(&config.observability, &config.workspace_dir);

    // 初始化 OTP（一次性密码）安全验证，如果已启用
    if config.security.otp.enabled {
        let config_dir =
            config.config_path.parent().context("Config path must have a parent directory")?;
        let store = security::SecretStore::new(config_dir, config.secrets.encrypt);
        let (_validator, enrollment_uri) =
            security::OtpValidator::from_config(&config.security.otp, config_dir, &store)?;
        // 如果是新初始化的 OTP，显示注册 URI
        if let Some(uri) = enrollment_uri {
            println!("Initialized OTP secret for ShiJu.");
            println!("Enrollment URI: {uri}");
        }
    }

    // 根据命令类型路由到相应的处理器
    match cli.command {
        /* Commands::Onboard { .. } | */ Commands::Completions { .. } => unreachable!(),
        Commands::Acp => unreachable!("ACP mode is routed before interactive initialization"),

        // 处理根级远程 Gateway API 命令。
        Commands::Api(api_commands) => handlers::api::run(api_commands.into_args()).await,

        // 处理 Agent 命令：启动 AI 代理
        Commands::Agent {
            message,
            provider,
            model,
            temperature,
            peripheral,
            autonomy_level,
            max_actions_per_hour,
            max_iterations,
            max_history_messages,
            compact_context,
            memory_backend,
        } => {
            // 应用命令行参数覆盖配置中的自主级别
            if let Some(level) = autonomy_level {
                config.autonomy.level = level;
            }
            // 应用每小时最大操作数限制
            if let Some(n) = max_actions_per_hour {
                config.autonomy.max_actions_per_hour = n;
            }
            let main_agent = vw_config_types::agent::ensure_main_agent_config(&mut config.agents);
            // 应用工具迭代次数限制
            if let Some(n) = max_iterations {
                main_agent.max_iterations = n;
            }
            // 应用历史消息数量限制
            if let Some(n) = max_history_messages {
                main_agent.max_history_messages = n;
            }
            // 启用上下文压缩模式
            if compact_context {
                main_agent.compact_context = true;
            }
            // 应用记忆后端配置
            if let Some(ref backend) = memory_backend {
                config.memory.backend = backend.clone();
            }
            // 启动代理运行
            Box::pin(crate::cli::run(
                config,
                message,
                provider,
                model,
                temperature,
                peripheral,
                interactive_agent_mode,
            ))
            .await
            .map(|_| ())
        }

        // 处理 Gateway 命令：启动 HTTP/WebSocket 网关服务
        Commands::Gateway { port, host } => {
            // 确定监听端口（使用参数或配置中的值）
            let port = port.unwrap_or(config.gateway.port);
            // 确定监听主机（使用参数或配置中的值）
            let host = host.unwrap_or_else(|| config.gateway.host.clone());
            // 显示启动信息
            if port == 0 {
                info!(" Starting ShiJu Gateway on {host} (random port)");
            } else {
                info!(" Starting ShiJu Gateway on {host}:{port}");
            }
            // 启动网关服务
            gateway::run_gateway(&host, port, config).await
        }

        // 处理 Daemon 命令：启动后台守护进程
        Commands::Daemon { port, host } => {
            // 确定监听端口和主机
            let port = port.unwrap_or(config.gateway.port);
            let host = host.unwrap_or_else(|| config.gateway.host.clone());
            // 显示启动信息
            if port == 0 {
                info!("💡 Starting ShiJu Daemon on {host} (random port)");
            } else {
                info!("💡 Starting ShiJu Daemon on {host}:{port}");
            }
            // 启动守护进程
            daemon::run(config, host, port).await
        }

        // 处理 Status 命令：显示系统状态
        Commands::Status => {
            println!("ShiJu Status");
            println!();
            println!("Version:     {}", env!("CARGO_PKG_VERSION"));
            println!("Workspace:   {}", config.workspace_dir.display());
            println!("Config:      {}", config.config_path.display());
            println!();
            println!(
                "🤖 Provider:      {}",
                config.default_provider.as_deref().unwrap_or("openrouter")
            );
            println!(
                "   Model:         {}",
                config.default_model.as_deref().unwrap_or("(default)")
            );
            println!("📊 Observability:  {}", config.observability.backend);
            println!(
                "🧾 Trace storage:  {} ({})",
                config.observability.runtime_trace_mode, config.observability.runtime_trace_path
            );
            println!("🛡️  Autonomy:      {:?}", config.autonomy.level);
            println!("⚙️  Runtime:       {}", config.runtime.kind);
            // 获取有效的记忆后端名称
            let effective_memory_backend = memory::effective_memory_backend_name(
                &config.memory.backend,
                Some(&config.storage.provider.config),
            );
            println!(
                "💓 Heartbeat:      {}",
                if config.heartbeat.enabled {
                    format!("every {}min", config.heartbeat.interval_minutes)
                } else {
                    "disabled".into()
                }
            );
            println!(
                "Memory:         {} (auto-save: {})",
                effective_memory_backend,
                if config.memory.auto_save { "on" } else { "off" }
            );

            println!();
            println!("Security:");
            println!("  Workspace only:    {}", config.autonomy.workspace_only);
            println!(
                "  Allowed roots:     {}",
                if config.autonomy.allowed_roots.is_empty() {
                    "(none)".to_string()
                } else {
                    config.autonomy.allowed_roots.join(", ")
                }
            );
            println!("  Allowed commands:  {}", config.autonomy.allowed_commands.join(", "));
            println!("  Max actions/hour:  {}", config.autonomy.max_actions_per_hour);
            println!(
                "  Max cost/day:      ${:.2}",
                f64::from(config.autonomy.max_cost_per_day_cents) / 100.0
            );
            println!("  OTP enabled:       {}", config.security.otp.enabled);
            println!("  E-stop enabled:    {}", config.security.estop.enabled);
            println!();
            println!("Channels:");
            println!("  CLI:      ✅ always");
            // 显示各个通道的配置状态
            for (channel, configured) in config.channels_config.channels() {
                println!(
                    "  {:9} {}",
                    channel.name(),
                    if configured { "✅ configured" } else { "❌ not configured" }
                );
            }

            Ok(())
        }

        /*
        Commands::Update { check, force } => {
            update::self_update(force, check).await?;
            Ok(())
        }
        */
        // 处理 E-stop 命令：紧急停止控制
        Commands::Estop { estop_command, level, domains, tools } => {
            handlers::estop::handle_estop_command(&config, estop_command, level, domains, tools)
        }

        // 处理 Security 命令：安全策略管理
        Commands::Security { security_command } => {
            handlers::security::handle_security_command(&config, security_command).await
        }

        // 处理 Cron 命令：定时任务管理
        Commands::Cron { cron_command } => cron::handle_command(cron_command, &config),

        // 处理 Providers 命令：列出支持的 AI 提供者
        Commands::Providers => {
            // 获取所有可用的提供者列表
            let provider_map = provider::list().await;
            let mut providers = provider_map.values().collect::<Vec<_>>();
            // 按提供者 ID 排序
            providers.sort_by(|a, b| a.id.cmp(&b.id));
            // 确定当前活动的提供者
            let current = config
                .default_provider
                .as_deref()
                .unwrap_or("openrouter")
                .trim()
                .to_ascii_lowercase();
            println!("Supported providers ({} total):\n", providers.len());
            println!("  ID (use in config)  DESCRIPTION");
            println!("  ─────────────────── ───────────");
            // 显示每个提供者的信息，标记当前活动的提供者
            for p in &providers {
                let is_active = p.id.eq_ignore_ascii_case(&current);
                let marker = if is_active { " (active)" } else { "" };
                println!("  {:<19} {}{}", p.id, p.name, marker);
            }
            // 显示自定义端点选项
            println!("\n  custom:<URL>   Any OpenAI-compatible endpoint");
            println!("  anthropic-custom:<URL>  Any Anthropic-compatible endpoint");
            Ok(())
        }

        // 处理 Service 命令：系统服务管理
        Commands::Service { service_command, service_init } => {
            // 解析初始化系统类型
            let init_system = service_init.parse()?;
            service::handle_command(&service_command, &config, init_system)
        }

        // 处理 Server 命令：service 的兼容别名
        Commands::Server { server_command, service_init } => {
            let init_system = service_init.parse()?;
            service::handle_command(&server_command, &config, init_system)
        }

        // 处理 Doctor 命令：系统诊断工具
        Commands::Doctor { doctor_command } => match doctor_command {
            // 检查可用的 AI 模型
            Some(DoctorCommands::Models { provider, use_cache }) => {
                doctor::run_models(&config, provider.as_deref(), use_cache).await
            }
            // 查看追踪记录
            Some(DoctorCommands::Traces { id, event, contains, limit }) => doctor::run_traces(
                &config,
                id.as_deref(),
                event.as_deref(),
                contains.as_deref(),
                limit,
            ),
            // 运行完整的系统诊断
            None => doctor::run(&config),
        },

        // 处理 Channel 命令：消息通道管理
        Commands::Channel { channel_command } => match channel_command {
            // 启动所有配置的通道
            ChannelCommands::Start => Box::pin(channels::start_channels(config)).await,
            // 诊断通道配置问题
            ChannelCommands::Doctor => channels::doctor_channels(config).await,
            // 处理其他通道命令
            other => channels::handle_command(other, &config).await,
        },

        // 处理 Integrations 命令：集成管理
        Commands::Integrations { integration_command } => {
            integrations::handle_command(integration_command, &config)
        }

        // 处理 Skills 命令：技能管理
        Commands::Skills { skill_command } => skills::handle_command(skill_command, &config),

        // 处理 Plugins 命令：插件管理
        Commands::Plugins { plugin_command } => plugins::handle_command(plugin_command, &config),

        // 处理 Task 命令：任务管理
        Commands::Task { project_dir, task_command } => {
            // 解析项目目录路径
            let project_path = resolve_project_dir(&project_dir)?;
            match task_command {
                // 创建新任务
                TaskCommands::Create {
                    priority,
                    prompt,
                    description,
                    assignee,
                    model,
                    executor,
                    task_split,
                    subtasks,
                } => {
                    // 清理和准备任务内容
                    let prompt = prompt.map(|value| value.trim().to_string());
                    let description = description.map(|value| value.trim().to_string());
                    let cleaned_subtasks = subtasks
                        .into_iter()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>();

                    // 确定任务的种子内容（从 prompt、description 或 subtasks 中获取第一个非空行）
                    let mut task_seed = first_non_empty_line(prompt.as_deref().unwrap_or_default());
                    if task_seed.is_empty() {
                        task_seed =
                            first_non_empty_line(description.as_deref().unwrap_or_default());
                    }
                    if task_seed.is_empty() {
                        task_seed = cleaned_subtasks.first().cloned().unwrap_or_default();
                    }
                    // 确保至少有一种内容来源
                    if task_seed.is_empty() {
                        bail!(
                            "task content is empty; provide at least one of --prompt, --description, or --subtask"
                        );
                    }

                    // 创建任务对象并设置各个字段
                    let mut task = Task::new(priority);

                    if let Some(description) = description {
                        task.description = description;
                    }
                    if let Some(assignee) = assignee {
                        let assignee = assignee.trim();
                        if !assignee.is_empty() {
                            task.assignee = assignee.to_string();
                        }
                    }
                    if let Some(model) = model {
                        let model = model.trim();
                        if !model.is_empty() {
                            task.model = model.to_string();
                        }
                    }
                    // 解析并设置执行器后端
                    if let Some(executor) = executor {
                        let parsed = TaskExecutorBackend::from_id(executor.trim())
                            .with_context(|| {
                                format!(
                                    "Invalid --executor value: {} (supported: internal, opencode, claude, codex)",
                                    executor
                                )
                            })?;
                        task.executor = parsed;
                    }
                    if let Some(task_split) = task_split {
                        task.task_split_mode = match task_split {
                            cli::TaskSplitCliMode::FollowSystem => TaskSplitMode::FollowSystem,
                            cli::TaskSplitCliMode::On => TaskSplitMode::Enabled,
                            cli::TaskSplitCliMode::Off => TaskSplitMode::Disabled,
                        };
                    }
                    // 设置子任务列表
                    task.subtasks = cleaned_subtasks.into_iter().map(SubTask::new).collect();

                    // 设置任务的提示内容
                    match prompt {
                        Some(prompt) if !prompt.is_empty() => {
                            task.prompt = prompt;
                        }
                        _ => {
                            task.prompt = first_non_empty_line(&task.description);
                        }
                    }
                    // 如果 prompt 仍为空，使用第一个子任务的内容
                    if task.prompt.is_empty()
                        && let Some(subtask) = task.subtasks.first()
                    {
                        task.prompt = subtask.content.clone();
                    }

                    // 创建任务并打印结果
                    let scheduler = AgentSchedulerStore::new();
                    let created =
                        create_native_task(&scheduler, std::path::Path::new(&project_path), task)
                            .with_context(|| {
                            format!("Failed to create task in {}", project_path.as_str())
                        })?;
                    print_task_json(&created)
                }
                // 读取任务
                TaskCommands::Read { id, status, include_archived, include_deleted, limit } => {
                    // 如果指定了任务 ID，直接加载该任务
                    if let Some(task_id) = id {
                        let scheduler = AgentSchedulerStore::new();
                        let task_id_value = TaskId::new(task_id.trim())?;
                        let task = load_native_task(
                            &scheduler,
                            std::path::Path::new(&project_path),
                            &task_id_value,
                        )?
                        .with_context(|| {
                            format!(
                                "Task not found in {} with id {}",
                                project_path.as_str(),
                                task_id
                            )
                        })?;
                        return print_task_json(&task);
                    }

                    // 解析状态过滤器
                    let status_filter = status
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(|value| {
                            TaskStatus::parse_key(value)
                                .with_context(|| format!("Invalid --status value: {value}"))
                        })
                        .transpose()?;

                    // 加载所有任务并应用过滤器
                    let scheduler = AgentSchedulerStore::new();
                    let mut tasks =
                        list_native_tasks(&scheduler, std::path::Path::new(&project_path))?;
                    tasks.retain(|task| {
                        let status_ok = status_filter.is_none_or(|s| task.status == s);
                        let archived_ok = include_archived || !task.archived;
                        let deleted_ok = include_deleted || !task.deleted;
                        status_ok && archived_ok && deleted_ok
                    });
                    // 按创建时间（降序）和优先级（升序）排序
                    tasks.sort_by(|a, b| {
                        b.created_at_ms
                            .cmp(&a.created_at_ms)
                            .then_with(|| a.priority.cmp(&b.priority))
                    });
                    // 限制返回的任务数量
                    if tasks.len() > limit {
                        tasks.truncate(limit);
                    }
                    print_tasks_json(&tasks)
                }
            }
        }

        /*
        Commands::Migrate { migrate_command } => {
            migration::handle_command(migrate_command, &config).await
        }
        */
        // 处理 Memory 命令：记忆系统管理
        Commands::Memory { memory_command } => {
            memory::cli::handle_command(memory_command, &config).await
        }

        /*
        Commands::Auth { auth_command } => handle_auth_command(auth_command, &config).await,
        */
        // 处理 Config 命令：配置管理
        Commands::Config { api_options, config_command } => match config_command {
            // 输出配置的 JSON Schema
            ConfigCommands::Schema => {
                let schema = schemars::schema_for!(config::Config);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&schema).expect("failed to serialize JSON Schema")
                );
                Ok(())
            }
            ConfigCommands::Gateway(command) => {
                handlers::api::run(handlers::api::config_args(api_options, command)).await
            }
        },
    }
}

fn init_tracing(interactive_agent_mode: bool, acp_mode: bool) {
    let terminal_filter = default_tracing_filter();
    let file_filter = default_tracing_filter();
    let runtime_log_dir = vw_shared::runtime_log::default_runtime_log_dir();
    if let Err(err) = vw_shared::runtime_log::ensure_runtime_log_dir(&runtime_log_dir) {
        eprintln!("failed to create runtime log directory {}: {err}", runtime_log_dir.display());
    }

    if interactive_agent_mode {
        let terminal_layer = fmt::layer()
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
            .with_writer(std::io::sink)
            .with_filter(terminal_filter);
        let file_layer = fmt::layer()
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
            .with_ansi(false)
            .with_writer(vw_shared::runtime_log::RuntimeLogMakeWriter::new(runtime_log_dir))
            .with_filter(file_filter);
        let subscriber = tracing_subscriber::registry().with(terminal_layer).with(file_layer);
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    } else if acp_mode {
        let terminal_layer = fmt::layer()
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
            .with_writer(std::io::stderr)
            .with_filter(terminal_filter);
        let file_layer = fmt::layer()
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
            .with_ansi(false)
            .with_writer(vw_shared::runtime_log::RuntimeLogMakeWriter::new(runtime_log_dir))
            .with_filter(file_filter);
        let subscriber = tracing_subscriber::registry().with(terminal_layer).with(file_layer);
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    } else {
        let terminal_layer = fmt::layer()
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
            .with_filter(terminal_filter);
        let file_layer = fmt::layer()
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
            .with_ansi(false)
            .with_writer(vw_shared::runtime_log::RuntimeLogMakeWriter::new(runtime_log_dir))
            .with_filter(file_filter);
        let subscriber = tracing_subscriber::registry().with(terminal_layer).with(file_layer);
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    }
}

fn default_tracing_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,rmcp=warn"))
}
#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

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

use vw_agent::channels::ChannelCommands;
use vw_agent::cron;
use vw_agent::integrations;
use vw_agent::provider::provider;
use vw_agent::{
    channels, config, daemon, doctor, gateway, initialize_runtime_build_id, memory, observability,
    plugins, security, service, skills,
};

use config::Config;
use config::schema::ChannelsConfigExt;
use config::schema::ConfigExt;

mod acp;
use cli_command_schema as cli;
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
            approval, channels, config, memory, observability, providers, runtime, security, shell,
            skills, tools,
        };

        pub(crate) mod session {
            pub(crate) use vw_agent::session::processor;
        }

        #[allow(clippy::module_inception)]
        pub(crate) mod agent {
            pub(crate) mod loop_ {
                pub(crate) use vw_agent::agent::loop_::{context, core, instructions};
            }
        }
    }
}

use cli::{Cli, Commands, ConfigCommands, DoctorCommands};

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
        Commands::Api(api_commands) => cli_gateway_commands::run(api_commands.into_args()).await,

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
            Box::pin(cli_agent_session::run(
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
            cli_gateway_process::prepare_gateway_port(&host, port).await?;
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
            cli_security_commands::handle_estop_command(&config, estop_command, level, domains, tools)
        }

        // 处理 Security 命令：安全策略管理
        Commands::Security { security_command } => {
            cli_security_commands::handle_security_command(&config, security_command).await
        }

        // 处理 Cron 命令：定时任务管理
        Commands::Cron { cron_command } => cron::handle_command(cron_command, &config),

        // 处理 Providers 命令：列出支持的 AI 提供者
        Commands::Providers => {
            // 获取所有可用的提供者列表
            let provider_map = provider::list().await?;
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
            cli_task_commands::handle_command(&project_dir, task_command)
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
                cli_gateway_commands::run(cli_gateway_commands::config_args(api_options, command))
                    .await
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

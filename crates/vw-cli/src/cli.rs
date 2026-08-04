//! # CLI 命令行接口模块
//!
//! 本模块定义了 VibeWindow 代理的命令行界面（CLI）结构，包含所有子命令、
//! 参数解析和 shell 补全生成功能。
//!
//! ## 主要功能
//!
//! - 定义顶层 CLI 结构 [`Cli`] 和全局参数
//! - 定义所有子命令枚举 [`Commands`]，包括 agent、gateway、daemon 等
//! - 支持多种 shell 的自动补全脚本生成
//! - 提供任务管理、安全控制、诊断等子命令的参数定义
//!
//! ## 架构说明
//!
//! 使用 `clap` 库的派生宏实现命令行参数解析，通过 `#[derive(Parser)]` 和
//! `#[derive(Subcommand)]` 自动生成解析逻辑。所有命令结构体使用 `pub(crate)`
//! 可见性，确保仅在当前二进制包内可访问。

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

use crate::parse_temperature;
use vw_agent::channels::ChannelCommands;
use vw_agent::cron::CronCommands;
use vw_agent::integrations::IntegrationCommands;
use vw_agent::memory::cli::MemoryCommands;
use vw_agent::plugins::PluginCommands;
use vw_agent::security;
use vw_agent::service::ServiceCommands;
use vw_agent::skill::SkillCommands;

#[path = "cli/mod.rs"]
mod runtime;

pub(crate) use runtime::run;
pub(crate) use runtime::{processor, session, setup, tui_v2};

/// Shell 补全脚本支持的 shell 类型枚举
///
/// 定义了 `vibewindow completions` 命令支持的所有目标 shell 类型，
/// 用于生成对应 shell 的命令行自动补全脚本。
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum CompletionShell {
    /// Bash shell 补全
    #[value(name = "bash")]
    Bash,
    /// Fish shell 补全
    #[value(name = "fish")]
    Fish,
    /// Zsh shell 补全
    #[value(name = "zsh")]
    Zsh,
    /// PowerShell 补全
    #[value(name = "powershell")]
    PowerShell,
    /// Elvish shell 补全
    #[value(name = "elvish")]
    Elvish,
}

/// 紧急停止（Estop）级别参数枚举
///
/// 定义了不同级别的紧急停止操作，用于在异常情况下快速限制或终止代理行为。
/// 级别从宽到严依次为：工具冻结 -> 域名阻止 -> 网络终止 -> 全部终止。
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum EstopLevelArg {
    /// 终止所有操作 - 最高级别的紧急停止
    #[value(name = "kill-all")]
    KillAll,
    /// 终止所有网络连接 - 阻止代理进行任何网络通信
    #[value(name = "network-kill")]
    NetworkKill,
    /// 域名阻止 - 阻止对指定域名的访问
    #[value(name = "domain-block")]
    DomainBlock,
    /// 工具冻结 - 禁用指定的工具，保持其他功能正常
    #[value(name = "tool-freeze")]
    ToolFreeze,
}

/// VibeWindow CLI 主入口结构体
///
/// 定义了命令行工具的顶层参数和子命令结构。
/// 使用 clap 的 Parser 派生宏自动实现参数解析。
///
/// # 全局参数
///
/// - `--config-dir`: 指定配置文件目录路径（可选）
///
/// # 示例
///
/// ```bash
/// # 使用默认配置目录
/// vibewindow agent
/// # 指定自定义配置目录
/// vibewindow --config-dir /path/to/config agent
/// ```
#[derive(Parser, Debug)]
#[command(name = "shiju")]
#[command(author = "theonlyhennygod")]
#[command(version)]
#[command(about = "The fastest, smallest AI assistant.", long_about = None)]
pub(crate) struct Cli {
    /// 配置文件目录路径（全局参数）
    ///
    /// 指定 VibeWindow 配置文件的存储目录。如未指定，
    /// 将使用系统默认位置（~/.config/vibewindow 或平台等效路径）。
    #[arg(long, global = true)]
    pub(crate) config_dir: Option<String>,

    /// 要执行的子命令（必需）
    #[command(subcommand)]
    pub(crate) command: Commands,
}

/// CLI 子命令枚举
///
/// 定义了 VibeWindow 支持的所有一级子命令。每个变体对应一个独立的功能模块，
/// 部分命令还包含二级子命令（通过嵌套的 `#[command(subcommand)]` 实现）。
#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// 通过 stdin/stdout 运行本地 ACP Agent。
    Acp,

    /// 启动 AI 代理循环
    ///
    /// 启动与配置的 AI 提供商的交互式聊天会话。使用 --message 参数可执行
    /// 单次查询而无需进入交互模式。
    ///
    /// # 示例
    ///
    /// ```bash
    /// vibewindow agent                              # 交互式会话
    /// vibewindow agent -m "总结今日日志"             # 单条消息模式
    /// vibewindow agent -p anthropic --model claude-sonnet-4-20250514
    /// vibewindow agent --peripheral nucleo-f401re:/dev/ttyACM0
    /// vibewindow agent --autonomy-level full --max-actions-per-hour 100
    /// vibewindow agent -m "快速任务" --memory-backend none --compact-context
    /// ```
    #[command(long_about = "\
Start the AI agent loop.

Launches an interactive chat session with the configured AI provider. \
Use --message for single-shot queries without entering interactive mode.

Examples:
  vibewindow agent                              # interactive session
  vibewindow agent -m \"Summarize today's logs\"  # single message
  vibewindow agent -p anthropic --model claude-sonnet-4-20250514
  vibewindow agent --peripheral nucleo-f401re:/dev/ttyACM0
  vibewindow agent --autonomy-level full --max-actions-per-hour 100
  vibewindow agent -m \"quick task\" --memory-backend none --compact-context")]
    Agent {
        /// 单条消息模式（不进入交互模式）
        #[arg(short, long)]
        message: Option<String>,

        /// 使用的提供商（openrouter, anthropic, openai, openai-codex）
        #[arg(short, long)]
        provider: Option<String>,

        /// 使用的模型名称
        #[arg(long)]
        model: Option<String>,

        /// 温度参数（0.0 - 2.0），控制输出随机性
        #[arg(short, long, default_value = "0.7", value_parser = parse_temperature)]
        temperature: f64,

        /// 附加外设设备（格式：board:path，例如 nucleo-f401re:/dev/ttyACM0）
        #[arg(long)]
        peripheral: Vec<String>,

        /// 自主级别（read_only, supervised, full）
        #[arg(long, value_parser = clap::value_parser!(security::AutonomyLevel))]
        autonomy_level: Option<security::AutonomyLevel>,

        /// 每小时最大 shell/工具操作次数限制
        #[arg(long)]
        max_actions_per_hour: Option<u32>,

        /// 每条消息最大工具调用迭代次数
        #[arg(long)]
        max_tool_iterations: Option<usize>,

        /// 最大对话历史消息数
        #[arg(long)]
        max_history_messages: Option<usize>,

        /// 启用紧凑上下文模式（为受限模型生成更小的提示）
        #[arg(long)]
        compact_context: bool,

        /// 记忆后端类型（sqlite, markdown, none）
        #[arg(long)]
        memory_backend: Option<String>,
    },

    /// 启动网关服务器（webhooks、websockets）
    ///
    /// 运行 HTTP/WebSocket 网关，接收传入的 webhook 事件和 WebSocket 连接。
    /// 绑定地址默认使用配置文件中的值（gateway.host / gateway.port）。
    ///
    /// # 示例
    ///
    /// ```bash
    /// vibewindow gateway                  # 使用配置默认值
    /// vibewindow gateway -p 8080          # 在 8080 端口监听
    /// vibewindow gateway --host 0.0.0.0   # 绑定到所有网络接口
    /// vibewindow gateway -p 0             # 使用随机可用端口
    /// ```
    #[command(long_about = "\
Start the gateway server (webhooks, websockets).

Runs the HTTP/WebSocket gateway that accepts incoming webhook events \
and WebSocket connections. Bind address defaults to the values in \
your config file (gateway.host / gateway.port).

Examples:
  vibewindow gateway                  # use config defaults
  vibewindow gateway -p 8080          # listen on port 8080
  vibewindow gateway --host 0.0.0.0   # bind to all interfaces
  vibewindow gateway -p 0             # random available port")]
    Gateway {
        /// 监听端口（使用 0 表示随机可用端口）；默认使用配置中的 gateway.port
        #[arg(short, long)]
        port: Option<u16>,

        /// 绑定主机地址；默认使用配置中的 gateway.host
        #[arg(long)]
        host: Option<String>,
    },

    /// 启动长时间运行的自主运行时（网关 + 通道 + 心跳 + 调度器）
    ///
    /// 启动完整的 VibeWindow 运行时：网关服务器、所有已配置的通道
    /// （Telegram、Discord、Slack 等）、心跳监控和 cron 调度器。
    /// 这是在生产环境中或将 VibeWindow 作为常驻助手运行的推荐方式。
    ///
    /// 使用 'vibewindow service install' 可将守护进程注册为操作系统
    /// 服务（systemd/launchd），实现开机自启动。
    ///
    /// # 示例
    ///
    /// ```bash
    /// vibewindow daemon                   # 使用配置默认值
    /// vibewindow daemon -p 9090           # 网关使用 9090 端口
    /// vibewindow daemon --host 127.0.0.1  # 仅监听本地主机
    /// ```
    #[command(long_about = "\
Start the long-running autonomous daemon.

Launches the full VibeWindow runtime: gateway server, all configured \
channels (Telegram, Discord, Slack, etc.), heartbeat monitor, and \
the cron scheduler. This is the recommended way to run VibeWindow in \
production or as an always-on assistant.

Use 'vibewindow service install' to register the daemon as an OS \
service (systemd/launchd) for auto-start on boot.

Examples:
  vibewindow daemon                   # use config defaults
  vibewindow daemon -p 9090           # gateway on port 9090
  vibewindow daemon --host 127.0.0.1  # localhost only")]
    Daemon {
        /// 监听端口（使用 0 表示随机可用端口）；默认使用配置中的 gateway.port
        #[arg(short, long)]
        port: Option<u16>,

        /// 绑定主机地址；默认使用配置中的 gateway.host
        #[arg(long)]
        host: Option<String>,
    },

    /// 管理 OS 服务生命周期（launchd/systemd 用户服务）
    Service {
        /// 使用的 init 系统：auto（自动检测）、systemd 或 openrc
        #[arg(long, default_value = "auto", value_parser = ["auto", "systemd", "openrc"])]
        service_init: String,

        /// 服务子命令
        #[command(subcommand)]
        service_command: ServiceCommands,
    },

    /// 管理本机后台 server 服务（兼容 `vibewindow server start`）
    ///
    /// 这是 `service` 命令的兼容别名，便于桌面端和旧脚本使用
    /// `vibewindow server start` 启动本机常驻服务。
    Server {
        /// 使用的 init 系统：auto（自动检测）、systemd 或 openrc
        #[arg(long, default_value = "auto", value_parser = ["auto", "systemd", "openrc"])]
        service_init: String,

        /// server 生命周期子命令
        #[command(subcommand)]
        server_command: ServiceCommands,
    },

    /// 运行诊断（用于守护进程/调度器/通道健康检查）
    Doctor {
        /// 诊断子命令
        #[command(subcommand)]
        doctor_command: Option<DoctorCommands>,
    },

    /// 显示系统状态（完整详情）
    Status,

    /*
    /// Self-update VibeWindow to the latest version
    #[command(long_about = "\
Self-update VibeWindow to the latest release from GitHub.

Downloads the appropriate pre-built binary for your platform and
replaces the current executable. Requires write permissions to
the binary location.

Examples:
  vibewindow update              # Update to latest version
  vibewindow update --check      # Check for updates without installing
  vibewindow update --force      # Reinstall even if already up to date")]
    Update {
        /// Check for updates without installing
        #[arg(long)]
        check: bool,

        /// Force update even if already at latest version
        #[arg(long)]
        force: bool,
    },
    */
    /// 触发、检查和恢复紧急停止状态
    ///
    /// 紧急停止（Estop）用于在异常情况下快速限制或终止代理行为。
    /// 支持多种级别的停止操作，以及后续的恢复操作。
    ///
    /// # 示例
    ///
    /// ```bash
    /// vibewindow estop                                    # 触发默认紧急停止
    /// vibewindow estop --level network-kill              # 终止所有网络连接
    /// vibewindow estop --level domain-block --domain "*.chase.com"  # 阻止特定域名
    /// vibewindow estop --level tool-freeze --tool shell --tool browser  # 冻结特定工具
    /// vibewindow estop status                            # 查看当前状态
    /// vibewindow estop resume --network                  # 恢复网络
    /// vibewindow estop resume --domain "*.chase.com"     # 恢复特定域名访问
    /// vibewindow estop resume --tool shell               # 恢复特定工具
    /// ```
    Estop {
        /// 紧急停止子命令
        #[command(subcommand)]
        estop_command: Option<EstopSubcommands>,

        /// 从 `vibewindow estop` 触发紧急停止时使用的级别
        #[arg(long, value_enum)]
        level: Option<EstopLevelArg>,

        /// 用于 `domain-block` 级别的域名模式（可重复指定）
        #[arg(long = "domain")]
        domains: Vec<String>,

        /// 用于 `tool-freeze` 级别的工具名称（可重复指定）
        #[arg(long = "tool")]
        tools: Vec<String>,
    },

    /// 管理安全维护任务
    ///
    /// 此命令组用于维护运行时使用的安全相关数据存储。
    ///
    /// # 示例
    ///
    /// ```bash
    /// vibewindow security update-guard-corpus
    /// vibewindow security update-guard-corpus --source builtin
    /// vibewindow security update-guard-corpus --source ./datassetsa/security/attack-corpus-v1.jsonl
    /// vibewindow security update-guard-corpus --source https://example.com/guard-corpus.jsonl --checksum <sha256>
    /// ```
    #[command(long_about = "\
Manage security maintenance tasks.

Commands in this group maintain security-related data stores used at runtime.

Examples:
  vibewindow security update-guard-corpus
  vibewindow security update-guard-corpus --source builtin
  vibewindow security update-guard-corpus --source ./assets/security/attack-corpus-v1.jsonl
  vibewindow security update-guard-corpus --source https://example.com/guard-corpus.jsonl --checksum <sha256>")]
    Security {
        /// 安全子命令
        #[command(subcommand)]
        security_command: SecurityCommands,
    },

    /// 配置和管理计划任务
    ///
    /// 使用 cron 表达式、RFC 3339 时间戳、持续时间或固定间隔来调度
    /// 重复性、一次性或基于间隔的任务。
    ///
    /// Cron 表达式使用标准 5 字段格式：'min hour day month weekday'。
    /// 时区默认为 UTC，可通过 --tz 参数和 IANA 时区名称覆盖。
    ///
    /// # 示例
    ///
    /// ```bash
    /// vibewindow cron list
    /// vibewindow cron add '0 9 * * 1-5' 'Good morning' --tz America/New_York
    /// vibewindow cron add '*/30 * * * *' 'Check system health'
    /// vibewindow cron add-at 2025-01-15T14:00:00Z 'Send reminder'
    /// vibewindow cron add-every 60000 'Ping heartbeat'
    /// vibewindow cron once 30m 'Run backup in 30 minutes'
    /// vibewindow cron pause <task-id>
    /// vibewindow cron update <task-id> --expression '0 8 * * *' --tz Europe/London
    /// ```
    #[command(long_about = "\
Configure and manage scheduled tasks.

Schedule recurring, one-shot, or interval-based tasks using cron \
expressions, RFC 3339 timestamps, durations, or fixed intervals.

Cron expressions use the standard 5-field format: \
'min hour day month weekday'. Timezones default to UTC; \
override with --tz and an IANA timezone name.

Examples:
  vibewindow cron list
  vibewindow cron add '0 9 * * 1-5' 'Good morning' --tz America/New_York
  vibewindow cron add '*/30 * * * *' 'Check system health'
  vibewindow cron add-at 2025-01-15T14:00:00Z 'Send reminder'
  vibewindow cron add-every 60000 'Ping heartbeat'
  vibewindow cron once 30m 'Run backup in 30 minutes'
  vibewindow cron pause <task-id>
  vibewindow cron update <task-id> --expression '0 8 * * *' --tz Europe/London")]
    Cron {
        /// 计划任务子命令
        #[command(subcommand)]
        cron_command: CronCommands,
    },

    /// 运行 Agent Scheduler 组件
    #[command(name = "agent-scheduler")]
    AgentScheduler {
        /// Agent Scheduler 子命令
        #[command(subcommand)]
        agent_scheduler_command: AgentSchedulerCommands,
    },

    /// 列出支持的 AI 提供商
    Providers,

    /// 管理通信通道（telegram、discord、slack）
    ///
    /// 添加、删除、列出和健康检查连接 VibeWindow 与消息平台的通道。
    /// 支持的通道类型：telegram、discord、slack、whatsapp、matrix、imessage、email。
    ///
    /// # 示例
    ///
    /// ```bash
    /// vibewindow channel list
    /// vibewindow channel doctor
    /// vibewindow channel add telegram '{"bot_token":"...","name":"my-bot"}'
    /// vibewindow channel remove my-bot
    /// vibewindow channel bind-telegram vibewindow_user
    /// ```
    #[command(long_about = "\
Manage communication channels.

Add, remove, list, and health-check channels that connect VibeWindow \
to messaging platforms. Supported channel types: telegram, discord, \
slack, whatsapp, matrix, imessage, email.

Examples:
  vibewindow channel list
  vibewindow channel doctor
  vibewindow channel add telegram '{\"bot_token\":\"...\",\"name\":\"my-bot\"}'
  vibewindow channel remove my-bot
  vibewindow channel bind-telegram vibewindow_user")]
    Channel {
        /// 通道子命令
        #[command(subcommand)]
        channel_command: ChannelCommands,
    },

    /// 浏览 50+ 集成
    Integrations {
        /// 集成子命令
        #[command(subcommand)]
        integration_command: IntegrationCommands,
    },

    /// 管理技能（用户定义的能力）
    Skills {
        /// 技能子命令
        #[command(subcommand)]
        skill_command: SkillCommands,
    },

    /// 管理插件包
    Plugins {
        /// 插件子命令
        #[command(subcommand)]
        plugin_command: PluginCommands,
    },

    /// 从 CLI 管理项目任务
    ///
    /// 创建和读取存储在 .vibewindow/tasks 目录中的项目任务。
    /// 使用 --project-dir 指定目标项目路径，默认为当前工作目录。
    ///
    /// # 示例
    ///
    /// ```bash
    /// vibewindow task create --prompt "检查 OAuth 回调并修复"
    /// vibewindow task create --prompt "检查 OAuth 回调并修复" --task-split off
    /// vibewindow task create --project-dir /path/to/repo --description "补充测试" --subtask "单元测试" --subtask "集成测试"
    /// vibewindow task read
    /// vibewindow task read --id T20260317.0001
    /// vibewindow task read --project-dir /path/to/repo --status pending --limit 20
    /// ```
    #[command(long_about = "\
Create and read project tasks stored in .vibewindow/tasks.

Use --project-dir to target a specific project path. Defaults to the \
current working directory.

Examples:
  vibewindow task create --prompt \"检查 OAuth 回调并修复\"
  vibewindow task create --prompt \"检查 OAuth 回调并修复\" --task-split off
  vibewindow task create --project-dir /path/to/repo --description \"补充测试\" --subtask \"单元测试\" --subtask \"集成测试\"
  vibewindow task read
  vibewindow task read --id T20260317.0001
  vibewindow task read --project-dir /path/to/repo --status pending --limit 20")]
    Task {
        /// 项目目录路径
        #[arg(long, default_value = ".")]
        project_dir: String,
        /// 任务子命令
        #[command(subcommand)]
        task_command: TaskCommands,
    },

    /*
    /// Migrate data from other agent runtimes
    Migrate {
        #[command(subcommand)]
        migrate_command: MigrateCommands,
    },

    /// Manage provider subscription authentication profiles
    Auth {
        #[command(subcommand)]
        auth_command: AuthCommands,
    },
    */
    /// 管理代理记忆（列出、获取、统计、清除）
    ///
    /// 列出、检查和清除代理存储的记忆条目。支持按类别和会话过滤、
    /// 分页以及带确认的批量清除。
    ///
    /// # 示例
    ///
    /// ```bash
    /// vibewindow memory stats
    /// vibewindow memory list
    /// vibewindow memory list --category core --limit 10
    /// vibewindow memory get <key>
    /// vibewindow memory clear --category conversation --yes
    /// ```
    #[command(long_about = "\
Manage agent memory entries.

List, inspect, and clear memory entries stored by the agent. \
Supports filtering by category and session, pagination, and \
batch clearing with confirmation.

Examples:
  vibewindow memory stats
  vibewindow memory list
  vibewindow memory list --category core --limit 10
  vibewindow memory get <key>
  vibewindow memory clear --category conversation --yes")]
    Memory {
        /// 记忆子命令
        #[command(subcommand)]
        memory_command: MemoryCommands,
    },

    /// 管理配置
    ///
    /// 检查和导出配置设置。使用 'schema' 子命令可转储配置文件的
    /// 完整 JSON Schema，其中记录了每个可用的键、类型和默认值。
    ///
    /// # 示例
    ///
    /// ```bash
    /// vibewindow config schema              # 打印 JSON Schema 到标准输出
    /// vibewindow config schema > schema.json
    /// ```
    #[command(long_about = "\
Manage VibeWindow configuration.

Inspect and export configuration settings. Use 'schema' to dump \
the full JSON Schema for the config file, which documents every \
available key, type, and default value.

Examples:
  vibewindow config schema              # print JSON Schema to stdout
  vibewindow config schema > schema.json")]
    Config {
        /// 配置子命令
        #[command(subcommand)]
        config_command: ConfigCommands,
    },

    /// 生成 shell 补全脚本到标准输出
    ///
    /// 脚本输出到标准输出，可以直接 source 使用：
    ///
    /// # 示例
    ///
    /// ```bash
    /// source <(vibewindow completions bash)
    /// vibewindow completions zsh > ~/.zfunc/_vibewindow
    /// vibewindow completions fish > ~/.config/fish/completions/vibewindow.fish
    /// ```
    #[command(long_about = "\
Generate shell completion scripts for `vibewindow`.

The script is printed to stdout so it can be sourced directly:

Examples:
  source <(vibewindow completions bash)
  vibewindow completions zsh > ~/.zfunc/_vibewindow
  vibewindow completions fish > ~/.config/fish/completions/vibewindow.fish")]
    Completions {
        /// 目标 shell 类型
        #[arg(value_enum)]
        shell: CompletionShell,
    },
}

/// 配置管理子命令
#[derive(Subcommand, Debug)]
pub(crate) enum ConfigCommands {
    /// 将完整的配置 JSON Schema 转储到标准输出
    Schema,
}

/// 任务管理子命令
#[derive(Subcommand, Debug)]
pub(crate) enum TaskCommands {
    /// 创建任务
    Create {
        /// 任务优先级（数字越小优先级越高）
        #[arg(long, default_value = "999")]
        priority: u32,
        /// 任务提示内容
        #[arg(long)]
        prompt: Option<String>,
        /// 任务描述
        #[arg(long)]
        description: Option<String>,
        /// 任务负责人
        #[arg(long)]
        assignee: Option<String>,
        /// 模型名称
        #[arg(long)]
        model: Option<String>,
        /// 执行器 ID：internal、opencode、claude、codex
        #[arg(long)]
        executor: Option<String>,
        /// 任务拆分模式：follow-system、on、off
        #[arg(long = "task-split", value_enum)]
        task_split: Option<TaskSplitCliMode>,
        /// 子任务项（可重复指定）
        #[arg(long = "subtask")]
        subtasks: Vec<String>,
    },
    /// 读取任务
    Read {
        /// 按 ID 读取单个任务
        #[arg(long)]
        id: Option<String>,
        /// 按状态键过滤
        #[arg(long)]
        status: Option<String>,
        /// 包含已归档任务
        #[arg(long)]
        include_archived: bool,
        /// 包含已删除任务
        #[arg(long)]
        include_deleted: bool,
        /// 列表模式下返回的最大任务数
        #[arg(long, default_value = "50")]
        limit: usize,
    },
}

/// Agent Scheduler 运行时子命令
#[derive(Subcommand, Debug)]
pub(crate) enum AgentSchedulerCommands {
    /// 启动单个 Rust Matrix worker
    #[command(name = "matrix-worker")]
    MatrixWorker {
        /// 平台作用域 ID
        #[arg(long, default_value = "local", value_parser = parse_required_scope_id)]
        platform_id: String,
        /// 公司作用域 ID
        #[arg(long, default_value = "local", value_parser = parse_required_scope_id)]
        company_id: String,
        /// Agent Scheduler 项目目录
        #[arg(long)]
        project_dir: String,
        /// workers 表中的 worker id
        #[arg(long)]
        worker_id: String,
    },
}

fn parse_required_scope_id(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        return Err("scope id must not be empty".to_string());
    }
    Ok(value.to_string())
}

/// CLI 任务拆分模式。
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum TaskSplitCliMode {
    FollowSystem,
    On,
    Off,
}

/// 紧急停止子命令
#[derive(Subcommand, Debug)]
pub(crate) enum EstopSubcommands {
    /// 打印当前紧急停止状态
    Status,
    /// 从已触发的紧急停止级别恢复
    Resume {
        /// 仅恢复网络终止
        #[arg(long)]
        network: bool,
        /// 恢复一个或多个被阻止的域名模式
        #[arg(long = "domain")]
        domains: Vec<String>,
        /// 恢复一个或多个被冻结的工具
        #[arg(long = "tool")]
        tools: Vec<String>,
        /// OTP 验证码。如省略且需要 OTP，将显示提示
        #[arg(long)]
        otp: Option<String>,
    },
}

/// 安全管理子命令
#[derive(Subcommand, Debug)]
pub(crate) enum SecurityCommands {
    /// 将语义提示注入语料库记录更新到配置的向量集合中
    UpdateGuardCorpus {
        /// 语料库来源：`builtin`（内置）、文件系统路径或 HTTP(S) URL
        #[arg(long)]
        source: Option<String>,
        /// 源载荷验证的预期 SHA-256 校验和（十六进制格式）
        #[arg(long)]
        checksum: Option<String>,
    },
}

/// 诊断子命令
#[derive(Subcommand, Debug)]
pub(crate) enum DoctorCommands {
    /// 探测各提供商的模型目录并报告可用性
    Models {
        /// 仅探测特定提供商（默认：所有已知提供商）
        #[arg(long)]
        provider: Option<String>,

        /// 优先使用缓存的目录（跳过强制实时刷新）
        #[arg(long)]
        use_cache: bool,
    },
    /// 查询运行时追踪事件（工具诊断和模型回复）
    Traces {
        /// 按 ID 显示特定追踪事件
        #[arg(long)]
        id: Option<String>,
        /// 按事件类型过滤列表输出
        #[arg(long)]
        event: Option<String>,
        /// 在消息/载荷中进行不区分大小写的文本匹配
        #[arg(long)]
        contains: Option<String>,
        /// 显示的最大事件数
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

/// 生成 shell 补全脚本
///
/// 为指定的 shell 类型生成命令行自动补全脚本，并写入到提供的 writer 中。
///
/// # 参数
///
/// * `shell` - 目标 shell 类型（bash、fish、zsh、powershell、elvish）
/// * `writer` - 用于写入补全脚本的输出流
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 示例
///
/// ```ignore
/// use std::io::stdout;
/// use vibe_agent::cli::{write_shell_completion, CompletionShell};
///
/// let mut out = stdout();
/// write_shell_completion(CompletionShell::Bash, &mut out)?;
/// ```
pub(crate) fn write_shell_completion<W: std::io::Write>(
    shell: CompletionShell,
    writer: &mut W,
) -> anyhow::Result<()> {
    use clap_complete::generate;
    use clap_complete::shells;

    // 获取 CLI 命令结构用于生成补全
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();

    // 根据目标 shell 类型生成对应的补全脚本
    match shell {
        CompletionShell::Bash => generate(shells::Bash, &mut cmd, bin_name.clone(), writer),
        CompletionShell::Fish => generate(shells::Fish, &mut cmd, bin_name.clone(), writer),
        CompletionShell::Zsh => generate(shells::Zsh, &mut cmd, bin_name.clone(), writer),
        CompletionShell::PowerShell => {
            generate(shells::PowerShell, &mut cmd, bin_name.clone(), writer);
        }
        CompletionShell::Elvish => generate(shells::Elvish, &mut cmd, bin_name, writer),
    }

    // 确保所有数据都写入到输出流
    writer.flush()?;
    Ok(())
}

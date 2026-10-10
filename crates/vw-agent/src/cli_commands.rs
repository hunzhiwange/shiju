use clap::Subcommand;
use serde::{Deserialize, Serialize};

/// 服务管理子命令
///
/// 提供代理服务的生命周期管理，包括安装、启动、停止、重启和状态检查。
/// 服务以系统守护进程方式运行，支持自动重启和配置热加载。
///
/// # 使用场景
///
/// - **Install**：首次部署时安装系统服务单元
/// - **Start**：启动代理服务
/// - **Stop**：停止代理服务
/// - **Restart**：应用最新配置后重启服务
/// - **Status**：检查服务运行状态和健康度
/// - **Uninstall**：移除系统服务单元
///
/// # 示例
///
/// ```bash
/// # 安装并启动服务
/// vibewindow service install
/// vibewindow service start
///
/// # 检查服务状态
/// vibewindow service status
///
/// # 应用配置后重启
/// vibewindow service restart
/// ```
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceCommands {
    /// 安装守护进程服务单元
    ///
    /// 安装后服务将自动启动并在崩溃时自动重启。
    /// 适用于首次部署或系统迁移场景。
    Install,

    /// 启动守护进程服务
    ///
    /// 启动已安装的代理服务。
    /// 如果服务未安装，将返回错误。
    Start,

    /// 停止守护进程服务
    ///
    /// 停止正在运行的代理服务。
    /// 停止后可通过 `start` 命令重新启动。
    Stop,

    /// 重启守护进程服务以应用最新配置
    ///
    /// 执行优雅重启，加载最新的配置文件。
    /// 适用于配置变更后的热加载场景。
    Restart,

    /// 检查守护进程服务状态
    ///
    /// 返回服务的运行状态、PID、运行时长等信息。
    Status,

    /// 卸载守护进程服务单元
    ///
    /// 从系统中移除服务单元文件。
    /// 卸载前应先停止服务。
    Uninstall,
}

/// 通道管理子命令
///
/// 提供消息通道的配置、管理和健康检查功能。
/// 支持多种通道类型：Telegram、Discord、Slack、WhatsApp、iMessage、Email。
///
/// # 通道类型
///
/// - **telegram**：Telegram Bot API
/// - **discord**：Discord Bot
/// - **slack**：Slack App
/// - **whatsapp**：WhatsApp Business API
/// - **imessage**：Apple iMessage
/// - **email**：电子邮件
///
/// # 示例
///
/// ```bash
/// # 列出所有配置的通道
/// vibewindow channel list
///
/// # 添加 Telegram 通道
/// vibewindow channel add telegram '{"bot_token":"YOUR_TOKEN","name":"my-bot"}'
///
/// # 绑定允许的用户
/// vibewindow channel bind-telegram vibewindow_user
/// ```
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChannelCommands {
    /// 列出所有已配置的通道
    ///
    /// 显示每个通道的名称、类型和基本状态信息。
    List,

    /// 启动所有已配置的通道
    ///
    /// 启动所有配置文件中定义的通道连接。
    /// 此命令在 main.rs 中以异步方式处理。
    Start,

    /// 对已配置的通道运行健康检查
    ///
    /// 检查每个通道的连接状态、认证有效性和可达性。
    /// 此命令在 main.rs 中以异步方式处理。
    Doctor,

    /// 添加新的通道配置
    ///
    /// 通过指定通道类型和 JSON 配置对象来添加新通道。
    /// 配置对象必须包含该通道类型所需的全部字段。
    ///
    /// # 支持的通道类型
    ///
    /// - telegram、discord、slack、whatsapp、imessage、email
    ///
    /// # 示例
    ///
    /// ```bash
    /// # 添加 Telegram 通道
    /// vibewindow channel add telegram '{"bot_token":"...","name":"my-bot"}'
    ///
    /// # 添加 Discord 通道
    /// vibewindow channel add discord '{"bot_token":"...","name":"my-discord"}'
    /// ```
    #[command(long_about = "\
添加新的通道配置。

提供通道类型和包含该通道类型所需配置键的 JSON 对象。

支持的通道类型：telegram、discord、slack、whatsapp、imessage、email。

示例：
  vibewindow channel add telegram '{\"bot_token\":\"...\",\"name\":\"my-bot\"}'
  vibewindow channel add discord '{\"bot_token\":\"...\",\"name\":\"my-discord\"}'")]
    Add {
        /// 通道类型
        ///
        /// 指定要添加的通道类型，支持：
        /// telegram、discord、slack、whatsapp、imessage、email
        channel_type: String,

        /// 配置对象（JSON 格式）
        ///
        /// 包含该通道类型所需的所有配置字段。
        /// 必须是有效的 JSON 字符串。
        config: String,
    },

    /// 移除通道配置
    ///
    /// 根据通道名称从配置中移除对应的通道。
    /// 移除后需要重启服务以生效。
    Remove {
        /// 要移除的通道名称
        name: String,
    },

    /// 将 Telegram 身份绑定到允许列表
    ///
    /// 添加 Telegram 用户名（不带 '@' 前缀）或数字用户 ID 到通道允许列表，
    /// 使代理能够响应该身份发送的消息。
    ///
    /// # 参数格式
    ///
    /// - 用户名：不带 '@' 前缀（例如：vibewindow_user）
    /// - 用户 ID：纯数字（例如：123456789）
    ///
    /// # 示例
    ///
    /// ```bash
    /// # 通过用户名绑定
    /// vibewindow channel bind-telegram vibewindow_user
    ///
    /// # 通过用户 ID 绑定
    /// vibewindow channel bind-telegram 123456789
    /// ```
    #[command(long_about = "\
将 Telegram 身份绑定到允许列表。

添加 Telegram 用户名（不带 '@' 前缀）或数字用户 ID \
到通道允许列表，使代理能够响应该身份发送的消息。

示例：
  vibewindow channel bind-telegram vibewindow_user
  vibewindow channel bind-telegram 123456789")]
    BindTelegram {
        /// Telegram 身份标识
        ///
        /// 可以是用户名（不带 '@'）或数字用户 ID
        identity: String,
    },
}

/// 技能管理子命令
///
/// 提供技能的安装、审计、列表和移除功能。
/// 技能是可扩展的功能模块，用于增强代理的能力。
///
/// # 技能生命周期
///
/// 1. **Install**：从 URL 或本地路径安装技能
/// 2. **Audit**：审计技能源码或已安装的技能
/// 3. **List**：列出所有已安装的技能
/// 4. **Remove**：移除不再需要的技能
///
/// # 安全说明
///
/// - 安装前建议使用 `audit` 命令检查技能安全性
/// - 仅从可信来源安装技能
/// - 定期审计已安装的技能
///
/// # 示例
///
/// ```bash
/// # 列出所有已安装的技能
/// vibewindow skill list
///
/// # 从 URL 安装技能
/// vibewindow skill install https://example.com/skills/weather
///
/// # 审计技能
/// vibewindow skill audit ./my-skill
/// ```
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillCommands {
    /// 列出所有已安装的技能
    ///
    /// 显示每个技能的名称、版本和来源信息。
    List,

    /// 审计技能源目录或已安装的技能
    ///
    /// 检查技能的安全性、合规性和最佳实践。
    /// 可用于审计本地技能目录或已安装的技能。
    ///
    /// # 参数
    ///
    /// - `source`：可以是本地路径或已安装技能的名称
    Audit {
        /// 技能路径或已安装的技能名称
        source: String,
    },

    /// 从 URL 或本地路径安装新技能
    ///
    /// 下载并安装技能到代理的技能目录。
    /// 安装完成后技能立即可用。
    ///
    /// # 支持的来源
    ///
    /// - HTTP/HTTPS URL
    /// - 本地文件系统路径
    /// - Git 仓库 URL
    Install {
        /// 来源 URL 或本地路径
        source: String,
    },

    /// 移除已安装的技能
    ///
    /// 从代理中移除指定的技能。
    /// 移除后需要重启服务以完全卸载。
    Remove {
        /// 要移除的技能名称
        name: String,
    },
}

/// 插件管理子命令
///
/// 提供本地插件包的安装、审计、启用、禁用、列出和移除能力。
/// 第一阶段仅支持本地目录源；远程 Git、npm、marketplace 等来源会返回明确错误。
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginCommands {
    /// 列出所有已安装插件
    List,

    /// 审计本地插件目录或已安装插件
    Audit {
        /// 插件路径或已安装插件名称
        source: String,
    },

    /// 从本地目录安装插件
    Install {
        /// 本地插件目录
        source: String,
    },

    /// 启用已安装插件
    Enable {
        /// 插件名称
        name: String,
    },

    /// 禁用已安装插件
    Disable {
        /// 插件名称
        name: String,
    },

    /// 删除已安装插件
    Remove {
        /// 插件名称
        name: String,
    },
}

/// 迁移子命令
///
/// 提供从其他系统导入数据的功能。
/// 当前支持从 OpenClaw 工作区迁移数据到 VibeWindow。
///
/// # 支持的迁移源
///
/// - **OpenClaw**：从 OpenClaw 工作区导入记忆数据
///
/// # 迁移流程
///
/// 1. 使用 `--dry-run` 预览迁移内容
/// 2. 确认无误后执行实际迁移
/// 3. 验证迁移结果
///
/// # 示例
///
/// ```bash
/// # 预览 OpenClaw 迁移（不写入数据）
/// vibewindow migrate openclaw --dry-run
///
/// # 从自定义路径迁移
/// vibewindow migrate openclaw --source /path/to/openclaw/workspace
/// ```
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MigrateCommands {
    /// 从 OpenClaw 工作区导入记忆到当前 VibeWindow 工作区
    ///
    /// 将 OpenClaw 的记忆数据迁移到 VibeWindow 的记忆系统中。
    /// 默认从 `~/.openclaw/workspace` 读取数据。
    ///
    /// # 参数
    ///
    /// - `--source`：可选，指定 OpenClaw 工作区路径
    /// - `--dry-run`：仅预览迁移内容，不写入任何数据
    Openclaw {
        /// OpenClaw 工作区路径（可选）
        ///
        /// 默认值：`~/.openclaw/workspace`
        #[arg(long)]
        source: Option<std::path::PathBuf>,

        /// 仅预览迁移，不写入数据
        ///
        /// 用于验证迁移内容和检查兼容性问题。
        #[arg(long)]
        dry_run: bool,
    },
}

/// 定时任务子命令
///
/// 提供基于 Cron 表达式的定时任务管理功能。
/// 支持周期性任务、一次性任务和固定间隔任务。
///
/// # 任务类型
///
/// - **周期性任务**：使用标准 5 字段 Cron 表达式
/// - **一次性任务**：指定 RFC3339 时间戳执行一次
/// - **固定间隔任务**：按毫秒间隔重复执行
/// - **延迟任务**：人类可读的延迟时间后执行
///
/// # Cron 表达式格式
///
/// 使用标准 5 字段语法：`分 时 日 月 星期`
///
/// | 字段 | 范围 | 说明 |
/// |------|------|------|
/// | 分 | 0-59 | 分钟 |
/// | 时 | 0-23 | 小时 |
/// | 日 | 1-31 | 日期 |
/// | 月 | 1-12 | 月份 |
/// | 星期 | 0-7 | 星期（0 和 7 都表示周日） |
///
/// # 时区
///
/// 默认使用 UTC 时区，可通过 `--tz` 参数指定 IANA 时区名称。
///
/// # 示例
///
/// ```bash
/// # 每个工作日早上 9 点执行
/// vibewindow cron add '0 9 * * 1-5' 'Good morning' --tz America/New_York
///
/// # 每 30 分钟执行一次
/// vibewindow cron add '*/30 * * * *' 'Check system health'
///
/// # 在指定时间执行一次
/// vibewindow cron add-at 2025-01-15T14:00:00Z 'Send reminder'
///
/// # 每分钟执行一次
/// vibewindow cron add-every 60000 'Ping heartbeat'
///
/// # 30 分钟后执行一次
/// vibewindow cron once 30m 'Run backup in 30 minutes'
/// ```
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CronCommands {
    /// 列出所有已调度的任务
    ///
    /// 显示每个任务的 ID、名称、Cron 表达式、下次执行时间等信息。
    List,

    /// 添加新的周期性定时任务
    ///
    /// 使用标准 5 字段 Cron 表达式创建周期性任务。
    /// 默认使用 UTC 时区，可通过 `--tz` 参数指定其他时区。
    ///
    /// # Cron 语法
    ///
    /// - `*`：任意值
    /// - `,`：值列表（如 `1,3,5`）
    /// - `-`：范围（如 `1-5`）
    /// - `/`：步长（如 `*/15` 表示每 15 分钟）
    ///
    /// # 示例
    ///
    /// ```bash
    /// # 每个工作日早上 9 点（纽约时区）
    /// vibewindow cron add '0 9 * * 1-5' 'Good morning' --tz America/New_York
    ///
    /// # 每 30 分钟执行一次
    /// vibewindow cron add '*/30 * * * *' 'Check system health'
    /// ```
    #[command(long_about = "\
添加新的周期性定时任务。

使用标准 5 字段 Cron 语法：'分 时 日 月 星期'。
默认使用 UTC 时区；使用 --tz 参数和 IANA \
时区名称来覆盖。

示例：
  vibewindow cron add '0 9 * * 1-5' 'Good morning' --tz America/New_York
  vibewindow cron add '*/30 * * * *' 'Check system health'")]
    Add {
        /// Cron 表达式
        ///
        /// 标准 5 字段格式：分 时 日 月 星期
        expression: String,

        /// IANA 时区名称（可选）
        ///
        /// 例如：America/Los_Angeles、Asia/Shanghai、Europe/London
        #[arg(long)]
        tz: Option<String>,

        /// 要执行的命令
        command: String,
    },

    /// 在指定 RFC3339 时间戳添加一次性任务
    ///
    /// 创建一个在特定 UTC 时间执行一次的任务。
    /// 执行后任务自动标记为完成。
    ///
    /// # 时间格式
    ///
    /// 必须使用 RFC 3339 格式：`YYYY-MM-DDTHH:MM:SSZ`
    ///
    /// # 示例
    ///
    /// ```bash
    /// vibewindow cron add-at 2025-01-15T14:00:00Z 'Send reminder'
    /// vibewindow cron add-at 2025-12-31T23:59:00Z 'Happy New Year!'
    /// ```
    #[command(long_about = "\
添加一个在指定 UTC 时间戳触发的一次性任务。

时间戳必须使用 RFC 3339 格式（例如 2025-01-15T14:00:00Z）。

示例：
  vibewindow cron add-at 2025-01-15T14:00:00Z 'Send reminder'
  vibewindow cron add-at 2025-12-31T23:59:00Z 'Happy New Year!'")]
    AddAt {
        /// 一次性任务的时间戳（RFC3339 格式）
        ///
        /// 格式：YYYY-MM-DDTHH:MM:SSZ
        at: String,

        /// 要执行的命令
        command: String,
    },

    /// 添加固定间隔的定时任务
    ///
    /// 创建一个按固定毫秒间隔重复执行的任务。
    /// 适用于需要定期执行的监控、心跳等场景。
    ///
    /// # 间隔单位
    ///
    /// 间隔以毫秒为单位：
    /// - 60000 = 1 分钟
    /// - 3600000 = 1 小时
    /// - 86400000 = 1 天
    ///
    /// # 示例
    ///
    /// ```bash
    /// # 每分钟执行一次
    /// vibewindow cron add-every 60000 'Ping heartbeat'
    ///
    /// # 每小时执行一次
    /// vibewindow cron add-every 3600000 'Hourly report'
    /// ```
    #[command(long_about = "\
添加一个按固定间隔重复执行的任务。

间隔以毫秒为单位。例如，60000 = 1 分钟。

示例：
  vibewindow cron add-every 60000 'Ping heartbeat'     # 每分钟
  vibewindow cron add-every 3600000 'Hourly report'    # 每小时")]
    AddEvery {
        /// 间隔时间（毫秒）
        ///
        /// 任务将按此间隔重复执行
        every_ms: u64,

        /// 要执行的命令
        command: String,
    },

    /// 添加一次性延迟任务
    ///
    /// 创建一个从当前时间延迟指定时长后执行的任务。
    /// 支持人类可读的时长格式。
    ///
    /// # 时长格式
    ///
    /// - `s`：秒（如 `30s`）
    /// - `m`：分钟（如 `30m`）
    /// - `h`：小时（如 `2h`）
    /// - `d`：天（如 `1d`）
    ///
    /// # 示例
    ///
    /// ```bash
    /// # 30 分钟后执行
    /// vibewindow cron once 30m 'Run backup in 30 minutes'
    ///
    /// # 2 小时后执行
    /// vibewindow cron once 2h 'Follow up on deployment'
    ///
    /// # 1 天后执行
    /// vibewindow cron once 1d 'Daily check'
    /// ```
    #[command(long_about = "\
添加一个从现在开始延迟指定时长后执行的一次性任务。

接受人类可读的时长格式：s（秒）、m（分钟）、\
h（小时）、d（天）。

示例：
  vibewindow cron once 30m 'Run backup in 30 minutes'
  vibewindow cron once 2h 'Follow up on deployment'
  vibewindow cron once 1d 'Daily check'")]
    Once {
        /// 延迟时长
        ///
        /// 支持格式：s（秒）、m（分钟）、h（小时）、d（天）
        delay: String,

        /// 要执行的命令
        command: String,
    },

    /// 移除已调度的任务
    ///
    /// 根据任务 ID 删除定时任务。
    /// 可以通过 `list` 命令查看任务 ID。
    Remove {
        /// 任务 ID
        id: String,
    },

    /// 更新已调度的任务
    ///
    /// 修改现有任务的一个或多个字段。
    /// 只更新指定的字段，其他字段保持不变。
    ///
    /// # 可更新字段
    ///
    /// - `--expression`：新的 Cron 表达式
    /// - `--tz`：新的时区
    /// - `--command`：新的命令
    /// - `--name`：新的任务名称
    ///
    /// # 示例
    ///
    /// ```bash
    /// # 更新 Cron 表达式
    /// vibewindow cron update <task-id> --expression '0 8 * * *'
    ///
    /// # 更新时区和名称
    /// vibewindow cron update <task-id> --tz Europe/London --name 'Morning check'
    ///
    /// # 更新命令
    /// vibewindow cron update <task-id> --command 'Updated message'
    /// ```
    #[command(long_about = "\
更新现有定时任务的一个或多个字段。

只更新指定的字段；其他字段保持不变。

示例：
  vibewindow cron update <task-id> --expression '0 8 * * *'
  vibewindow cron update <task-id> --tz Europe/London --name 'Morning check'
  vibewindow cron update <task-id> --command 'Updated message'")]
    Update {
        /// 任务 ID
        id: String,

        /// 新的 Cron 表达式
        #[arg(long)]
        expression: Option<String>,

        /// 新的 IANA 时区名称
        #[arg(long)]
        tz: Option<String>,

        /// 新的要执行的命令
        #[arg(long)]
        command: Option<String>,

        /// 新的任务名称
        #[arg(long)]
        name: Option<String>,
    },

    /// 暂停已调度的任务
    ///
    /// 临时停止任务的执行，但保留任务配置。
    /// 可以通过 `resume` 命令恢复执行。
    Pause {
        /// 任务 ID
        id: String,
    },

    /// 恢复已暂停的任务
    ///
    /// 恢复之前暂停的任务的执行。
    Resume {
        /// 任务 ID
        id: String,
    },
}

/// 记忆管理子命令
///
/// 提供代理记忆系统的查看、搜索、统计和清理功能。
/// 记忆系统存储代理的长期知识和对话上下文。
///
/// # 记忆类别
///
/// - **core**：核心记忆，持久化的重要信息
/// - **daily**：日常记忆，按日期组织的短期信息
/// - **conversation**：对话记忆，会话上下文
/// - 自定义类别：用户定义的记忆分类
///
/// # 后端支持
///
/// - Markdown 文件
/// - SQLite 数据库
/// - PostgreSQL 数据库
/// - 向量数据库
///
/// # 示例
///
/// ```bash
/// # 列出所有记忆
/// vibewindow memory list
///
/// # 按类别过滤
/// vibewindow memory list --category core
///
/// # 获取特定记忆
/// vibewindow memory get user_preferences
///
/// # 查看统计信息
/// vibewindow memory stats
///
/// # 清理记忆
/// vibewindow memory clear --category daily --yes
/// ```
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryCommands {
    /// 列出记忆条目（支持过滤）
    ///
    /// 显示记忆条目列表，可通过类别、会话等条件过滤。
    /// 支持分页查询。
    ///
    /// # 过滤选项
    ///
    /// - `--category`：按类别过滤（core、daily、conversation 或自定义）
    /// - `--session`：按会话 ID 过滤
    /// - `--limit`：返回的最大条目数
    /// - `--offset`：跳过的条目数（用于分页）
    ///
    /// # 示例
    ///
    /// ```bash
    /// # 列出前 50 条记忆
    /// vibewindow memory list
    ///
    /// # 列出核心记忆
    /// vibewindow memory list --category core
    ///
    /// # 分页查询
    /// vibewindow memory list --limit 20 --offset 40
    /// ```
    List {
        /// 按类别过滤
        ///
        /// 支持的值：core、daily、conversation 或自定义类别名称
        #[arg(long)]
        category: Option<String>,

        /// 按会话 ID 过滤
        #[arg(long)]
        session: Option<String>,

        /// 显示的最大条目数
        #[arg(long, default_value = "50")]
        limit: usize,

        /// 跳过的条目数（用于分页）
        #[arg(long, default_value = "0")]
        offset: usize,
    },

    /// 根据键获取特定的记忆条目
    ///
    /// 查找并显示指定键对应的记忆内容。
    ///
    /// # 参数
    ///
    /// - `key`：记忆的键名，支持精确匹配
    ///
    /// # 示例
    ///
    /// ```bash
    /// vibewindow memory get user_preferences
    /// ```
    Get {
        /// 要查找的记忆键
        key: String,
    },

    /// 显示记忆后端统计信息和健康状况
    ///
    /// 返回记忆系统的使用统计、存储健康状况和性能指标。
    Stats,

    /// 清理记忆条目
    ///
    /// 按键、类别或清理全部记忆。支持确认提示。
    ///
    /// # 清理选项
    ///
    /// - `--key`：删除单个条目（支持前缀匹配）
    /// - `--category`：仅清理指定类别的条目
    /// - `--yes`：跳过确认提示
    ///
    /// # 警告
    ///
    /// 不带参数执行将清理所有记忆，此操作不可逆！
    ///
    /// # 示例
    ///
    /// ```bash
    /// # 删除单个条目
    /// vibewindow memory clear --key temp_*
    ///
    /// # 清理日常记忆
    /// vibewindow memory clear --category daily --yes
    ///
    /// # 清理所有记忆（危险操作！）
    /// vibewindow memory clear --yes
    /// ```
    Clear {
        /// 根据键删除单个条目（支持前缀匹配）
        #[arg(long)]
        key: Option<String>,

        /// 仅清理指定类别的条目
        #[arg(long)]
        category: Option<String>,

        /// 跳过确认提示
        #[arg(long)]
        yes: bool,
    },
}

/// 集成管理子命令
///
/// 提供第三方集成的发现、搜索和信息查询功能。
/// 集成扩展了代理与外部服务交互的能力。
///
/// # 集成类别
///
/// - **chat**：聊天平台集成
/// - **ai**：AI 服务集成
/// - **productivity**：生产力工具集成
/// - 更多类别...
///
/// # 集成状态
///
/// - **active**：已激活可用
/// - **available**：可用但未激活
/// - **coming-soon**：即将推出
///
/// # 示例
///
/// ```bash
/// # 列出所有集成
/// vibewindow integration list
///
/// # 按类别过滤
/// vibewindow integration list --category chat
///
/// # 按状态过滤
/// vibewindow integration list --status active
///
/// # 搜索集成
/// vibewindow integration search slack
///
/// # 查看集成详情
/// vibewindow integration info slack
/// ```
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntegrationCommands {
    /// 列出所有集成（支持按类别或状态过滤）
    ///
    /// 显示可用的集成列表及其基本信息。
    ///
    /// # 过滤选项
    ///
    /// - `-c, --category`：按类别过滤（如 "chat"、"ai"、"productivity"）
    /// - `-s, --status`：按状态过滤（active、available、coming-soon）
    ///
    /// # 示例
    ///
    /// ```bash
    /// # 列出所有集成
    /// vibewindow integration list
    ///
    /// # 列出聊天类集成
    /// vibewindow integration list --category chat
    ///
    /// # 列出已激活的集成
    /// vibewindow integration list --status active
    /// ```
    List {
        /// 按类别过滤
        ///
        /// 例如：chat、ai、productivity
        #[arg(long, short)]
        category: Option<String>,

        /// 按状态过滤
        ///
        /// 支持的值：active（已激活）、available（可用）、coming-soon（即将推出）
        #[arg(long, short)]
        status: Option<String>,
    },

    /// 根据关键词搜索集成
    ///
    /// 在集成名称和描述中搜索匹配的关键词。
    ///
    /// # 参数
    ///
    /// - `query`：搜索关键词
    ///
    /// # 示例
    ///
    /// ```bash
    /// vibewindow integration search slack
    /// ```
    Search {
        /// 搜索关键词
        ///
        /// 将匹配集成名称和描述
        query: String,
    },

    /// 显示特定集成的详细信息
    ///
    /// 查看集成的配置要求、功能说明和使用示例。
    ///
    /// # 参数
    ///
    /// - `name`：集成名称
    ///
    /// # 示例
    ///
    /// ```bash
    /// vibewindow integration info slack
    /// ```
    Info {
        /// 集成名称
        name: String,
    },
}

//! # 安全审计日志模块
//!
//! 本模块提供安全事件的审计日志记录功能，用于追踪和记录系统中发生的各类安全相关操作。
//!
//! ## 主要功能
//!
//! - **事件记录**：记录命令执行、文件访问、配置变更、认证事件等安全相关操作
//! - **结构化日志**：使用 JSON 格式存储审计事件，便于后续分析和查询
//! - **日志轮转**：支持基于文件大小的自动日志轮转，防止单个日志文件过大
//! - **构建器模式**：通过链式调用灵活构建审计事件
//!
//! ## 核心组件
//!
//! - [`AuditEvent`]：完整的审计事件，包含时间戳、事件ID、执行者、动作、结果等信息
//! - [`AuditLogger`]：审计日志记录器，负责将事件持久化到文件
//! - [`AuditEventType`]：审计事件类型枚举
//!
//! ## 使用示例
//!
//! ```ignore
//! use security_audit::{AuditLogger, AuditEvent, AuditEventType};
//!
//! // 创建审计事件
//! let event = AuditEvent::new(AuditEventType::CommandExecution)
//!     .with_actor("telegram".to_string(), Some("user123".to_string()), None)
//!     .with_action("ls -la".to_string(), "low".to_string(), true, true)
//!     .with_result(true, Some(0), 100, None);
//!
//! // 记录事件
//! logger.log(&event)?;
//! ```

use config_security::AuditConfig;
use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

/// 审计事件类型枚举
///
/// 定义了系统中所有可被审计的事件类型，用于分类和筛选审计日志。
///
/// # 变体说明
///
/// - `CommandExecution`：命令执行事件，记录 Shell 命令的执行情况
/// - `FileAccess`：文件访问事件，记录对文件系统的读写操作
/// - `ConfigChange`：配置变更事件，记录系统配置的修改操作
/// - `AuthSuccess`：认证成功事件，记录成功的身份验证
/// - `AuthFailure`：认证失败事件，记录失败的身份验证尝试
/// - `PolicyViolation`：策略违规事件，记录违反安全策略的行为
/// - `SecurityEvent`：通用安全事件，记录其他安全相关的操作
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// 命令执行事件
    CommandExecution,
    /// 文件访问事件
    FileAccess,
    /// 配置变更事件
    ConfigChange,
    /// 认证成功事件
    AuthSuccess,
    /// 认证失败事件
    AuthFailure,
    /// 策略违规事件
    PolicyViolation,
    /// 通用安全事件
    SecurityEvent,
}

/// 执行者信息结构体
///
/// 记录执行操作的主体信息，包括来源渠道、用户ID和用户名。
/// 用于追踪"谁"执行了某个操作。
///
/// # 字段说明
///
/// - `channel`：操作来源渠道（如 telegram、slack、discord 等）
/// - `user_id`：用户的唯一标识符（可选）
/// - `username`：用户的显示名称（可选）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    /// 操作来源渠道标识
    pub channel: String,
    /// 用户唯一标识符
    pub user_id: Option<String>,
    /// 用户显示名称
    pub username: Option<String>,
}

/// 动作信息结构体
///
/// 记录被执行的操作详情，包括命令内容、风险等级以及审批和允许状态。
/// 用于描述"做了什么"操作。
///
/// # 字段说明
///
/// - `command`：执行的命令或操作描述
/// - `risk_level`：操作的风险等级（如 low、medium、high、critical）
/// - `approved`：操作是否已通过审批
/// - `allowed`：操作是否被策略允许执行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// 执行的命令或操作描述
    pub command: Option<String>,
    /// 操作的风险等级
    pub risk_level: Option<String>,
    /// 是否已通过审批
    pub approved: bool,
    /// 是否被策略允许
    pub allowed: bool,
}

/// 执行结果结构体
///
/// 记录操作的执行结果，包括成功状态、退出码、执行时长和错误信息。
/// 用于描述操作"执行得如何"。
///
/// # 字段说明
///
/// - `success`：操作是否成功完成
/// - `exit_code`：进程退出码（适用于命令执行）
/// - `duration_ms`：操作执行耗时（毫秒）
/// - `error`：错误信息（如果操作失败）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// 操作是否成功
    pub success: bool,
    /// 进程退出码
    pub exit_code: Option<i32>,
    /// 执行耗时（毫秒）
    pub duration_ms: Option<u64>,
    /// 错误信息
    pub error: Option<String>,
}

/// 安全上下文结构体
///
/// 记录操作执行时的安全环境信息，包括策略违规状态、速率限制和沙箱后端。
/// 用于提供操作执行的"安全背景"信息。
///
/// # 字段说明
///
/// - `policy_violation`：操作是否违反了安全策略
/// - `rate_limit_remaining`：剩余的速率限制配额
/// - `sandbox_backend`：使用的沙箱后端（如 docker、native、wasm）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// 是否违反安全策略
    pub policy_violation: bool,
    /// 剩余速率限制配额
    pub rate_limit_remaining: Option<u32>,
    /// 沙箱后端类型
    pub sandbox_backend: Option<String>,
}

/// 完整审计事件结构体
///
/// 表示一个完整的审计日志事件，包含时间戳、事件ID、事件类型、执行者、动作、执行结果和安全上下文。
/// 所有审计事件都以 JSON 格式序列化并持久化到日志文件中。
///
/// # 字段说明
///
/// - `timestamp`：事件发生的时间戳（UTC 时间）
/// - `event_id`：事件的唯一标识符（UUID v4）
/// - `event_type`：事件类型
/// - `actor`：执行者信息（可选）
/// - `action`：动作信息（可选）
/// - `result`：执行结果（可选）
/// - `security`：安全上下文
///
/// # 使用示例
///
/// ```ignore
/// // 创建命令执行审计事件
/// let event = AuditEvent::new(AuditEventType::CommandExecution)
///     .with_actor("telegram".to_string(), Some("user123".to_string()), None)
///     .with_action("rm -rf /".to_string(), "critical".to_string(), false, false)
///     .with_result(false, None, 0, Some("Blocked by security policy".to_string()))
///     .with_security(Some("docker".to_string()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// 事件发生的时间戳（UTC）
    pub timestamp: DateTime<Utc>,
    /// 事件的唯一标识符
    pub event_id: String,
    /// 事件类型
    pub event_type: AuditEventType,
    /// 执行者信息
    pub actor: Option<Actor>,
    /// 动作信息
    pub action: Option<Action>,
    /// 执行结果
    pub result: Option<ExecutionResult>,
    /// 安全上下文
    pub security: SecurityContext,
}

impl AuditEvent {
    /// 创建新的审计事件
    ///
    /// 使用指定的事件类型创建一个新的审计事件实例。
    /// 自动生成当前时间戳和唯一的 UUID 作为事件ID。
    /// 默认初始化安全上下文为无策略违规状态。
    ///
    /// # 参数
    ///
    /// - `event_type`：审计事件类型
    ///
    /// # 返回值
    ///
    /// 返回新创建的 `AuditEvent` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let event = AuditEvent::new(AuditEventType::CommandExecution);
    /// ```
    pub fn new(event_type: AuditEventType) -> Self {
        Self {
            timestamp: Utc::now(),
            event_id: Uuid::new_v4().to_string(),
            event_type,
            actor: None,
            action: None,
            result: None,
            security: SecurityContext {
                policy_violation: false,
                rate_limit_remaining: None,
                sandbox_backend: None,
            },
        }
    }

    /// 设置执行者信息
    ///
    /// 使用构建器模式为审计事件添加执行者信息。
    ///
    /// # 参数
    ///
    /// - `channel`：操作来源渠道（如 telegram、slack）
    /// - `user_id`：用户唯一标识符（可选）
    /// - `username`：用户显示名称（可选）
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `AuditEvent` 实例，支持链式调用
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let event = AuditEvent::new(AuditEventType::AuthSuccess)
    ///     .with_actor("telegram".to_string(), Some("user123".to_string()), Some("alice".to_string()));
    /// ```
    pub fn with_actor(
        mut self,
        channel: String,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Self {
        self.actor = Some(Actor { channel, user_id, username });
        self
    }

    /// 设置动作信息
    ///
    /// 使用构建器模式为审计事件添加动作详情。
    ///
    /// # 参数
    ///
    /// - `command`：执行的命令或操作描述
    /// - `risk_level`：操作的风险等级
    /// - `approved`：操作是否已通过审批
    /// - `allowed`：操作是否被策略允许
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `AuditEvent` 实例，支持链式调用
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let event = AuditEvent::new(AuditEventType::CommandExecution)
    ///     .with_action("ls -la".to_string(), "low".to_string(), true, true);
    /// ```
    pub fn with_action(
        mut self,
        command: String,
        risk_level: String,
        approved: bool,
        allowed: bool,
    ) -> Self {
        self.action = Some(Action {
            command: Some(command),
            risk_level: Some(risk_level),
            approved,
            allowed,
        });
        self
    }

    /// 设置执行结果
    ///
    /// 使用构建器模式为审计事件添加执行结果信息。
    ///
    /// # 参数
    ///
    /// - `success`：操作是否成功完成
    /// - `exit_code`：进程退出码（可选，适用于命令执行）
    /// - `duration_ms`：操作执行耗时（毫秒）
    /// - `error`：错误信息（可选，如果操作失败）
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `AuditEvent` 实例，支持链式调用
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let event = AuditEvent::new(AuditEventType::CommandExecution)
    ///     .with_result(true, Some(0), 150, None);
    /// ```
    pub fn with_result(
        mut self,
        success: bool,
        exit_code: Option<i32>,
        duration_ms: u64,
        error: Option<String>,
    ) -> Self {
        self.result =
            Some(ExecutionResult { success, exit_code, duration_ms: Some(duration_ms), error });
        self
    }

    /// 设置安全上下文
    ///
    /// 使用构建器模式为审计事件添加安全上下文信息。
    ///
    /// # 参数
    ///
    /// - `sandbox_backend`：使用的沙箱后端类型（如 docker、native、wasm）
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `AuditEvent` 实例，支持链式调用
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let event = AuditEvent::new(AuditEventType::CommandExecution)
    ///     .with_security(Some("docker".to_string()));
    /// ```
    pub fn with_security(mut self, sandbox_backend: Option<String>) -> Self {
        self.security.sandbox_backend = sandbox_backend;
        self
    }
}

/// 审计日志记录器
///
/// 负责将审计事件持久化到文件系统，支持日志轮转和缓冲。
/// 所有审计事件以 JSON Lines 格式存储，每行一个 JSON 对象。
///
/// # 功能特性
///
/// - **自动创建目录**：如果日志文件所在目录不存在，会自动创建
/// - **日志轮转**：当日志文件超过配置的最大大小时，自动进行轮转
/// - **同步写入**：每次写入后调用 `sync_all` 确保数据持久化
/// - **可配置**：通过 `AuditConfig` 控制是否启用、日志路径、最大大小等
///
/// # 字段说明
///
/// - `log_path`：日志文件的完整路径
/// - `config`：审计配置
/// - `buffer`：事件缓冲区（用于批量写入，当前实现为单个事件写入）
pub struct AuditLogger {
    /// 日志文件路径
    log_path: PathBuf,
    /// 审计配置
    config: AuditConfig,
    /// 事件缓冲区（线程安全）
    buffer: Mutex<Vec<AuditEvent>>,
}

/// 命令执行日志结构体
///
/// 用于记录命令执行的结构化详情，作为 `log_command_event` 方法的参数。
/// 提供了一种简洁的方式来传递命令执行的所有相关信息。
///
/// # 字段说明
///
/// - `channel`：命令来源渠道
/// - `command`：执行的命令字符串
/// - `risk_level`：命令的风险等级
/// - `approved`：命令是否已通过审批
/// - `allowed`：命令是否被策略允许
/// - `success`：命令是否执行成功
/// - `duration_ms`：命令执行耗时（毫秒）
///
/// # 生命周期
///
/// 使用生命周期参数 `'a` 来避免不必要的字符串克隆，
/// 调用时可以直接传递字符串切片引用。
#[derive(Debug, Clone)]
pub struct CommandExecutionLog<'a> {
    /// 命令来源渠道
    pub channel: &'a str,
    /// 执行的命令
    pub command: &'a str,
    /// 风险等级
    pub risk_level: &'a str,
    /// 是否已审批
    pub approved: bool,
    /// 是否被允许
    pub allowed: bool,
    /// 是否成功
    pub success: bool,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
}

impl AuditLogger {
    /// 创建新的审计日志记录器
    ///
    /// 根据配置初始化审计日志记录器，如果审计功能已启用，
    /// 会确保日志文件存在（包括创建必要的目录结构）。
    ///
    /// # 参数
    ///
    /// - `config`：审计配置，包含启用状态、日志路径、最大大小等设置
    /// - `vibewindow_dir`：VibeWindow 数据目录路径，日志文件将在此目录下创建
    ///
    /// # 返回值
    ///
    /// 成功时返回 `AuditLogger` 实例，失败时返回错误
    ///
    /// # 错误
    ///
    /// - 如果无法创建日志文件所在目录，返回 IO 错误
    /// - 如果无法创建或打开日志文件，返回 IO 错误
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use config_security::AuditConfig;
    /// use std::path::PathBuf;
    ///
    /// let config = AuditConfig::default();
    /// let logger = AuditLogger::new(config, PathBuf::from("/var/lib/vibewindow"))?;
    /// ```
    pub fn new(config: AuditConfig, vibewindow_dir: PathBuf) -> Result<Self> {
        // 拼接完整的日志文件路径
        let log_path = vibewindow_dir.join(&config.log_path);

        // 如果审计功能已启用，确保日志文件存在
        if config.enabled {
            initialize_audit_log_file(&log_path)?;
        }

        Ok(Self { log_path, config, buffer: Mutex::new(Vec::new()) })
    }

    /// 记录审计事件
    ///
    /// 将审计事件序列化为 JSON 格式并追加写入日志文件。
    /// 如果审计功能未启用，此方法将直接返回成功而不执行任何操作。
    ///
    /// # 参数
    ///
    /// - `event`：要记录的审计事件引用
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误
    ///
    /// # 错误
    ///
    /// - 如果日志文件超过最大大小且轮转失败，返回 IO 错误
    /// - 如果 JSON 序列化失败，返回序列化错误
    /// - 如果无法打开或写入日志文件，返回 IO 错误
    ///
    /// # 注意
    ///
    /// - 每次写入后会调用 `sync_all` 确保数据持久化到磁盘
    /// - 写入前会检查日志文件大小并在需要时进行轮转
    pub fn log(&self, event: &AuditEvent) -> Result<()> {
        // 如果审计功能未启用，直接返回
        if !self.config.enabled {
            return Ok(());
        }

        // 检查日志文件大小，如超过限制则进行轮转
        self.rotate_if_needed()?;

        // 确保日志文件存在（包括创建目录）
        initialize_audit_log_file(&self.log_path)?;

        // 将事件序列化为 JSON 字符串
        let line = serde_json::to_string(event)?;

        // 以追加模式打开日志文件并写入
        let mut file = OpenOptions::new().create(true).append(true).open(&self.log_path)?;

        // 写入 JSON 行
        writeln!(file, "{}", line)?;

        // 同步到磁盘，确保数据持久化
        file.sync_all()?;

        Ok(())
    }

    /// 记录命令执行事件
    ///
    /// 便捷方法，用于记录命令执行的审计事件。
    /// 内部会构造完整的 `AuditEvent` 并调用 `log` 方法。
    ///
    /// # 参数
    ///
    /// - `entry`：命令执行日志结构体，包含所有必要信息
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let entry = CommandExecutionLog {
    ///     channel: "telegram",
    ///     command: "ls -la",
    ///     risk_level: "low",
    ///     approved: true,
    ///     allowed: true,
    ///     success: true,
    ///     duration_ms: 50,
    /// };
    /// logger.log_command_event(entry)?;
    /// ```
    pub fn log_command_event(&self, entry: CommandExecutionLog<'_>) -> Result<()> {
        // 构造命令执行审计事件
        let event = AuditEvent::new(AuditEventType::CommandExecution)
            .with_actor(entry.channel.to_string(), None, None)
            .with_action(
                entry.command.to_string(),
                entry.risk_level.to_string(),
                entry.approved,
                entry.allowed,
            )
            .with_result(entry.success, None, entry.duration_ms, None);

        // 记录事件
        self.log(&event)
    }

    /// 向后兼容的命令执行日志记录方法
    ///
    /// 提供向后兼容的接口，使用独立参数而非结构体。
    /// 内部会构造 `CommandExecutionLog` 并调用 `log_command_event`。
    ///
    /// # 参数
    ///
    /// - `channel`：命令来源渠道
    /// - `command`：执行的命令
    /// - `risk_level`：风险等级
    /// - `approved`：是否已审批
    /// - `allowed`：是否被允许
    /// - `success`：是否成功
    /// - `duration_ms`：执行耗时（毫秒）
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误
    ///
    /// # 注意
    ///
    /// 此方法使用 `#[allow(clippy::too_many_arguments)]` 因为需要保持向后兼容性。
    /// 推荐使用 `log_command_event` 方法替代。
    #[allow(clippy::too_many_arguments)]
    pub fn log_command(
        &self,
        channel: &str,
        command: &str,
        risk_level: &str,
        approved: bool,
        allowed: bool,
        success: bool,
        duration_ms: u64,
    ) -> Result<()> {
        // 构造命令执行日志并记录
        self.log_command_event(CommandExecutionLog {
            channel,
            command,
            risk_level,
            approved,
            allowed,
            success,
            duration_ms,
        })
    }

    /// 检查并在需要时轮转日志文件
    ///
    /// 检查当前日志文件大小，如果超过配置的最大大小则触发轮转。
    /// 轮转采用数字后缀命名方式，保留最多 10 个历史文件。
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误
    ///
    /// # 错误
    ///
    /// - 如果无法读取日志文件元数据，忽略错误继续执行
    /// - 如果轮转过程中发生 IO 错误，返回该错误
    fn rotate_if_needed(&self) -> Result<()> {
        // 尝试获取日志文件元数据
        if let Ok(metadata) = std::fs::metadata(&self.log_path) {
            // 计算当前文件大小（MB）
            let current_size_mb = metadata.len() / (1024 * 1024);

            // 如果超过配置的最大大小，触发轮转
            if current_size_mb >= u64::from(self.config.max_size_mb) {
                self.rotate()?;
            }
        }
        Ok(())
    }

    /// 执行日志文件轮转
    ///
    /// 将现有的日志文件进行轮转，采用数字后缀命名策略：
    /// - `audit.log` -> `audit.log.1.log`
    /// - `audit.log.1.log` -> `audit.log.2.log`
    /// - 以此类推，最多保留 10 个历史文件
    ///
    /// # 轮转逻辑
    ///
    /// 1. 从编号 9 开始向前遍历到 1，将每个文件重命名为编号 +1
    /// 2. 将当前日志文件重命名为 `audit.log.1.log`
    /// 3. 这样可以保留最近 10 个历史文件
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误
    ///
    /// # 错误
    ///
    /// - 如果无法重命名日志文件，返回 IO 错误
    fn rotate(&self) -> Result<()> {
        // 从后向前移动历史文件（9 -> 10, 8 -> 9, ..., 1 -> 2）
        // 使用 rev() 确保从大到小编号处理，避免覆盖
        for i in (1..10).rev() {
            let old_name = format!("{}.{}.log", self.log_path.display(), i);
            let new_name = format!("{}.{}.log", self.log_path.display(), i + 1);
            // 忽略重命名错误（文件可能不存在）
            let _ = std::fs::rename(&old_name, &new_name);
        }

        // 将当前日志文件重命名为编号 1
        let rotated = format!("{}.1.log", self.log_path.display());
        std::fs::rename(&self.log_path, &rotated)?;
        Ok(())
    }
}

/// 初始化审计日志文件
///
/// 确保审计日志文件存在，包括创建必要的目录结构。
/// 如果文件已存在，此函数不会修改文件内容。
///
/// # 参数
///
/// - `log_path`：日志文件的路径
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误
///
/// # 错误
///
/// - 如果无法创建父目录，返回 IO 错误
/// - 如果无法创建或打开文件，返回 IO 错误
///
/// # 实现细节
///
/// 1. 检查日志路径是否有父目录
/// 2. 如果有父目录且路径不为空，创建所有必要的目录
/// 3. 以追加模式打开（或创建）日志文件
fn initialize_audit_log_file(log_path: &std::path::Path) -> Result<()> {
    // 获取父目录路径
    if let Some(parent) = log_path.parent() {
        // 如果父目录路径不为空，创建目录结构
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    // 以追加模式打开或创建日志文件
    // create(true) 表示如果文件不存在则创建
    // append(true) 表示以追加模式打开
    let _ = OpenOptions::new().create(true).append(true).open(log_path)?;
    Ok(())
}

#[cfg(test)]
#[path = "audit_tests.rs"]
mod audit_tests;

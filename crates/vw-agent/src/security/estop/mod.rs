//! 紧急停止（E-Stop）系统模块
//!
//! 本模块实现了一个高可靠性的紧急停止系统，用于在检测到异常情况时快速、安全地
//! 限制或停止代理的运行。紧急停止系统采用"失败即关闭"（fail-closed）的安全策略，
//! 确保在状态文件损坏或配置错误时，系统默认进入最安全的锁定状态。
//!
//! # 核心功能
//!
//! - **多级紧急停止**：支持全局终止、网络隔离、域名阻断、工具冻结等多种停止级别
//! - **持久化状态管理**：将紧急停止状态持久化到磁盘，确保重启后状态可恢复
//! - **安全恢复机制**：支持 OTP（一次性密码）验证的恢复机制，防止未授权恢复
//! - **原子性写入**：使用临时文件+重命名的方式保证状态文件的原子性更新
//!
//! # 安全策略
//!
//! - 状态文件读取失败时，进入 fail-closed 模式（全局终止激活）
//! - 状态文件解析失败时，进入 fail-closed 模式（全局终止激活）
//! - Unix 系统上设置状态文件权限为 0o600（仅所有者可读写）
//! - 恢复操作可选择要求 OTP 验证，防止未授权恢复
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use crate::app::agent::security::estop::{EstopManager, EstopLevel};
//! use crate::app::agent::config::EstopConfig;
//!
//! // 加载紧急停止管理器
//! let config = EstopConfig::default();
//! let mut manager = EstopManager::load(&config, config_dir)?;
//!
//! // 激活全局紧急停止
//! manager.engage(EstopLevel::KillAll)?;
//!
//! // 检查状态
//! if manager.status().is_engaged() {
//!     println!("紧急停止已激活");
//! }
//!
//! // 恢复运行（可能需要 OTP）
//! manager.resume(ResumeSelector::KillAll, Some("123456"), Some(&otp_validator))?;
//! ```

use super::domain_matcher::DomainMatcher;
use super::otp::OtpValidator;
use crate::app::agent::config::EstopConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 紧急停止级别枚举
///
/// 定义不同严重程度的紧急停止级别，用于控制代理的不同方面。
/// 级别从高到低依次为：KillAll > NetworkKill > DomainBlock/ToolFreeze
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EstopLevel {
    /// 全局终止 - 停止所有代理活动
    ///
    /// 最高级别的紧急停止，立即停止所有代理的所有活动。
    /// 这是最严格的安全措施，通常用于检测到严重安全威胁时。
    KillAll,

    /// 网络隔离 - 切断所有网络连接
    ///
    /// 禁止所有网络通信，但允许本地操作继续进行。
    /// 适用于检测到网络层面的安全威胁或异常行为。
    NetworkKill,

    /// 域名阻断 - 阻止访问指定域名列表
    ///
    /// 仅阻止对指定域名的访问，其他网络活动正常。
    /// 适用于检测到特定域名的安全风险或策略违规。
    ///
    /// # 参数
    /// - `Vec<String>`: 要阻断的域名列表（自动转换为小写并验证格式）
    DomainBlock(Vec<String>),

    /// 工具冻结 - 禁用指定工具列表
    ///
    /// 禁用指定的工具，防止其被调用执行。
    /// 适用于检测到特定工具的安全风险或滥用行为。
    ///
    /// # 参数
    /// - `Vec<String>`: 要冻结的工具名称列表（仅允许字母数字、下划线和连字符）
    ToolFreeze(Vec<String>),
}

/// 恢复操作选择器枚举
///
/// 用于指定要恢复（解除）哪个级别的紧急停止。
/// 与 [`EstopLevel`] 相对应，但用于恢复操作而非激活操作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResumeSelector {
    /// 恢复全局终止状态
    ///
    /// 解除 KillAll 紧急停止，允许所有代理活动恢复。
    KillAll,

    /// 恢复网络隔离状态
    ///
    /// 解除 NetworkKill 紧急停止，恢复网络连接。
    Network,

    /// 恢复指定域名的访问权限
    ///
    /// 从域名阻断列表中移除指定的域名，恢复对这些域名的访问。
    ///
    /// # 参数
    /// - `Vec<String>`: 要恢复访问的域名列表
    Domains(Vec<String>),

    /// 解冻指定工具
    ///
    /// 从工具冻结列表中移除指定的工具，允许这些工具被调用。
    ///
    /// # 参数
    /// - `Vec<String>`: 要解冻的工具名称列表
    Tools(Vec<String>),
}

/// 紧急停止状态结构体
///
/// 表示当前紧急停止系统的完整状态，包括所有激活的停止级别和最后更新时间。
/// 该结构体会被持久化到磁盘，确保重启后状态可恢复。
///
/// # 序列化
///
/// 使用 JSON 格式进行序列化，支持人类可读的状态检查。
/// 所有字段都有默认值，确保向后兼容性。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct EstopState {
    /// 全局终止标志 - 为 true 时停止所有代理活动
    #[serde(default)]
    pub kill_all: bool,

    /// 网络隔离标志 - 为 true 时切断所有网络连接
    #[serde(default)]
    pub network_kill: bool,

    /// 已阻断的域名列表 - 这些域名的访问将被阻止
    #[serde(default)]
    pub blocked_domains: Vec<String>,

    /// 已冻结的工具列表 - 这些工具的调用将被拒绝
    #[serde(default)]
    pub frozen_tools: Vec<String>,

    /// 最后更新时间（RFC3339 格式）- 记录状态变更的时间戳
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl EstopState {
    /// 创建 fail-closed 状态（失败即关闭的安全默认状态）
    ///
    /// 当状态文件损坏、读取失败或解析错误时使用此状态。
    /// 这是一个安全优先的默认状态，会激活全局终止以确保系统安全。
    ///
    /// # 返回值
    ///
    /// 返回一个 `EstopState` 实例，其中：
    /// - `kill_all` 设置为 `true`（全局终止激活）
    /// - `network_kill` 设置为 `false`（避免过度限制）
    /// - `blocked_domains` 和 `frozen_tools` 为空列表
    /// - `updated_at` 设置为当前时间
    ///
    /// # 设计理念
    ///
    /// 采用 fail-closed 策略而非 fail-open 策略，因为：
    /// - 在安全关键系统中，宁可误报也不可漏报
    /// - 状态文件损坏可能意味着系统已被入侵或文件系统异常
    /// - 全局终止是最安全的选择，可通过验证后恢复
    pub fn fail_closed() -> Self {
        Self {
            kill_all: true,
            network_kill: false,
            blocked_domains: Vec::new(),
            frozen_tools: Vec::new(),
            updated_at: Some(now_rfc3339()),
        }
    }

    /// 检查是否有任何紧急停止级别处于激活状态
    ///
    /// # 返回值
    ///
    /// - `true`: 至少有一个紧急停止级别处于激活状态
    /// - `false`: 所有紧急停止级别都已解除，系统正常运行
    ///
    /// # 判断逻辑
    ///
    /// 只要满足以下任一条件，即认为紧急停止已激活：
    /// - 全局终止标志为 true
    /// - 网络隔离标志为 true
    /// - 域名阻断列表不为空
    /// - 工具冻结列表不为空
    pub fn is_engaged(&self) -> bool {
        self.kill_all
            || self.network_kill
            || !self.blocked_domains.is_empty()
            || !self.frozen_tools.is_empty()
    }

    /// 规范化状态数据
    ///
    /// 对域名列表和工具列表进行去重和排序，确保状态的一致性和可预测性。
    /// 这有助于状态比较、差异检测和调试。
    fn normalize(&mut self) {
        self.blocked_domains = dedup_sort(&self.blocked_domains);
        self.frozen_tools = dedup_sort(&self.frozen_tools);
    }
}

/// 紧急停止管理器
///
/// 负责管理紧急停止系统的核心组件，提供状态的加载、持久化、激活和恢复功能。
/// 该管理器遵循"失败即关闭"的安全原则，确保在任何异常情况下都能保持系统安全。
#[derive(Debug, Clone)]
pub struct EstopManager {
    /// 紧急停止配置
    config: EstopConfig,

    /// 状态文件路径
    state_path: PathBuf,

    /// 当前紧急停止状态
    state: EstopState,
}

impl EstopManager {
    /// 从配置加载紧急停止管理器
    ///
    /// 根据配置文件初始化紧急停止管理器，并从磁盘加载持久化状态。
    /// 如果状态文件不存在，则创建默认状态；如果状态文件损坏，则进入 fail-closed 模式。
    ///
    /// # 参数
    ///
    /// - `config`: 紧急停止配置引用
    /// - `config_dir`: 配置文件所在目录，用于解析相对路径
    ///
    /// # 返回值
    ///
    /// 成功时返回初始化完成的 `EstopManager` 实例
    ///
    /// # 错误
    ///
    /// 该函数不会返回错误，即使在状态文件读取或解析失败时也会返回管理器实例，
    /// 此时管理器会处于 fail-closed 模式（全局终止激活）。
    ///
    /// # 加载逻辑
    ///
    /// 1. 解析状态文件路径（支持 `~` 展开和相对路径）
    /// 2. 如果状态文件存在：
    ///    - 读取并解析 JSON 内容
    ///    - 解析失败时进入 fail-closed 模式并记录警告日志
    /// 3. 如果状态文件不存在：
    ///    - 使用默认状态（所有停止级别均未激活）
    /// 4. 规范化状态数据（去重、排序）
    /// 5. 如果进入了 fail-closed 模式，持久化当前状态到磁盘
    pub fn load(config: &EstopConfig, config_dir: &Path) -> Result<Self> {
        let state_path = resolve_state_file_path(config_dir, &config.state_file);
        let mut should_fail_closed = false;

        // 尝试加载现有状态文件
        let mut state = if state_path.exists() {
            match fs::read_to_string(&state_path) {
                Ok(raw) => match serde_json::from_str::<EstopState>(&raw) {
                    Ok(mut parsed) => {
                        parsed.normalize();
                        parsed
                    }
                    Err(error) => {
                        // 状态文件解析失败 - 进入 fail-closed 模式
                        tracing::warn!(
                            path = %state_path.display(),
                            "Failed to parse estop state file; entering fail-closed mode: {error}"
                        );
                        should_fail_closed = true;
                        EstopState::fail_closed()
                    }
                },
                Err(error) => {
                    // 状态文件读取失败 - 进入 fail-closed 模式
                    tracing::warn!(
                        path = %state_path.display(),
                        "Failed to read estop state file; entering fail-closed mode: {error}"
                    );
                    should_fail_closed = true;
                    EstopState::fail_closed()
                }
            }
        } else {
            // 状态文件不存在 - 使用默认状态
            EstopState::default()
        };

        state.normalize();

        let mut manager = Self { config: config.clone(), state_path, state };

        // 如果进入了 fail-closed 模式，持久化状态以确保可追溯
        if should_fail_closed {
            let _ = manager.persist_state();
        }

        Ok(manager)
    }

    /// 获取状态文件路径
    ///
    /// # 返回值
    ///
    /// 返回状态文件的完整路径引用
    pub fn state_path(&self) -> &Path {
        &self.state_path
    }

    /// 获取当前紧急停止状态的快照
    ///
    /// # 返回值
    ///
    /// 返回当前状态的克隆副本，调用者可以安全地修改而不影响管理器内部状态
    pub fn status(&self) -> EstopState {
        self.state.clone()
    }

    /// 激活指定级别的紧急停止
    ///
    /// 根据指定的紧急停止级别，更新系统状态并持久化到磁盘。
    ///
    /// # 参数
    ///
    /// - `level`: 要激活的紧急停止级别
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`
    ///
    /// # 错误
    ///
    /// - 域名格式验证失败时返回错误
    /// - 工具名称格式验证失败时返回错误
    /// - 状态持久化失败时返回错误
    ///
    /// # 处理逻辑
    ///
    /// 根据 `level` 的不同变体执行不同操作：
    /// - `KillAll`: 设置全局终止标志
    /// - `NetworkKill`: 设置网络隔离标志
    /// - `DomainBlock`: 验证并添加域名到阻断列表
    /// - `ToolFreeze`: 验证并添加工具到冻结列表
    ///
    /// 所有操作都会：
    /// 1. 更新 `updated_at` 时间戳
    /// 2. 规范化状态数据
    /// 3. 持久化状态到磁盘
    pub fn engage(&mut self, level: EstopLevel) -> Result<()> {
        match level {
            EstopLevel::KillAll => {
                self.state.kill_all = true;
            }
            EstopLevel::NetworkKill => {
                self.state.network_kill = true;
            }
            EstopLevel::DomainBlock(domains) => {
                for domain in domains {
                    // 规范化域名：去除首尾空白并转换为小写
                    let normalized = domain.trim().to_ascii_lowercase();
                    // 验证域名格式是否符合匹配规则
                    DomainMatcher::validate_pattern(&normalized)?;
                    self.state.blocked_domains.push(normalized);
                }
            }
            EstopLevel::ToolFreeze(tools) => {
                for tool in tools {
                    // 规范化并验证工具名称
                    let normalized = normalize_tool_name(&tool)?;
                    self.state.frozen_tools.push(normalized);
                }
            }
        }

        self.state.updated_at = Some(now_rfc3339());
        self.state.normalize();
        self.persist_state()
    }

    /// 恢复（解除）指定级别的紧急停止
    ///
    /// 解除指定的紧急停止级别，并持久化更新后的状态。
    /// 如果配置要求 OTP 验证，则在执行恢复前验证 OTP 代码。
    ///
    /// # 参数
    ///
    /// - `selector`: 指定要恢复的紧急停止级别
    /// - `otp_code`: 可选的 OTP 验证码（配置要求时必须提供）
    /// - `otp_validator`: 可选的 OTP 验证器（配置要求时必须提供）
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`
    ///
    /// # 错误
    ///
    /// - OTP 验证失败时返回错误
    /// - 未提供必需的 OTP 代码或验证器时返回错误
    /// - 工具名称格式验证失败时返回错误
    /// - 状态持久化失败时返回错误
    ///
    /// # 处理逻辑
    ///
    /// 1. 首先验证恢复操作是否被授权（可能需要 OTP）
    /// 2. 根据 `selector` 的不同变体执行不同的恢复操作：
    ///    - `KillAll`: 清除全局终止标志
    ///    - `Network`: 清除网络隔离标志
    ///    - `Domains`: 从阻断列表中移除指定域名
    ///    - `Tools`: 从冻结列表中移除指定工具
    /// 3. 更新时间戳、规范化状态并持久化
    pub fn resume(
        &mut self,
        selector: ResumeSelector,
        otp_code: Option<&str>,
        otp_validator: Option<&OtpValidator>,
    ) -> Result<()> {
        self.ensure_resume_is_authorized(otp_code, otp_validator)?;

        match selector {
            ResumeSelector::KillAll => {
                self.state.kill_all = false;
            }
            ResumeSelector::Network => {
                self.state.network_kill = false;
            }
            ResumeSelector::Domains(domains) => {
                // 规范化要恢复的域名列表
                let normalized = domains
                    .iter()
                    .map(|domain| domain.trim().to_ascii_lowercase())
                    .collect::<Vec<_>>();
                // 从阻断列表中移除匹配的域名
                self.state
                    .blocked_domains
                    .retain(|existing| !normalized.iter().any(|target| target == existing));
            }
            ResumeSelector::Tools(tools) => {
                // 规范化并验证要解冻的工具名称
                let normalized = tools
                    .iter()
                    .map(|tool| normalize_tool_name(tool))
                    .collect::<Result<Vec<_>>>()?;
                // 从冻结列表中移除匹配的工具
                self.state
                    .frozen_tools
                    .retain(|existing| !normalized.iter().any(|target| target == existing));
            }
        }

        self.state.updated_at = Some(now_rfc3339());
        self.state.normalize();
        self.persist_state()
    }

    /// 确保恢复操作已被授权
    ///
    /// 如果配置要求 OTP 验证，则验证提供的 OTP 代码是否有效。
    /// 这是一个内部安全检查方法，防止未授权的恢复操作。
    ///
    /// # 参数
    ///
    /// - `otp_code`: 可选的 OTP 验证码
    /// - `otp_validator`: 可选的 OTP 验证器
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 恢复操作已授权
    ///
    /// # 错误
    ///
    /// - 配置要求 OTP 但未提供代码时返回错误
    /// - 配置要求 OTP 但未提供验证器时返回错误
    /// - OTP 验证失败时返回错误
    fn ensure_resume_is_authorized(
        &self,
        otp_code: Option<&str>,
        otp_validator: Option<&OtpValidator>,
    ) -> Result<()> {
        // 如果配置不要求 OTP，直接允许恢复
        if !self.config.require_otp_to_resume {
            return Ok(());
        }

        // 提取并清理 OTP 代码
        let code = otp_code
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .context("OTP code is required to resume estop state")?;

        // 获取 OTP 验证器
        let validator = otp_validator
            .context("OTP validator is required to resume estop state with OTP enabled")?;

        // 验证 OTP 代码
        let valid = validator.validate(code)?;
        if !valid {
            anyhow::bail!("Invalid OTP code; estop resume denied");
        }
        Ok(())
    }

    /// 持久化当前状态到磁盘
    ///
    /// 将当前紧急停止状态以 JSON 格式写入状态文件。
    /// 使用原子性写入策略（临时文件+重命名）确保状态文件的完整性。
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`
    ///
    /// # 错误
    ///
    /// - 创建状态文件目录失败时返回错误
    /// - 序列化状态失败时返回错误
    /// - 写入临时文件失败时返回错误
    /// - 重命名文件失败时返回错误
    ///
    /// # 原子性保证
    ///
    /// 1. 先写入到一个带 UUID 的临时文件
    /// 2. 在 Unix 系统上设置文件权限为 0o600（仅所有者可读写）
    /// 3. 使用 `rename` 系统调用原子性地替换目标文件
    ///
    /// 这种方式确保即使在写入过程中崩溃，也不会损坏原有的状态文件。
    fn persist_state(&mut self) -> Result<()> {
        // 确保状态文件目录存在
        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create estop state dir {}", parent.display())
            })?;
        }

        // 序列化状态为格式化的 JSON
        let body =
            serde_json::to_string_pretty(&self.state).context("Failed to serialize estop state")?;

        // 生成临时文件路径（使用 UUID 避免冲突）
        let temp_path = self.state_path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
        fs::write(&temp_path, body).with_context(|| {
            format!("Failed to write temporary estop state file {}", temp_path.display())
        })?;

        // 在 Unix 系统上设置严格的文件权限（仅所有者可读写）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600));
        }

        // 原子性地替换状态文件
        fs::rename(&temp_path, &self.state_path).with_context(|| {
            format!("Failed to atomically replace estop state file {}", self.state_path.display())
        })?;

        Ok(())
    }
}

/// 解析状态文件的完整路径
///
/// 根据配置目录和状态文件路径配置，解析出状态文件的完整路径。
/// 支持 `~` 符号展开（用户主目录）和相对路径解析。
///
/// # 参数
///
/// - `config_dir`: 配置文件所在目录，用于解析相对路径
/// - `state_file`: 状态文件路径配置（可以是绝对路径或相对路径）
///
/// # 返回值
///
/// 返回解析后的完整文件路径：
/// - 如果 `state_file` 是绝对路径，直接返回
/// - 如果是相对路径，相对于 `config_dir` 解析
/// - 非 WASM 目标：支持 `~` 展开为用户主目录
/// - WASM 目标：不进行 `~` 展开
///
/// # 示例
///
/// ```rust,ignore
/// let config_dir = Path::new("/etc/vibewindow");
/// let path = resolve_state_file_path(config_dir, "~/.shiju-agent/estop.json");
/// // 在 Unix 系统上返回：/home/user/.shiju-agent/estop.json
///
/// let path = resolve_state_file_path(config_dir, "estop.json");
/// // 返回：/etc/vibewindow/estop.json
/// ```
pub fn resolve_state_file_path(config_dir: &Path, state_file: &str) -> PathBuf {
    // 在非 WASM 目标上展开 `~` 符号
    #[cfg(not(target_arch = "wasm32"))]
    let expanded = shellexpand::tilde(state_file).into_owned();
    #[cfg(target_arch = "wasm32")]
    let expanded = state_file.to_string();

    let path = PathBuf::from(expanded);
    // 如果是绝对路径则直接使用，否则相对于配置目录解析
    if path.is_absolute() { path } else { config_dir.join(path) }
}

/// 规范化并验证工具名称
///
/// 对工具名称进行清理和格式验证，确保符合工具标识符的命名规范。
///
/// # 参数
///
/// - `raw`: 原始工具名称字符串
///
/// # 返回值
///
/// 成功时返回规范化后的工具名称（小写、去除首尾空白）
///
/// # 错误
///
/// - 工具名称为空或仅包含空白时返回错误
/// - 工具名称包含非法字符（非字母数字、下划线、连字符）时返回错误
///
/// # 规范化规则
///
/// 1. 去除首尾空白字符
/// 2. 转换为小写
/// 3. 验证仅包含：字母、数字、下划线(_)、连字符(-)
fn normalize_tool_name(raw: &str) -> Result<String> {
    let value = raw.trim().to_ascii_lowercase();
    if value.is_empty() {
        anyhow::bail!("Tool name must not be empty");
    }
    if !value.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-') {
        anyhow::bail!("Tool name '{raw}' contains invalid characters");
    }
    Ok(value)
}

/// 对字符串列表进行去重和排序
///
/// 对字符串数组进行规范化处理：去除空白字符、过滤空字符串、去重并排序。
/// 这确保了列表的一致性，便于比较和存储。
///
/// # 参数
///
/// - `values`: 原始字符串数组引用
///
/// # 返回值
///
/// 返回处理后的新向量：
/// - 所有元素已去除首尾空白
/// - 所有空字符串已过滤
/// - 已排序（不稳定排序，性能更优）
/// - 已去重（仅保留唯一值）
fn dedup_sort(values: &[String]) -> Vec<String> {
    let mut deduped = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    deduped.sort_unstable();
    deduped.dedup();
    deduped
}

/// 获取当前时间的 RFC3339 格式字符串
///
/// 生成当前 UTC 时间的 RFC3339 格式字符串，用于记录状态更新时间戳。
///
/// # 返回值
///
/// 返回 RFC3339 格式的时间字符串，例如："2024-01-15T10:30:45.123456789+00:00"
///
/// # 错误处理
///
/// 如果系统时间获取失败（极罕见），返回 Unix 纪元时间（1970-01-01T00:00:00+00:00）。
/// 如果时间戳转换失败，同样返回 Unix 纪元时间作为回退。
fn now_rfc3339() -> String {
    // 获取自 Unix 纪元以来的秒数
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    // 将秒数转换为 DateTime 并格式化为 RFC3339
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs as i64, 0)
        .unwrap_or(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH)
        .to_rfc3339()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

//! # 计划任务类型定义模块
//!
//! 本模块定义了 VibeWindow 计划任务系统的核心数据类型。
//!
//! ## 主要功能
//!
//! - **任务类型枚举**：定义 Shell 命令和 Agent 代理两种任务类型
//! - **会话目标枚举**：定义任务执行的会话隔离策略
//! - **调度配置**：支持 Cron 表达式、指定时间、固定间隔和双周日历调度
//! - **投递配置**：定义任务执行结果的通知/投递策略
//! - **任务实体**：完整描述一个计划任务的配置、状态与执行历史
//!
//! ## 设计原则
//!
//! - 所有类型均实现 `Serialize`/`Deserialize`，支持持久化存储
//! - 提供合理的默认值，减少配置复杂度
//! - 类型安全，避免字符串硬编码

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// 计划任务类型枚举
///
/// 定义任务执行时采用的执行器类型。
///
/// # 变体
///
/// - `Shell`：执行系统 Shell 命令
/// - `Agent`：调用 VibeWindow Agent 进行智能任务处理
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::cron::types::JobType;
///
/// let job_type = JobType::Shell;
/// let type_str: &str = job_type.into();
/// assert_eq!(type_str, "shell");
///
/// let parsed: JobType = "agent".try_into().unwrap();
/// assert_eq!(parsed, JobType::Agent);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum JobType {
    /// Shell 命令类型，执行系统 Shell 命令
    #[default]
    Shell,
    /// Agent 类型，调用 VibeWindow Agent 执行智能任务
    Agent,
}

impl From<JobType> for &'static str {
    /// 将 `JobType` 转换为静态字符串
    ///
    /// # 参数
    ///
    /// - `value`: 要转换的任务类型
    ///
    /// # 返回值
    ///
    /// 返回对应的小写字符串标识：
    /// - `Shell` -> `"shell"`
    /// - `Agent` -> `"agent"`
    fn from(value: JobType) -> Self {
        match value {
            JobType::Shell => "shell",
            JobType::Agent => "agent",
        }
    }
}

impl TryFrom<&str> for JobType {
    type Error = String;

    /// 从字符串解析 `JobType`
    ///
    /// # 参数
    ///
    /// - `value`: 字符串表示的任务类型（不区分大小写）
    ///
    /// # 返回值
    ///
    /// - `Ok(JobType)`: 解析成功时返回对应的任务类型
    /// - `Err(String)`: 解析失败时返回错误信息
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use crate::app::agent::cron::types::JobType;
    ///
    /// assert_eq!(JobType::try_from("shell").unwrap(), JobType::Shell);
    /// assert_eq!(JobType::try_from("AGENT").unwrap(), JobType::Agent);
    /// assert!(JobType::try_from("invalid").is_err());
    /// ```
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "shell" => Ok(JobType::Shell),
            "agent" => Ok(JobType::Agent),
            _ => Err(format!("Invalid job type '{}'. Expected one of: 'shell', 'agent'", value)),
        }
    }
}

/// 会话目标枚举
///
/// 定义任务执行时使用的会话隔离策略，影响上下文共享范围。
///
/// # 变体
///
/// - `Isolated`：隔离会话，任务在独立的上下文中执行，不共享状态（默认）
/// - `Main`：主会话，任务在主会话中执行，可共享上下文和状态
///
/// # 使用场景
///
/// - `Isolated`：适合独立的一次性任务，避免状态污染
/// - `Main`：适合需要访问主会话上下文的连续任务
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SessionTarget {
    /// 隔离会话，任务在独立上下文中执行
    #[default]
    Isolated,
    /// 主会话，任务在主会话中执行，可共享上下文
    Main,
}

impl SessionTarget {
    /// 获取会话目标的字符串表示
    ///
    /// # 返回值
    ///
    /// 返回小写的静态字符串：
    /// - `Isolated` -> `"isolated"`
    /// - `Main` -> `"main"`
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Isolated => "isolated",
            Self::Main => "main",
        }
    }

    /// 从字符串解析会话目标
    ///
    /// # 参数
    ///
    /// - `raw`: 字符串表示的会话目标（不区分大小写）
    ///
    /// # 返回值
    ///
    /// 返回对应的 `SessionTarget`，无法识别时默认返回 `Isolated`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use crate::app::agent::cron::types::SessionTarget;
    ///
    /// assert_eq!(SessionTarget::parse("main"), SessionTarget::Main);
    /// assert_eq!(SessionTarget::parse("ISOLATED"), SessionTarget::Isolated);
    /// assert_eq!(SessionTarget::parse("unknown"), SessionTarget::Isolated);
    /// ```
    pub(crate) fn parse(raw: &str) -> Self {
        // 忽略大小写匹配 "main"，其他情况均返回 Isolated
        if raw.eq_ignore_ascii_case("main") { Self::Main } else { Self::Isolated }
    }
}

/// 调度配置枚举
///
/// 定义任务的调度方式，支持四种不同的时间触发机制。
///
/// # 变体
///
/// - `Cron`：基于 Cron 表达式的周期性调度
/// - `At`：在指定的绝对时间点执行一次
/// - `Every`：按固定时间间隔重复执行
/// - `Biweekly`：锚定当地日历周的隔周调度
///
/// # 序列化格式
///
/// 使用 `#[serde(tag = "kind")]` 实现内部标签，JSON 格式示例：
///
/// ```json
/// // Cron 调度
/// {"kind": "cron", "expr": "0 0 * * * *", "tz": "Asia/Shanghai"}
///
/// // 定时执行
/// {"kind": "at", "at": "2024-01-01T00:00:00Z"}
///
/// // 固定间隔
/// {"kind": "every", "every_ms": 60000}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Schedule {
    /// Cron 表达式调度
    ///
    /// # 字段
    ///
    /// - `expr`: Cron 表达式字符串
    /// - `tz`: 可选时区标识（如 "Asia/Shanghai"），默认使用 UTC
    Cron {
        /// Cron 表达式，格式为 `秒 分 时 日 月 周`
        expr: String,
        /// 可选时区，未指定时使用 UTC
        #[serde(default)]
        tz: Option<String>,
    },
    /// 从锚点所在当地周一开始，每隔一周在选定星期的当地时间执行。
    Biweekly {
        /// 当地锚点日期，其所在周为第零活动周。
        anchor_date: NaiveDate,
        /// 当地星期，周一为 1，周日为 7；至少选择一个。
        weekdays: Vec<u32>,
        /// 当地小时（0–23）。
        hour: u32,
        /// 当地分钟（0–59）。
        minute: u32,
        /// IANA 时区标识。
        timezone: String,
    },
    /// 指定时间点执行
    ///
    /// # 字段
    ///
    /// - `at`: UTC 时间的执行时刻
    At {
        /// 执行的绝对时间点（UTC）
        at: DateTime<Utc>,
    },
    /// 固定间隔执行
    ///
    /// # 字段
    ///
    /// - `every_ms`: 间隔毫秒数
    Every {
        /// 执行间隔，单位为毫秒
        every_ms: u64,
    },
}

/// 任务投递配置
///
/// 定义任务执行结果的通知/投递策略。
///
/// # 字段
///
/// - `mode`: 投递模式标识，默认为 "none"（不投递）
/// - `channel`: 可选的投递通道标识（如 "telegram"、"slack"）
/// - `to`: 可选的目标接收者标识
/// - `best_effort`: 是否启用尽力投递模式，默认为 true
///
/// # 尽力投递模式
///
/// 当 `best_effort` 为 `true` 时，投递失败不会导致任务失败标记；
/// 为 `false` 时，投递失败将记录为任务执行错误。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliveryConfig {
    /// 投递模式标识，如 "none"、"webhook"、"channel" 等
    #[serde(default)]
    pub mode: String,
    /// 投递通道标识，指定通过哪个通道发送结果
    #[serde(default)]
    pub channel: Option<String>,
    /// 目标接收者标识，如用户 ID 或频道名称
    #[serde(default)]
    pub to: Option<String>,
    /// 尽力投递标志，为 true 时投递失败不影响任务状态
    #[serde(default = "default_true")]
    pub best_effort: bool,
}

impl Default for DeliveryConfig {
    /// 生成默认的投递配置
    ///
    /// # 默认值
    ///
    /// - `mode`: `"none"`（不进行投递）
    /// - `channel`: `None`
    /// - `to`: `None`
    /// - `best_effort`: `true`
    fn default() -> Self {
        Self { mode: "none".to_string(), channel: None, to: None, best_effort: true }
    }
}

/// 返回 `true` 的辅助函数
///
/// 用于为 `DeliveryConfig::best_effort` 字段提供序列化默认值。
/// 该函数作为 `#[serde(default = "...")]` 的参数使用。
fn default_true() -> bool {
    true
}

/// 解析并清理模型回退列表。
pub(crate) fn normalize_fallbacks(fallbacks: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for fallback in fallbacks {
        let value = fallback.trim();
        if !value.is_empty() && !out.iter().any(|existing: &String| existing == value) {
            out.push(value.to_string());
        }
    }
    out
}

/// 计划任务完整定义
///
/// 描述一个计划任务的所有配置信息、调度规则、执行状态和历史记录。
///
/// # 字段分组
///
/// ## 标识与基础配置
/// - `id`: 任务唯一标识符
/// - `name`: 可选的任务名称
/// - `expression`: 原始 Cron 表达式（用于显示）
///
/// ## 执行配置
/// - `schedule`: 调度配置
/// - `command`: Shell 命令或 Agent 指令
/// - `prompt`: Agent 类型任务的可选提示词
/// - `job_type`: 任务类型
/// - `session_target`: 会话目标
/// - `model`: Agent 类型任务的可选模型标识
///
/// ## 状态与控制
/// - `enabled`: 是否启用
/// - `delete_after_run`: 单次执行后是否删除
/// - `delivery`: 投递配置
///
/// ## 时间与历史
/// - `created_at`: 创建时间
/// - `next_run`: 下次执行时间
/// - `last_run`: 上次执行时间
/// - `last_status`: 上次执行状态
/// - `last_output`: 上次执行输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    /// 任务唯一标识符
    pub id: String,
    /// 原始 Cron 表达式（用于显示和调试）
    pub expression: String,
    /// 调度配置，定义任务触发规则
    pub schedule: Schedule,
    /// Shell 命令或 Agent 指令内容
    pub command: String,
    /// Agent 类型任务的可选提示词
    pub prompt: Option<String>,
    /// 可选的任务名称
    pub name: Option<String>,
    /// 任务类型（Shell 或 Agent）
    pub job_type: JobType,
    /// 会话目标（隔离或主会话）
    pub session_target: SessionTarget,
    /// Agent 类型任务的可选模型标识
    pub model: Option<String>,
    /// 可选的委托代理标识，用于套用代理的 provider/model/温度配置。
    pub agent: Option<String>,
    /// 可选的 ACP 智能体标识；为空时使用普通 Agent 执行链路。
    pub acp_agent: Option<String>,
    /// 可选的项目工作目录；设置后任务在该项目下执行。
    pub project_path: Option<String>,
    /// 是否在桌面端唤醒/提示用户关注该任务。
    pub wake: bool,
    /// Agent 模型失败时依次尝试的回退模型列表。
    pub fallbacks: Vec<String>,
    /// Agent 任务是否以完全访问权限执行。
    pub full_access: bool,
    /// Agent 任务是否投递到项目任务池，而不是立即执行。
    pub task_pool: bool,
    /// 是否启用该任务
    pub enabled: bool,
    /// 投递配置
    pub delivery: DeliveryConfig,
    /// 单次执行后是否删除该任务
    pub delete_after_run: bool,
    /// 任务创建时间（UTC）
    pub created_at: DateTime<Utc>,
    /// 预计下次执行时间（UTC）
    pub next_run: DateTime<Utc>,
    /// 实际上次执行时间（UTC）
    pub last_run: Option<DateTime<Utc>>,
    /// 上次执行状态（如 "success"、"failed"）
    pub last_status: Option<String>,
    /// 上次执行的输出内容
    pub last_output: Option<String>,
}

/// 计划任务执行记录
///
/// 记录一次计划任务的完整执行信息，包括时间、状态和输出。
///
/// # 字段
///
/// - `id`: 执行记录唯一标识符
/// - `job_id`: 关联的任务标识符
/// - `started_at`: 执行开始时间
/// - `finished_at`: 执行结束时间
/// - `status`: 执行状态
/// - `output`: 可选的执行输出
/// - `duration_ms`: 可选的执行耗时（毫秒）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronRun {
    /// 执行记录唯一标识符
    pub id: i64,
    /// 关联的计划任务标识符
    pub job_id: String,
    /// 执行开始时间（UTC）
    pub started_at: DateTime<Utc>,
    /// 执行结束时间（UTC）
    pub finished_at: DateTime<Utc>,
    /// 执行状态（如 "success"、"failed"、"timeout"）
    pub status: String,
    /// 可选的执行输出内容
    pub output: Option<String>,
    /// 可选的执行耗时，单位为毫秒
    pub duration_ms: Option<i64>,
}

/// 计划任务部分更新配置
///
/// 用于对现有任务进行部分字段的更新，所有字段均为可选。
/// 未指定的字段在更新时保持原值不变。
///
/// # 字段
///
/// 所有字段与 `CronJob` 对应，但均为 `Option` 类型，
/// 仅更新明确指定的字段。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CronJobPatch {
    /// 新的任务类型
    pub job_type: Option<JobType>,
    /// 新的调度配置
    pub schedule: Option<Schedule>,
    /// 新的命令或指令
    pub command: Option<String>,
    /// 新的 Agent 提示词
    pub prompt: Option<String>,
    /// 新的任务名称
    pub name: Option<String>,
    /// 新的启用状态
    pub enabled: Option<bool>,
    /// 新的投递配置
    pub delivery: Option<DeliveryConfig>,
    /// 新的模型标识
    pub model: Option<String>,
    /// 新的会话目标
    pub session_target: Option<SessionTarget>,
    /// 新的执行后删除标志
    pub delete_after_run: Option<bool>,
    /// 新的委托代理标识
    pub agent: Option<String>,
    /// 新的 ACP 智能体标识
    pub acp_agent: Option<String>,
    /// 新的项目工作目录。空字符串清除项目，None 保留原值。
    pub project_path: Option<String>,
    /// 新的唤醒标志
    pub wake: Option<bool>,
    /// 新的模型回退列表
    pub fallbacks: Option<Vec<String>>,
    /// 新的完全访问权限标志
    pub full_access: Option<bool>,
    /// 新的任务池投递标志
    pub task_pool: Option<bool>,
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

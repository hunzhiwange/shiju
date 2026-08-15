use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
/// 定时任务调度器配置（`[scheduler]` 配置段）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SchedulerConfig {
    /// 是否启用内置调度循环。
    #[serde(default = "default_scheduler_enabled")]
    pub enabled: bool,
    /// 可持久化保存的定时任务数量上限。
    #[serde(default = "default_scheduler_max_tasks")]
    pub max_tasks: usize,
    /// 每次调度轮询周期内最多执行的任务数。
    #[serde(default = "default_scheduler_max_concurrent")]
    pub max_concurrent: usize,
}

fn default_scheduler_enabled() -> bool {
    true
}

fn default_scheduler_max_tasks() -> usize {
    64
}

fn default_scheduler_max_concurrent() -> usize {
    4
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: default_scheduler_enabled(),
            max_tasks: default_scheduler_max_tasks(),
            max_concurrent: default_scheduler_max_concurrent(),
        }
    }
}

/// Cron 作业配置（`[cron]` 配置段）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CronConfig {
    /// 是否启用 cron 子系统。默认值为 `true`。
    ///
    /// - `true`：允许启动调度循环并分发到期作业。
    /// - `false`：保持调度循环关闭，但现有作业定义仍保留在磁盘中。
    ///
    /// 这是整个 cron 运行时的全局安全开关。
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 每个作业保留的历史运行记录上限。默认值为 `50`。
    ///
    /// 超过该限制时会优先裁剪最旧记录。
    /// 增大该值可保留更长审计历史，减小该值可降低存储开销。
    #[serde(default = "default_max_run_history")]
    pub max_run_history: u32,
}

fn default_max_run_history() -> u32 {
    50
}

impl Default for CronConfig {
    fn default() -> Self {
        Self { enabled: true, max_run_history: default_max_run_history() }
    }
}

/// 心跳配置（`[heartbeat]` 配置段），用于周期性健康检查。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HeartbeatConfig {
    /// 是否启用周期性心跳。默认值为 `false`。
    pub enabled: bool,
    /// 心跳发送间隔，单位为分钟。默认值为 `30`。
    pub interval_minutes: u32,
    /// 当 `HEARTBEAT.md` 中没有任务条目时使用的可选兜底消息。
    #[serde(default)]
    pub message: Option<String>,
    /// 心跳输出使用的可选投递通道，例如 `telegram`。
    #[serde(default, alias = "channel")]
    pub target: Option<String>,
    /// 可选的投递接收方或聊天标识；设置 `target` 时通常需要一并配置。
    #[serde(default, alias = "recipient")]
    pub to: Option<String>,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self { enabled: false, interval_minutes: 30, message: None, target: None, to: None }
    }
}

fn default_true() -> bool {
    true
}
#[cfg(test)]
#[path = "automation_tests.rs"]
mod automation_tests;

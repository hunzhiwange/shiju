use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
/// 研究阶段触发模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum ResearchTrigger {
    /// 从不触发研究阶段。
    #[default]
    Never,
    /// 每次响应前都触发研究阶段。
    Always,
    /// 当消息包含配置的关键词时触发。
    Keywords,
    /// 当消息长度超过阈值时触发。
    Length,
    /// 当消息中包含问号时触发。
    Question,
}

/// 研究阶段配置（`[research]` 配置段）。
///
/// 启用后，代理会在生成主响应前主动使用工具收集信息，形成一个“研究”阶段，
/// 用于探索代码库、检索记忆或抓取外部数据，从而提升回答质量。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResearchPhaseConfig {
    /// 是否启用研究阶段。
    #[serde(default)]
    pub enabled: bool,

    /// 何时触发研究阶段。
    #[serde(default)]
    pub trigger: ResearchTrigger,

    /// 触发研究阶段的关键词列表，在 `trigger = "keywords"` 时生效。
    #[serde(default = "default_research_keywords")]
    pub keywords: Vec<String>,

    /// 触发研究阶段的最小消息长度，在 `trigger = "length"` 时生效。
    #[serde(default = "default_research_min_length")]
    pub min_message_length: usize,

    /// 研究阶段允许的最大工具调用迭代次数。
    #[serde(default = "default_research_max_iterations")]
    pub max_iterations: usize,

    /// 是否在研究阶段显示详细进度，例如工具调用和结果。
    #[serde(default = "default_true")]
    pub show_progress: bool,

    /// 研究阶段使用的自定义 system prompt 前缀。
    /// 为空时使用默认研究指令。
    #[serde(default)]
    pub system_prompt_prefix: String,
}

fn default_research_keywords() -> Vec<String> {
    vec![
        "find".into(),
        "search".into(),
        "check".into(),
        "investigate".into(),
        "look".into(),
        "research".into(),
        "найди".into(),
        "проверь".into(),
        "исследуй".into(),
        "поищи".into(),
    ]
}

fn default_research_min_length() -> usize {
    50
}

fn default_research_max_iterations() -> usize {
    5
}

impl Default for ResearchPhaseConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            trigger: ResearchTrigger::default(),
            keywords: default_research_keywords(),
            min_message_length: default_research_min_length(),
            max_iterations: default_research_max_iterations(),
            show_progress: true,
            system_prompt_prefix: String::new(),
        }
    }
}

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

/// 自主目标循环引擎配置（`[goal_loop]`）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GoalLoopConfig {
    /// 是否启用自主目标执行。默认值为 `false`。
    pub enabled: bool,
    /// 目标循环之间的间隔，单位为分钟。默认值为 `10`。
    pub interval_minutes: u32,
    /// 单步执行超时时间，单位为秒。默认值为 `120`。
    pub step_timeout_secs: u64,
    /// 每个循环最多执行的步骤数。默认值为 `3`。
    pub max_steps_per_cycle: u32,
    /// 发送目标事件的可选通道，例如 `lark`、`telegram`。
    #[serde(default)]
    pub channel: Option<String>,
    /// 目标事件投递使用的可选接收方或 chat_id。
    #[serde(default)]
    pub target: Option<String>,
}

impl Default for GoalLoopConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_minutes: 10,
            step_timeout_secs: 120,
            max_steps_per_cycle: 3,
            channel: None,
            target: None,
        }
    }
}

fn default_true() -> bool {
    true
}
#[cfg(test)]
#[path = "automation_tests.rs"]
mod automation_tests;

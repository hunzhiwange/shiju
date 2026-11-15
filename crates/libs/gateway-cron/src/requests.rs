//! Cron HTTP 请求及窗口缺失/null 区分。
use serde::Deserialize;

/// 添加定时任务的请求体
///
/// 用于创建新的定时任务（Cron Job），指定执行计划、命令和可选的任务名称。
///
/// # 示例
///
/// ```json
/// {
///     "name": "每日数据备份",
///     "schedule": "0 2 * * *",
///     "command": "backup --target=/data"
/// }
/// ```
#[derive(Default, Deserialize)]
pub struct CronAddBody {
    pub timezone: Option<String>,
    pub local_at: Option<String>,
    pub anchor_date: Option<String>,
    pub weekdays: Option<Vec<u32>>,
    pub hour: Option<u32>,
    pub minute: Option<u32>,
    #[serde(default, deserialize_with = "cron_window_field")]
    pub starts_at: Option<Option<String>>,
    #[serde(default, deserialize_with = "cron_window_field")]
    pub ends_at: Option<Option<String>>,
    #[serde(default, deserialize_with = "cron_window_field")]
    pub start_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "cron_window_field")]
    pub end_date: Option<Option<String>>,

    /// 任务的可读名称
    ///
    /// 可选参数，用于在日志和管理界面中标识此任务。
    /// 如果不提供，系统可能会生成默认名称或使用任务 ID。
    #[serde(default)]
    pub name: Option<String>,

    /// 任务类型：`shell` 或 `agent`。默认 `shell`。
    #[serde(default)]
    pub job_type: Option<String>,

    /// 调度类型：`cron`、`at`、`every` 或 `biweekly`。默认 `cron`。
    #[serde(default)]
    pub schedule_kind: Option<String>,

    /// Cron 表达式，定义任务的执行计划
    ///
    /// 必填参数，使用标准的五字段或六字段 Cron 表达式格式。
    /// 例如：`0 * * * *` 表示每小时执行一次。
    #[serde(default)]
    pub schedule: Option<String>,

    /// `at` 调度的 RFC3339 时间。
    #[serde(default)]
    pub at: Option<String>,

    /// `every` 调度的间隔毫秒数。
    #[serde(default)]
    pub every_ms: Option<u64>,

    /// 任务执行时要运行的命令
    ///
    /// 必填参数，支持 shell 命令或系统支持的命令语法。
    /// 注意：命令会在受限的执行环境中运行，需遵守安全策略。
    #[serde(default)]
    pub command: Option<String>,

    /// Agent 任务的提示词。
    #[serde(default)]
    pub prompt: Option<String>,

    /// Agent 任务的会话目标：`isolated` 或 `main`。
    #[serde(default)]
    pub session_target: Option<String>,

    /// Agent 任务的模型覆盖。
    #[serde(default)]
    pub model: Option<String>,

    /// Agent 任务使用的委托代理配置 key。
    #[serde(default)]
    pub agent: Option<String>,

    /// Agent 任务使用的 ACP 智能体；空值表示不使用 ACP。
    #[serde(default)]
    pub acp_agent: Option<String>,

    /// 任务执行项目目录。
    #[serde(default)]
    pub project_path: Option<String>,

    /// 是否唤醒/提示用户关注该任务。
    #[serde(default)]
    pub wake: Option<bool>,

    /// Agent 模型失败时依次尝试的回退模型列表。
    #[serde(default)]
    pub fallbacks: Option<Vec<String>>,

    /// Agent 任务是否以完全访问权限执行。
    #[serde(default)]
    pub full_access: Option<bool>,

    /// Agent 任务是否投递到项目任务池。
    #[serde(default)]
    pub task_pool: Option<bool>,

    /// 任务结果投递模式，例如 `none` 或 `announce`。
    #[serde(default)]
    pub delivery_mode: Option<String>,

    /// 任务结果投递通道。
    #[serde(default)]
    pub delivery_channel: Option<String>,

    /// 任务结果投递目标。
    #[serde(default)]
    pub delivery_to: Option<String>,

    /// 投递失败是否不影响任务状态。
    #[serde(default)]
    pub delivery_best_effort: Option<bool>,

    /// 单次任务运行成功后是否删除。
    #[serde(default)]
    pub delete_after_run: Option<bool>,
}

// Missing uses Default (None); explicit null must remain Some(None).
fn cron_window_field<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(Some)
}

/// 更新定时任务的请求体。
///
/// 所有字段均为可选，只修改请求中明确提供的字段。
#[derive(Default, Deserialize)]
pub struct CronUpdateBody {
    pub timezone: Option<String>,
    pub local_at: Option<String>,
    pub anchor_date: Option<String>,
    pub weekdays: Option<Vec<u32>>,
    pub hour: Option<u32>,
    pub minute: Option<u32>,
    #[serde(default, deserialize_with = "cron_window_field")]
    pub starts_at: Option<Option<String>>,
    #[serde(default, deserialize_with = "cron_window_field")]
    pub ends_at: Option<Option<String>>,
    #[serde(default, deserialize_with = "cron_window_field")]
    pub start_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "cron_window_field")]
    pub end_date: Option<Option<String>>,

    /// 新任务名称。
    pub name: Option<String>,

    /// 新任务类型：`shell` 或 `agent`。
    pub job_type: Option<String>,

    /// 新调度类型：`cron`、`at`、`every` 或 `biweekly`。
    pub schedule_kind: Option<String>,

    /// 新 Cron 表达式。
    pub schedule: Option<String>,

    /// `at` 调度的 RFC3339 时间。
    pub at: Option<String>,

    /// `every` 调度的间隔毫秒数。
    pub every_ms: Option<u64>,

    /// 新 Shell 命令。
    pub command: Option<String>,

    /// 新 Agent 提示词。
    pub prompt: Option<String>,

    /// 新会话目标。
    pub session_target: Option<String>,

    /// 新模型覆盖。
    pub model: Option<String>,

    /// 新委托代理配置 key。
    pub agent: Option<String>,

    /// 新 ACP 智能体；空值表示不使用 ACP。
    pub acp_agent: Option<String>,

    /// 新项目工作目录。
    pub project_path: Option<String>,

    /// 新唤醒标志。
    pub wake: Option<bool>,

    /// 新回退模型列表。
    pub fallbacks: Option<Vec<String>>,

    /// 新完全访问权限标志。
    pub full_access: Option<bool>,

    /// 新任务池投递标志。
    pub task_pool: Option<bool>,

    /// 新投递模式。
    pub delivery_mode: Option<String>,

    /// 新投递通道。
    pub delivery_channel: Option<String>,

    /// 新投递目标。
    pub delivery_to: Option<String>,

    /// 新投递失败策略。
    pub delivery_best_effort: Option<bool>,

    /// 新执行后删除标志。
    pub delete_after_run: Option<bool>,

    /// 是否启用任务。
    pub enabled: Option<bool>,
}

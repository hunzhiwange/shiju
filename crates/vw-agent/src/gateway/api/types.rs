//! # REST API 请求与响应类型
//!
//! 本模块定义了 REST API 处理器使用的所有请求和响应数据结构。
//!
//! ## 主要功能
//!
//! - 定义内存查询与存储的请求结构
//! - 定义定时任务（Cron）的添加请求结构
//! - 定义集成凭据更新的请求与响应结构
//! - 提供集成设置相关的完整数据模型
//!
//! ## 使用场景
//!
//! 这些类型主要用于：
//! 1. 反序列化 HTTP 请求体（通过 `Deserialize` trait）
//! 2. 序列化 HTTP 响应体（通过 `Serialize` trait）
//! 3. 在 API 路由处理器与业务逻辑层之间传递数据

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 内存查询请求参数
///
/// 用于查询存储在内存系统中的数据，支持按关键词和类别进行过滤。
///
/// # 示例
///
/// ```json
/// {
///     "query": "用户偏好设置",
///     "category": "preferences"
/// }
/// ```
#[derive(Deserialize)]
pub struct MemoryQuery {
    /// 查询关键词或短语
    ///
    /// 可选参数，用于模糊匹配内存中的内容。
    /// 如果不提供，则可能返回所有或默认数量的记录。
    pub query: Option<String>,

    /// 数据类别过滤器
    ///
    /// 可选参数，用于限定查询的数据类别。
    /// 例如："preferences"、"history"、"cache" 等。
    pub category: Option<String>,
}

/// 内存存储请求体
///
/// 用于向内存系统存储新的数据条目，包含键、内容和可选的类别标签。
///
/// # 示例
///
/// ```json
/// {
///     "key": "user_theme",
///     "content": "dark",
///     "category": "preferences"
/// }
/// ```
#[derive(Deserialize)]
pub struct MemoryStoreBody {
    /// 数据的唯一标识键
    ///
    /// 必填参数，用于后续查询或更新此数据条目。
    /// 建议使用有意义且唯一的命名，如 "user_settings.theme"。
    pub key: String,

    /// 要存储的实际内容
    ///
    /// 必填参数，支持任意文本格式的内容。
    /// 对于结构化数据，建议使用 JSON 字符串格式。
    pub content: String,

    /// 数据类别标签
    ///
    /// 可选参数，用于对存储的数据进行分类管理。
    /// 便于后续按类别批量查询或清理数据。
    pub category: Option<String>,
}

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

/// 集成凭据更新请求体
///
/// 用于更新特定集成（如第三方服务）的凭据字段，支持乐观并发控制。
///
/// # 并发控制
///
/// 使用 `revision` 字段实现乐观并发控制，确保在多个并发更新时
/// 不会意外覆盖其他客户端的修改。
///
/// # 示例
///
/// ```json
/// {
///     "revision": "v3",
///     "fields": {
///         "api_key": "sk-xxxx",
///         "endpoint": "https://api.example.com"
///     }
/// }
/// ```
#[derive(Deserialize)]
pub struct IntegrationCredentialsUpdateBody {
    /// 当前配置的修订版本标识
    ///
    /// 可选参数，用于乐观并发控制。
    /// 如果服务器上的当前版本与此不匹配，更新可能会被拒绝。
    pub revision: Option<String>,

    /// 要更新的凭据字段键值对
    ///
    /// 键为字段名称，值为新的凭据值。
    /// 使用 `BTreeMap` 确保字段的序列化顺序稳定。
    /// 默认为空 map，可通过 `#[serde(default)]` 处理缺失字段。
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
}

/// 集成凭据字段的元数据
///
/// 描述单个凭据字段的完整信息，包括如何显示、验证和编辑该字段。
/// 主要用于前端表单渲染和用户交互。
///
/// # 安全考虑
///
/// - `current_value` 和 `masked_value` 字段可能包含敏感信息
/// - 在日志和响应中应谨慎处理这些字段的显示
#[derive(Debug, Clone, Serialize)]
pub struct IntegrationCredentialsField {
    /// 字段的唯一标识符
    ///
    /// 用于在 API 请求中引用此字段，通常是技术性的键名。
    /// 例如："api_key"、"access_token"、"endpoint_url"。
    pub key: String,

    /// 字段的人类可读标签
    ///
    /// 用于在用户界面中显示的字段名称。
    /// 例如："API 密钥"、"访问令牌"、"端点 URL"。
    pub label: String,

    /// 字段是否为必填项
    ///
    /// 如果为 `true`，用户必须在提交表单前填写此字段。
    /// 必填字段通常在 UI 中会有视觉标记（如星号）。
    pub required: bool,

    /// 该字段当前是否已配置值
    ///
    /// 如果为 `true`，表示系统中已存储了该字段的值。
    /// 这有助于前端区分"从未配置"和"需要更新"的状态。
    pub has_value: bool,

    /// 字段的输入类型
    ///
    /// 指定前端应使用的输入控件类型，使用静态字符串引用以优化性能。
    /// 常见值包括：
    /// - `"text"`：普通文本输入
    /// - `"password"`：密码输入（字符掩码）
    /// - `"select"`：下拉选择框（配合 `options` 使用）
    /// - `"textarea"`：多行文本输入
    pub input_type: &'static str,

    /// 下拉选择框的可选项列表
    ///
    /// 当 `input_type` 为 `"select"` 时，此字段提供可选值的列表。
    /// 对于其他输入类型，此列表通常为空。
    /// 默认为空 vector，通过 `#[serde(default)]` 处理。
    #[serde(default)]
    pub options: Vec<String>,

    /// 字段的当前实际值
    ///
    /// 可选字段，包含当前存储的凭据值。
    /// 注意：仅在安全上下文中返回，通常不建议在 API 响应中包含原始值。
    /// 如果值为 `None`，则该字段尚未配置。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_value: Option<String>,

    /// 字段的掩码显示值
    ///
    /// 可选字段，提供经过掩码处理的值用于安全显示。
    /// 例如，API 密钥可能显示为 "sk-****xxxx"。
    /// 如果为 `None`，则该字段不适合显示掩码值或未配置。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub masked_value: Option<String>,
}

/// 单个集成设置的完整条目
///
/// 包含特定集成（如 Slack、Discord、Telegram 等）的所有元数据、状态和配置信息。
/// 用于在集成设置列表中展示每个集成的详细情况。
///
/// # 数据来源
///
/// 此结构通常由 `integrations` 模块的数据转换而来，
/// 聚合了集成的定义、运行时状态和配置信息。
#[derive(Debug, Clone, Serialize)]
pub struct IntegrationSettingsEntry {
    /// 集成的唯一标识符
    ///
    /// 用于在 API 请求中引用此集成。
    /// 通常是集成提供者的名称，如 "slack"、"discord"、"telegram"。
    pub id: String,

    /// 集成的显示名称
    ///
    /// 用于在用户界面中显示的友好名称。
    /// 例如："Slack 通知"、"Discord 机器人"、"Telegram 消息"。
    pub name: String,

    /// 集成的详细描述
    ///
    /// 提供集成的功能说明、使用场景或注意事项。
    /// 帮助用户理解该集成的作用和配置要求。
    pub description: String,

    /// 集成的类别分类
    ///
    /// 从 `integrations` 模块导入的枚举类型，用于将集成分组显示。
    /// 常见类别包括：消息通道、AI 提供者、存储后端等。
    pub category: crate::app::agent::integrations::IntegrationCategory,

    /// 集成的当前运行状态
    ///
    /// 从 `integrations` 模块导入的枚举类型，表示集成的运行状况。
    /// 可能的状态包括：活跃、停用、错误、初始化中等。
    pub status: crate::app::agent::integrations::IntegrationStatus,

    /// 集成是否已完成基本配置
    ///
    /// 如果为 `true`，表示该集成的必填字段都已配置。
    /// 注意：已配置并不意味着集成可用，还需检查 `status` 字段。
    pub configured: bool,

    /// 该集成是否激活了默认提供者
    ///
    /// 如果为 `true`，表示此集成是某个类别的默认提供者。
    /// 例如，如果该集成是 AI 提供者类别中的默认选择。
    /// 对于非提供者类型的集成，此字段通常为 `false`。
    pub activates_default_provider: bool,

    /// 该集成所需的凭据字段列表
    ///
    /// 包含所有需要配置的字段的元数据和当前状态。
    /// 前端通过此列表动态生成配置表单。
    pub fields: Vec<IntegrationCredentialsField>,
}

/// 集成设置的完整载荷
///
/// 包含所有集成设置的完整信息，包括配置版本和当前激活的默认提供者。
/// 作为集成设置 API 端点的主要响应结构。
///
/// # 版本控制
///
/// `revision` 字段用于跟踪配置的变更历史，支持乐观并发控制。
/// 客户端在更新配置时应提供最新的 revision 值。
///
/// # 使用场景
///
/// 此结构通常在以下情况下返回：
/// 1. 用户打开集成设置页面时
/// 2. 客户端轮询配置变更时
/// 3. 配置更新后的确认响应
#[derive(Debug, Clone, Serialize)]
pub struct IntegrationSettingsPayload {
    /// 当前配置的修订版本标识
    ///
    /// 用于乐观并发控制和缓存失效。
    /// 每次配置变更时，此值都会更新。
    pub revision: String,

    /// 当前激活的默认提供者集成 ID
    ///
    /// 可选字段，标识哪个集成是当前激活的默认提供者。
    /// 如果为 `None`，则表示没有设置默认提供者。
    /// 序列化时跳过 `None` 值以保持响应简洁。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_default_provider_integration_id: Option<String>,

    /// 所有可用集成的设置条目列表
    ///
    /// 包含系统中定义的所有集成的完整信息。
    /// 列表顺序可能按类别和名称排序，便于前端展示。
    pub integrations: Vec<IntegrationSettingsEntry>,
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;

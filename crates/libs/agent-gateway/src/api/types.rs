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

use serde::Serialize;
// Integration credential request types are owned by gateway-integrations.

pub use gateway_memory::{MemoryQuery, MemoryStoreBody};

pub use gateway_cron::requests::{CronAddBody, CronUpdateBody};

pub use gateway_integrations::IntegrationCredentialsUpdateBody;
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
    pub category: agent_integrations::IntegrationCategory,

    /// 集成的当前运行状态
    ///
    /// 从 `integrations` 模块导入的枚举类型，表示集成的运行状况。
    /// 可能的状态包括：活跃、停用、错误、初始化中等。
    pub status: agent_integrations::IntegrationStatus,

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

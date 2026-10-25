//! # 网关共享状态模块
//!
//! 本模块定义了所有 Axum HTTP 处理器共享的应用状态。
//!
//! ## 主要功能
//!
//! - 提供线程安全的共享状态容器，包含配置、提供商、内存、工具等核心组件
//! - 管理多通道集成（WhatsApp、Linq、Nextcloud Talk、Wati、QQ）
//! - 提供安全相关的状态（配对守卫、Webhook 密钥哈希）
//! - 支持可观测性、实时事件广播等扩展功能
//!
//! ## 设计原则
//!
//! - 所有敏感信息（如 Webhook 密钥）仅存储哈希值，绝不存储明文
//! - 使用 `Arc` 实现线程安全的共享访问
//! - 通过 `Clone` trait 支持在多个处理器间传递状态

use crate::app::agent::agent::loop_::query_engine::QueryEngine;
use crate::app::agent::channels::{
    LinqChannel, NextcloudTalkChannel, QQChannel, WatiChannel, WhatsAppChannel,
};
use crate::app::agent::config::Config;
use crate::app::agent::memory::Memory;
use crate::app::agent::observability::Observer;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use crate::app::agent::tools::Tool;
use crate::app::agent::tools::traits::ToolSpec;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) type SharedQueryEngine = Arc<tokio::sync::Mutex<QueryEngine>>;
pub(crate) type SessionQueryEngineStore =
    Arc<tokio::sync::Mutex<HashMap<String, SharedQueryEngine>>>;

/// 所有 Axum 处理器共享的应用状态
///
/// 该结构体封装了网关运行时所需的所有共享状态，包括配置、模型提供商、
/// 内存系统、安全机制、多通道集成等。通过 `Clone` trait 可以在多个
/// 异步任务和 HTTP 处理器间安全传递。
///
/// # 线程安全
///
/// - 使用 `Arc` 包装实现多所有者共享
/// - 可变状态（如 `config`）使用 `Mutex` 保护
/// - 内部可变性的组件自身实现了线程安全
///
/// # 示例
///
/// ```rust,ignore
/// let state = AppState {
///     config: Arc::new(Mutex::new(config)),
///     provider: Arc::new(my_provider),
///     model: "gpt-4".to_string(),
///     // ... 其他字段
/// };
///
/// // 在路由中使用
/// let app = Router::new()
///     .route("/api/chat", post(chat_handler))
///     .with_state(state);
/// ```
#[derive(Clone)]
pub struct AppState {
    /// 应用配置，使用互斥锁保护以支持运行时更新
    pub config: Arc<Mutex<Config>>,

    /// AI 模型提供商实例，用于调用 LLM API
    pub provider: Arc<dyn Provider>,

    /// 当前使用的模型名称（如 "gpt-4"、"claude-3-opus"）
    pub model: String,

    /// 模型生成的温度参数，控制输出随机性（0.0 - 2.0）
    pub temperature: f64,

    /// 记忆系统实例，用于对话历史和长期记忆存储
    pub mem: Arc<dyn Memory>,

    /// 是否启用自动保存功能
    pub auto_save: bool,

    /// 配对守卫，管理设备配对和授权状态
    pub pairing: Arc<PairingGuard>,

    /// 是否信任转发头部（如 X-Forwarded-For），用于反向代理场景
    pub trust_forwarded_headers: bool,

    /// 网关限流器，防止 API 滥用
    pub rate_limiter: Arc<super::GatewayRateLimiter>,

    /// 幂等性存储，用于请求去重和防止重复处理
    pub idempotency_store: Arc<super::IdempotencyStore>,

    /// 原子替换的通道连接与校验凭据；请求使用同一份快照。
    pub(crate) channels: Arc<Mutex<GatewayChannels>>,

    /// 可观测性后端，用于指标采集和监控
    pub observer: Arc<dyn Observer>,

    /// 已注册的工具规范列表（用于 Web 仪表板工具页面展示）
    pub tools_registry: Arc<Vec<ToolSpec>>,

    /// 可执行的工具实例列表（用于代理循环和 Web 聊天）
    pub tools_registry_exec: Arc<Vec<Box<dyn Tool>>>,

    /// 多模态配置，用于 Web 聊天中的图像处理
    pub multimodal: crate::app::agent::config::MultimodalConfig,

    /// 代理循环的最大工具迭代次数，防止无限循环
    pub max_iterations: usize,

    /// SSE（Server-Sent Events）广播通道发送端，用于实时事件推送
    pub event_tx: tokio::sync::broadcast::Sender<serde_json::Value>,

    /// 会话级查询引擎缓存，用于按 session_id 复用多轮历史与运行时装配
    pub(crate) session_query_engines: SessionQueryEngineStore,

    /// 可热替换的共享钩子运行时；测试 fixture 可用 `None` 保持原有语义。
    pub(crate) hook_runtime: Option<crate::app::agent::hooks::HookRuntime>,
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;

/// Webhook 请求的通道快照，连接与签名凭据始终来自同一版配置。
#[derive(Clone, Default)]
pub(crate) struct GatewayChannels {
    /// 在连接重载之间保留聊天执行状态；按租户隔离。
    pub(crate) runtimes: Arc<
        Mutex<HashMap<vw_api_types::id::TenantScope, Arc<crate::channels::ChannelRuntimeContext>>>,
    >,
    /// `X-Webhook-Secret` 的 SHA-256 哈希值（十六进制编码）
    ///
    /// **安全注意**: 绝不存储明文密钥，仅存储哈希值用于验证
    pub webhook_secret_hash: Option<Arc<str>>,

    /// WhatsApp 通道实例（可选）
    pub whatsapp: Option<Arc<WhatsAppChannel>>,

    /// WhatsApp 应用密钥，用于 Webhook 签名验证（`X-Hub-Signature-256` 头部）
    pub whatsapp_app_secret: Option<Arc<str>>,

    /// Linq 通道实例（可选）
    pub linq: Option<Arc<LinqChannel>>,

    /// Linq Webhook 签名密钥，用于验证请求来源
    pub linq_signing_secret: Option<Arc<str>>,

    /// Nextcloud Talk 通道实例（可选）
    pub nextcloud_talk: Option<Arc<NextcloudTalkChannel>>,

    /// Nextcloud Talk Webhook 密钥，用于签名验证
    pub nextcloud_talk_webhook_secret: Option<Arc<str>>,

    /// Wati 通道实例（可选）
    pub wati: Option<Arc<WatiChannel>>,

    /// QQ 通道实例（可选）
    pub qq: Option<Arc<QQChannel>>,

    /// 是否启用 QQ Webhook 功能
    pub qq_webhook_enabled: bool,
}

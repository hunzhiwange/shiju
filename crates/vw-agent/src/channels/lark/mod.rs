//! Lark/飞书通道模块
//!
//! 本模块实现了 Lark（海外版飞书）和飞书（国内版）的通道集成，支持机器人通过这两种即时通讯平台
//! 接收和发送消息。
//!
//! # 主要功能
//!
//! - **双模式接收**：支持 WebSocket 长连接和 HTTP Webhook 两种事件接收模式
//! - **多平台支持**：自动适配 Lark 和飞书不同的 API 端点
//! - **租户令牌管理**：自动获取和刷新 tenant_access_token
//! - **消息去重**：WebSocket 模式下防止消息重复分发
//! - **权限控制**：基于用户白名单的访问控制
//!
//! # 子模块
//!
//! - `ack`：消息确认和响应反应处理
//! - `constants`：平台相关常量定义
//! - `parsing`：消息解析和响应判断逻辑
//! - `platform`：平台类型和端点配置
//! - `token`：令牌获取和缓存管理
//! - `types`：Lark API 类型定义
//! - `webhook`：HTTP Webhook 服务器实现
//! - `ws`：WebSocket 客户端实现
//!
//! # 示例
//!
//! ```ignore
//! use vibewindow::app::agent::channels::lark::LarkChannel;
//! use vibewindow::app::agent::config::schema::LarkConfig;
//!
//! let config = LarkConfig {
//!     app_id: "your_app_id".to_string(),
//!     app_secret: "your_secret".to_string(),
//!     ..Default::default()
//! };
//!
//! let channel = LarkChannel::from_config(&config);
//! ```

mod ack;
mod constants;
mod parsing;
mod platform;
mod token;
mod types;
mod webhook;
mod ws;

use super::traits::{Channel, SendMessage};
use async_trait::async_trait;
use platform::LarkPlatform;
use std::collections::HashMap;
use std::sync::{Arc, RwLock as StdRwLock};
use token::CachedTenantToken;
use tokio::sync::RwLock;

/// 重导出 Lark 确认相关类型和函数
#[allow(unused_imports)]
pub(crate) use ack::{
    LarkAckLocale, detect_lark_ack_locale, map_locale_tag, random_lark_ack_reaction,
};

/// 重导出平台常量
#[allow(unused_imports)]
pub(crate) use constants::{
    FEISHU_BASE_URL, FEISHU_WS_BASE_URL, LARK_ACK_REACTIONS_EN, LARK_ACK_REACTIONS_JA,
    LARK_ACK_REACTIONS_ZH_CN, LARK_ACK_REACTIONS_ZH_TW, LARK_BASE_URL, LARK_DEFAULT_TOKEN_TTL,
    LARK_IMAGE_DOWNLOAD_FALLBACK_TEXT, LARK_INVALID_ACCESS_TOKEN_CODE, LARK_WS_BASE_URL,
};

/// 重导出消息解析函数
#[allow(unused_imports)]
pub(crate) use parsing::{normalize_group_reply_allowed_sender_ids, should_respond_in_group};

/// 重导出令牌管理函数
#[allow(unused_imports)]
pub(crate) use token::{ensure_lark_send_success, should_refresh_lark_tenant_token};
#[cfg(test)]
pub(crate) use token::{extract_lark_token_ttl_seconds, next_token_refresh_deadline};

/// 重导出 WebSocket 消息类型
#[allow(unused_imports)]
pub(crate) use tokio_tungstenite::tungstenite::Message as WsMsg;

/// 重导出 WebSocket 接收时间刷新判断函数
#[allow(unused_imports)]
pub(crate) use ws::should_refresh_last_recv;

/// 通道消息类型别名
pub(crate) type ChannelMessage = super::traits::ChannelMessage;

/// Lark/飞书通道实现
///
/// 该结构体实现了 `Channel` trait，提供与 Lark 和飞书平台的完整集成能力。
/// 支持两种事件接收模式，通过配置中的 `receive_mode` 参数进行选择：
///
/// - **`websocket`**（默认）：使用持久化 WebSocket 长连接接收事件，无需公网 URL
/// - **`webhook`**：启动 HTTP 回调服务器接收事件，需要公网可访问的 HTTPS 端点
///
/// # 字段说明
///
/// * `app_id` - Lark/飞书应用的 App ID
/// * `app_secret` - Lark/飞书应用的 App Secret
/// * `verification_token` - 用于验证 Webhook 请求的令牌
/// * `port` - Webhook 模式下监听的端口号（可选）
/// * `allowed_users` - 允许与机器人交互的用户 open_id 白名单，`"*"` 表示所有用户
/// * `group_reply_allowed_sender_ids` - 在群聊中允许触发回复的发送者 ID 列表
/// * `resolved_bot_open_id` - 运行时通过 `/bot/v3/info` 接口解析的机器人 open_id
/// * `mention_only` - 是否仅在@机器人时才响应群聊消息
/// * `platform` - 平台类型（Lark 或飞书）
/// * `receive_mode` - 事件接收模式（WebSocket 或 Webhook）
/// * `tenant_token` - 缓存的租户访问令牌
/// * `ws_seen_ids` - WebSocket 消息去重集合，记录最近约 30 分钟内已处理的消息 ID
///
/// # 示例
///
/// ```ignore
/// // 创建 Lark 通道
/// let channel = LarkChannel::new(
///     "cli_xxx".to_string(),
///     "secret".to_string(),
///     "verify_token".to_string(),
///     Some(8080),
///     vec!["ou_xxx".to_string()],
///     true,
/// );
///
/// // 从配置创建
/// let config = LarkConfig::default();
/// let channel = LarkChannel::from_config(&config);
/// ```
#[derive(Clone)]
pub struct LarkChannel {
    /// Lark/飞书应用的 App ID
    app_id: String,
    /// Lark/飞书应用的 App Secret
    app_secret: String,
    /// 用于验证 Webhook 请求的令牌
    verification_token: String,
    /// Webhook 模式下监听的端口号
    port: Option<u16>,
    /// 允许与机器人交互的用户 open_id 白名单
    allowed_users: Vec<String>,
    /// 在群聊中允许触发回复的发送者 ID 列表
    group_reply_allowed_sender_ids: Vec<String>,
    /// 运行时通过 `/bot/v3/info` 接口解析的机器人 open_id
    resolved_bot_open_id: Arc<StdRwLock<Option<String>>>,
    /// 是否仅在@机器人时才响应群聊消息
    mention_only: bool,
    /// 平台类型（Lark 或飞书）
    platform: LarkPlatform,
    #[cfg(test)]
    api_base_override: Option<String>,
    /// 事件接收模式：WebSocket 长连接或 HTTP Webhook
    receive_mode: crate::app::agent::config::schema::LarkReceiveMode,
    /// 缓存的租户访问令牌
    tenant_token: Arc<RwLock<Option<CachedTenantToken>>>,
    /// 消息去重集合：记录 WebSocket 消息 ID 及其首次见到的时间
    /// 用于防止消息重复分发，保留最近约 30 分钟的数据
    ws_seen_ids: Arc<RwLock<HashMap<String, std::time::Instant>>>,
}

impl LarkChannel {
    /// 创建新的 Lark 通道实例
    ///
    /// 使用默认的 Lark 平台配置创建通道。如需指定飞书平台，
    /// 请使用 [`from_feishu_config`](Self::from_feishu_config) 方法。
    ///
    /// # 参数
    ///
    /// * `app_id` - Lark 应用的 App ID
    /// * `app_secret` - Lark 应用的 App Secret
    /// * `verification_token` - 用于验证 Webhook 请求的令牌
    /// * `port` - Webhook 模式下的监听端口（传 `None` 使用默认端口）
    /// * `allowed_users` - 允许的用户 open_id 列表，传入 `vec!["*"]` 允许所有用户
    /// * `mention_only` - 是否仅在群聊中被@时才响应
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `LarkChannel` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = LarkChannel::new(
    ///     "cli_a1b2c3d4e5f6".to_string(),
    ///     "your_app_secret".to_string(),
    ///     "verification_token".to_string(),
    ///     Some(8443),
    ///     vec!["*".to_string()],
    ///     false,
    /// );
    /// ```
    pub fn new(
        app_id: String,
        app_secret: String,
        verification_token: String,
        port: Option<u16>,
        allowed_users: Vec<String>,
        mention_only: bool,
    ) -> Self {
        Self::new_with_platform(
            app_id,
            app_secret,
            verification_token,
            port,
            allowed_users,
            mention_only,
            LarkPlatform::Lark,
        )
    }

    /// 创建指定平台的 Lark 通道实例
    ///
    /// 内部构造函数，允许显式指定平台类型（Lark 或飞书）。
    ///
    /// # 参数
    ///
    /// * `app_id` - 应用的 App ID
    /// * `app_secret` - 应用的 App Secret
    /// * `verification_token` - 验证令牌
    /// * `port` - Webhook 监听端口
    /// * `allowed_users` - 用户白名单
    /// * `mention_only` - 是否仅响应@
    /// * `platform` - 平台类型（Lark 或飞书）
    ///
    /// # 返回值
    ///
    /// 返回初始化的通道实例，所有可变字段设为默认值
    fn new_with_platform(
        app_id: String,
        app_secret: String,
        verification_token: String,
        port: Option<u16>,
        allowed_users: Vec<String>,
        mention_only: bool,
        platform: LarkPlatform,
    ) -> Self {
        Self {
            app_id,
            app_secret,
            verification_token,
            port,
            allowed_users,
            group_reply_allowed_sender_ids: Vec::new(),
            resolved_bot_open_id: Arc::new(StdRwLock::new(None)),
            mention_only,
            platform,
            #[cfg(test)]
            api_base_override: None,
            receive_mode: crate::app::agent::config::schema::LarkReceiveMode::default(),
            tenant_token: Arc::new(RwLock::new(None)),
            ws_seen_ids: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 从 LarkConfig 配置创建通道实例（支持飞书兼容）
    ///
    /// 根据配置中的 `use_feishu` 标志自动选择 Lark 或飞书平台。
    /// 这是为了兼容旧版配置而提供的便捷方法。
    ///
    /// # 参数
    ///
    /// * `config` - Lark 配置对象
    ///
    /// # 返回值
    ///
    /// 返回根据配置创建的通道实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let config = LarkConfig {
    ///     app_id: "cli_xxx".to_string(),
    ///     app_secret: "secret".to_string(),
    ///     use_feishu: true, // 将使用飞书端点
    ///     ..Default::default()
    /// };
    /// let channel = LarkChannel::from_config(&config);
    /// ```
    pub fn from_config(config: &crate::app::agent::config::schema::LarkConfig) -> Self {
        // 根据配置选择平台：use_feishu=true 时使用飞书端点
        let platform = if config.use_feishu { LarkPlatform::Feishu } else { LarkPlatform::Lark };
        let mut ch = Self::new_with_platform(
            config.app_id.clone(),
            config.app_secret.clone(),
            config.verification_token.clone().unwrap_or_default(),
            config.port,
            config.allowed_users.clone(),
            config.effective_group_reply_mode().requires_mention(),
            platform,
        );
        // 设置群聊回复允许的发送者 ID
        ch.group_reply_allowed_sender_ids =
            normalize_group_reply_allowed_sender_ids(config.group_reply_allowed_sender_ids());
        // 设置接收模式
        ch.receive_mode = config.receive_mode.clone();
        ch
    }

    /// 从 LarkConfig 创建 Lark 专用通道
    ///
    /// 强制使用 Lark（海外版）平台，忽略配置中的 `use_feishu` 标志。
    ///
    /// # 参数
    ///
    /// * `config` - Lark 配置对象
    ///
    /// # 返回值
    ///
    /// 返回配置为 Lark 平台的通道实例
    pub fn from_lark_config(config: &crate::app::agent::config::schema::LarkConfig) -> Self {
        let mut ch = Self::new_with_platform(
            config.app_id.clone(),
            config.app_secret.clone(),
            config.verification_token.clone().unwrap_or_default(),
            config.port,
            config.allowed_users.clone(),
            config.effective_group_reply_mode().requires_mention(),
            LarkPlatform::Lark,
        );
        ch.group_reply_allowed_sender_ids =
            normalize_group_reply_allowed_sender_ids(config.group_reply_allowed_sender_ids());
        ch.receive_mode = config.receive_mode.clone();
        ch
    }

    /// 从 FeishuConfig 创建飞书专用通道
    ///
    /// 强制使用飞书（国内版）平台。
    ///
    /// # 参数
    ///
    /// * `config` - 飞书配置对象
    ///
    /// # 返回值
    ///
    /// 返回配置为飞书平台的通道实例
    pub fn from_feishu_config(config: &crate::app::agent::config::schema::FeishuConfig) -> Self {
        let mut ch = Self::new_with_platform(
            config.app_id.clone(),
            config.app_secret.clone(),
            config.verification_token.clone().unwrap_or_default(),
            config.port,
            config.allowed_users.clone(),
            config.effective_group_reply_mode().requires_mention(),
            LarkPlatform::Feishu,
        );
        ch.group_reply_allowed_sender_ids =
            normalize_group_reply_allowed_sender_ids(config.group_reply_allowed_sender_ids());
        ch.receive_mode = config.receive_mode.clone();
        ch
    }

    /// 获取 HTTP 客户端
    ///
    /// 创建配置了平台代理设置的 HTTP 客户端实例。
    fn http_client(&self) -> reqwest::Client {
        crate::app::agent::config::build_runtime_proxy_client(self.platform.proxy_service_key())
    }

    /// 获取通道名称
    ///
    /// 返回当前平台的显示名称（"lark" 或 "feishu"）。
    fn channel_name(&self) -> &'static str {
        self.platform.channel_name()
    }

    /// 获取 API 基础 URL
    ///
    /// 根据平台类型返回对应的 Open API 基础地址。
    fn api_base(&self) -> &str {
        #[cfg(test)]
        if let Some(api_base) = self.api_base_override.as_deref() {
            return api_base;
        }

        self.platform.api_base()
    }

    #[cfg(test)]
    fn with_api_base(mut self, api_base: String) -> Self {
        self.api_base_override = Some(api_base.trim_end_matches('/').to_string());
        self
    }

    /// 获取 WebSocket 基础 URL
    ///
    /// 根据平台类型返回对应的 WebSocket 服务地址。
    fn ws_base(&self) -> &'static str {
        self.platform.ws_base()
    }

    /// 获取租户访问令牌的 API URL
    ///
    /// 构造用于获取 tenant_access_token 的完整 URL。
    fn tenant_access_token_url(&self) -> String {
        format!("{}/auth/v3/tenant_access_token/internal", self.api_base())
    }

    /// 获取机器人信息的 API URL
    ///
    /// 构造用于获取机器人信息的完整 URL（`/bot/v3/info`）。
    fn bot_info_url(&self) -> String {
        format!("{}/bot/v3/info", self.api_base())
    }

    /// 获取发送消息的 API URL
    ///
    /// 构造发送消息的完整 URL，使用 `chat_id` 作为接收者 ID 类型。
    fn send_message_url(&self) -> String {
        format!("{}/im/v1/messages?receive_id_type=chat_id", self.api_base())
    }

    /// 获取消息反应的 API URL
    ///
    /// 构造指定消息的反应操作 URL。
    ///
    /// # 参数
    ///
    /// * `message_id` - 消息 ID
    fn message_reaction_url(&self, message_id: &str) -> String {
        format!("{}/im/v1/messages/{message_id}/reactions", self.api_base())
    }

    /// 获取图片下载的 API URL
    ///
    /// 构造下载指定图片的完整 URL。
    ///
    /// # 参数
    ///
    /// * `image_key` - 图片的唯一标识符
    fn image_download_url(&self, image_key: &str) -> String {
        format!("{}/im/v1/images/{image_key}", self.api_base())
    }

    /// 检查用户是否在白名单中
    ///
    /// 验证指定的用户 open_id 是否被允许与机器人交互。
    /// 如果白名单包含 `"*"`，则所有用户都被允许。
    ///
    /// # 参数
    ///
    /// * `open_id` - 用户的 open_id
    ///
    /// # 返回值
    ///
    /// 如果用户被允许返回 `true`，否则返回 `false`
    fn is_user_allowed(&self, open_id: &str) -> bool {
        self.allowed_users.iter().any(|u| u == "*" || u == open_id)
    }
}

/// 为 LarkChannel 实现 Channel trait
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for LarkChannel {
    /// 获取通道名称
    ///
    /// 返回当前平台的标识符（"lark" 或 "feishu"）。
    fn name(&self) -> &str {
        self.channel_name()
    }

    /// 发送消息到 Lark/飞书
    ///
    /// 将文本消息发送给指定的接收者。该方法会自动处理令牌过期和重试逻辑：
    ///
    /// 1. 获取当前的 tenant_access_token
    /// 2. 发送消息
    /// 3. 如果返回令牌无效错误，刷新令牌后重试一次
    /// 4. 如果重试仍失败，返回错误
    ///
    /// # 参数
    ///
    /// * `message` - 要发送的消息，包含接收者 ID 和消息内容
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`，失败返回错误信息
    ///
    /// # 错误
    ///
    /// - 如果获取令牌失败
    /// - 如果发送请求失败
    /// - 如果令牌刷新后仍然发送失败
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        // 获取租户访问令牌
        let token = self.get_tenant_access_token().await?;
        let url = self.send_message_url();

        // 构造消息体
        let content = serde_json::json!({ "text": message.content }).to_string();
        let body = serde_json::json!({
            "receive_id": message.recipient,
            "msg_type": "text",
            "content": content,
        });

        // 首次发送尝试
        let (status, response) = self.send_text_once(&url, &token, &body).await?;

        // 检查是否需要刷新令牌
        if should_refresh_lark_tenant_token(status, &response) {
            // 令牌已过期或无效，使缓存失效并重试
            self.invalidate_token().await;
            let new_token = self.get_tenant_access_token().await?;
            let (retry_status, retry_response) =
                self.send_text_once(&url, &new_token, &body).await?;

            // 检查重试后是否仍需要刷新令牌（异常情况）
            if should_refresh_lark_tenant_token(retry_status, &retry_response) {
                let sanitized = token::sanitize_lark_body(&retry_response);
                anyhow::bail!(
                    "Lark send failed after token refresh: status={retry_status}, body={sanitized}"
                );
            }

            // 确保重试成功
            ensure_lark_send_success(retry_status, &retry_response, "after token refresh")?;
            return Ok(());
        }

        // 首次发送成功
        ensure_lark_send_success(status, &response, "without token refresh")?;
        Ok(())
    }

    /// 启动事件监听
    ///
    /// 根据配置的接收模式启动对应的事件监听器：
    ///
    /// - **WebSocket 模式**：建立持久化 WebSocket 连接，实时接收推送事件
    /// - **Webhook 模式**：启动 HTTP 服务器，监听平台推送的回调请求
    ///
    /// # 参数
    ///
    /// * `tx` - 用于发送接收到的消息的通道发送端
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`，监听过程中出错返回错误
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        use crate::app::agent::config::schema::LarkReceiveMode;
        match self.receive_mode {
            LarkReceiveMode::Websocket => self.listen_ws(tx).await,
            LarkReceiveMode::Webhook => self.listen_http(tx).await,
        }
    }

    /// 执行健康检查
    ///
    /// 通过尝试获取租户访问令牌来验证通道配置是否正确。
    /// 如果能成功获取令牌，说明配置有效。
    ///
    /// # 返回值
    ///
    /// 配置有效返回 `true`，否则返回 `false`
    async fn health_check(&self) -> bool {
        self.get_tenant_access_token().await.is_ok()
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

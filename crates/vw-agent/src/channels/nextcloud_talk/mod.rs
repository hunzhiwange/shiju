//! Nextcloud Talk 通道模块
//!
//! 本模块实现了 Nextcloud Talk 的 Channel trait，用于支持通过 Nextcloud Talk 进行消息收发。
//!
//! # 工作模式
//!
//! Nextcloud Talk 通道采用 webhook 模式：
//! - **入站消息**：由网关端点 `/nextcloud-talk` 接收 Nextcloud Talk 的 webhook 推送
//! - **出站消息**：通过 Nextcloud Talk OCS API 发送回复消息到指定聊天室
//!
//! # 支持的消息格式
//!
//! 本模块支持两种 webhook payload 格式：
//! - **传统消息事件**：`type = "message"` 的直接消息通知
//! - **Activity Streams 事件**：`type = "create"` 且 `object.type = "note"` 的活动流消息
//!
//! # 安全机制
//!
//! - 基于用户白名单的访问控制（支持通配符 `*` 允许所有用户）
//! - HMAC-SHA256 签名验证，防止消息伪造
//! - 自动过滤机器人消息，防止反馈循环
//!
//! # 示例配置
//!
//! ```json
//! {
//!   "channels": {
//!     "nextcloud_talk": {
//!       "base_url": "https://nextcloud.example.com",
//!       "app_token": "your-bot-token",
//!       "allowed_users": ["alice", "bob"]
//!     }
//!   }
//! }
//! ```

use super::traits::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use uuid::Uuid;

/// Nextcloud Talk 通道实现（webhook 模式）
///
/// 该结构体封装了与 Nextcloud Talk 交互所需的所有配置和客户端。
///
/// # 字段说明
///
/// - `base_url`：Nextcloud 服务器的基础 URL（如 `https://nextcloud.example.com`）
/// - `app_token`：Nextcloud Talk 机器人应用的访问令牌
/// - `allowed_users`：允许与机器人交互的用户 ID 白名单（支持 `"*"` 通配符）
/// - `client`：HTTP 客户端，用于调用 Nextcloud OCS API
///
/// # 消息流程
///
/// 1. Nextcloud Talk 通过 webhook 将消息推送到网关端点
/// 2. 网关验证签名后调用 [`parse_webhook_payload`] 解析消息
/// 3. 经过权限检查后，消息进入代理处理流程
/// 4. 代理生成的回复通过 [`send_to_room`] 发送回 Nextcloud Talk
pub struct NextcloudTalkChannel {
    base_url: String,
    app_token: String,
    allowed_users: Vec<String>,
    client: reqwest::Client,
}

impl NextcloudTalkChannel {
    /// 创建新的 Nextcloud Talk 通道实例
    ///
    /// # 参数
    ///
    /// - `base_url`：Nextcloud 服务器的基础 URL，末尾的斜杠会被自动移除
    /// - `app_token`：用于 API 认证的机器人令牌
    /// - `allowed_users`：允许的用户白名单，使用 `["*"]` 允许所有用户
    ///
    /// # 示例
    ///
    /// ```
    /// let channel = NextcloudTalkChannel::new(
    ///     "https://nextcloud.example.com".to_string(),
    ///     "bot-token-here".to_string(),
    ///     vec!["alice".to_string(), "bob".to_string()],
    /// );
    /// ```
    pub fn new(base_url: String, app_token: String, allowed_users: Vec<String>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            app_token,
            allowed_users,
            client: reqwest::Client::new(),
        }
    }

    /// 提取 actor ID 的规范形式（仅保留最后一个路径段）
    ///
    /// Nextcloud actor ID 可能是完整 URI（如 `https://.../users/alice`）
    /// 或简单用户名。此方法统一提取最后一部分作为标识符。
    ///
    /// # 参数
    ///
    /// - `actor_id`：原始 actor ID 字符串
    ///
    /// # 返回值
    ///
    /// 返回规范化后的 actor ID（最后一个路径段或原字符串）
    ///
    /// # 示例
    ///
    /// ```
    /// assert_eq!(NextcloudTalkChannel::canonical_actor_id("alice"), "alice");
    /// assert_eq!(NextcloudTalkChannel::canonical_actor_id("https://cloud.example.com/users/alice"), "alice");
    /// ```
    fn canonical_actor_id(actor_id: &str) -> &str {
        let trimmed = actor_id.trim();
        trimmed.rsplit('/').next().unwrap_or(trimmed)
    }

    /// 检查指定用户是否在允许列表中
    ///
    /// 权限检查逻辑：
    /// 1. 空 actor ID 直接拒绝
    /// 2. 白名单包含 `"*"` 时允许所有用户
    /// 3. 否则进行不区分大小写的精确匹配（支持完整 ID 和规范形式）
    ///
    /// # 参数
    ///
    /// - `actor_id`：要检查的用户 ID
    ///
    /// # 返回值
    ///
    /// 如果用户被允许返回 `true`，否则返回 `false`
    fn is_user_allowed(&self, actor_id: &str) -> bool {
        let actor_id = actor_id.trim();
        // 拒绝空用户 ID
        if actor_id.is_empty() {
            return false;
        }

        // 通配符 "*" 允许所有用户
        if self.allowed_users.iter().any(|u| u == "*") {
            return true;
        }

        // 规范化后进行不区分大小写的匹配
        let actor_short = Self::canonical_actor_id(actor_id);
        self.allowed_users.iter().any(|allowed| {
            let allowed = allowed.trim();
            if allowed.is_empty() {
                return false;
            }
            let allowed_short = Self::canonical_actor_id(allowed);
            // 支持完整 ID 和规范形式的双向匹配
            allowed.eq_ignore_ascii_case(actor_id)
                || allowed.eq_ignore_ascii_case(actor_short)
                || allowed_short.eq_ignore_ascii_case(actor_id)
                || allowed_short.eq_ignore_ascii_case(actor_short)
        })
    }

    /// 获取当前时间的 Unix 时间戳（秒）
    ///
    /// # 返回值
    ///
    /// 返回自 Unix 纪元以来的秒数，如果系统时间获取失败则返回 0
    fn now_unix_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// 解析时间戳字段为 Unix 秒数
    ///
    /// 支持多种格式：
    /// - JSON 数字：直接作为时间戳
    /// - JSON 字符串：尝试解析为数字
    /// - 毫秒时间戳：自动转换为秒（值大于 1万亿时）
    ///
    /// # 参数
    ///
    /// - `value`：JSON 值的可选引用
    ///
    /// # 返回值
    ///
    /// 返回 Unix 时间戳（秒），解析失败时返回当前时间
    fn parse_timestamp_secs(value: Option<&serde_json::Value>) -> u64 {
        let raw = match value {
            Some(serde_json::Value::Number(num)) => num.as_u64(),
            Some(serde_json::Value::String(s)) => s.trim().parse::<u64>().ok(),
            _ => None,
        }
        .unwrap_or_else(Self::now_unix_secs);

        // 部分负载数据使用毫秒时间戳，需要转换为秒
        if raw > 1_000_000_000_000 { raw / 1000 } else { raw }
    }

    /// 将 JSON 值转换为字符串
    ///
    /// 仅支持字符串和数字类型，其他类型返回 `None`。
    ///
    /// # 参数
    ///
    /// - `value`：可选的 JSON 值引用
    ///
    /// # 返回值
    ///
    /// 转换后的字符串，如果类型不支持则返回 `None`
    fn value_to_string(value: Option<&serde_json::Value>) -> Option<String> {
        match value {
            Some(serde_json::Value::String(s)) => Some(s.clone()),
            Some(serde_json::Value::Number(n)) => Some(n.to_string()),
            _ => None,
        }
    }

    /// 从 Activity Streams 2.0 对象中提取消息内容
    ///
    /// Activity Streams 负载通常将消息文本嵌入在 `object.content` 字段中，
    /// 可能是纯文本或包含 `message` 字段的 JSON 字符串。
    ///
    /// # 参数
    ///
    /// - `payload`：完整的 webhook payload
    ///
    /// # 返回值
    ///
    /// 提取的消息内容，如果无法提取则返回 `None`
    ///
    /// # 内容提取逻辑
    ///
    /// 1. 尝试从 `payload.object.content` 获取内容
    /// 2. 如果内容是字符串，尝试解析为 JSON 并提取 `message` 字段
    /// 3. 如果内容是对象，直接提取 `message` 字段
    fn extract_content_from_as2_object(payload: &serde_json::Value) -> Option<String> {
        let Some(content_value) = payload.get("object").and_then(|obj| obj.get("content")) else {
            return None;
        };

        let content = match content_value {
            serde_json::Value::String(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return None;
                }

                // Activity Streams 负载常将消息文本作为 JSON 嵌入在 object.content 中
                if let Ok(decoded) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    if let Some(message) = decoded.get("message").and_then(|v| v.as_str()) {
                        let message = message.trim();
                        if !message.is_empty() {
                            return Some(message.to_string());
                        }
                    }
                }

                trimmed.to_string()
            }
            serde_json::Value::Object(map) => map
                .get("message")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|message| !message.is_empty())
                .map(ToOwned::to_owned)?,
            _ => return None,
        };

        if content.is_empty() { None } else { Some(content) }
    }

    /// 解析 Nextcloud Talk webhook payload 为通道消息
    ///
    /// 该方法是消息入站的核心处理逻辑，负责将原始 webhook 数据转换为
    /// 标准的 [`ChannelMessage`] 结构。
    ///
    /// # 支持的 Payload 格式
    ///
    /// ## 传统消息事件（type = "message"）
    ///
    /// ```json
    /// {
    ///   "type": "message",
    ///   "message": {
    ///     "actorType": "users",
    ///     "actorId": "alice",
    ///     "message": "Hello, bot!",
    ///     "timestamp": 1234567890,
    ///     "token": "room-token-here"
    ///   }
    /// }
    /// ```
    ///
    /// ## Activity Streams 事件（type = "create", object.type = "note"）
    ///
    /// ```json
    /// {
    ///   "type": "create",
    ///   "object": {
    ///     "type": "note",
    ///     "token": "room-token-here",
    ///     "content": "{\"message\": \"Hello from AS2!\"}",
    ///     "id": "message-123"
    ///   },
    ///   "actor": {
    ///     "type": "Person",
    ///     "id": "alice"
    ///   }
    /// }
    /// ```
    ///
    /// # 参数
    ///
    /// - `payload`：webhook 请求体的 JSON 数据
    ///
    /// # 返回值
    ///
    /// 返回解析后的消息向量，通常包含 0 或 1 条消息：
    /// - 空向量：消息被过滤（非消息类型、机器人消息、未授权用户等）
    /// - 包含消息：成功解析的消息
    ///
    /// # 过滤规则
    ///
    /// 1. **事件类型过滤**：仅处理 `type = "message"` 或 `type = "create"`
    /// 2. **机器人消息过滤**：忽略 `actorType = "bots"` 或 `"application"`
    /// 3. **用户权限过滤**：仅接受白名单用户的消息
    /// 4. **系统消息过滤**：忽略包含 `systemMessage` 字段的消息
    /// 5. **消息类型过滤**：传统事件仅处理 `messageType = "comment"`
    pub fn parse_webhook_payload(&self, payload: &serde_json::Value) -> Vec<ChannelMessage> {
        let mut messages = Vec::new();

        // 获取事件类型并确定处理方式
        let event_type = payload.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let is_legacy_message_event = event_type.eq_ignore_ascii_case("message");
        let is_activity_streams_event = event_type.eq_ignore_ascii_case("create");

        // 跳过不支持的事件类型
        if !is_legacy_message_event && !is_activity_streams_event {
            tracing::debug!("Nextcloud Talk: skipping non-message event: {event_type}");
            return messages;
        }

        // Activity Streams 事件需要额外的 object.type 检查
        if is_activity_streams_event {
            let object_type = payload
                .get("object")
                .and_then(|obj| obj.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !object_type.eq_ignore_ascii_case("note") {
                tracing::debug!(
                    "Nextcloud Talk: skipping Activity Streams event with unsupported object.type: {object_type}"
                );
                return messages;
            }
        }

        let message_obj = payload.get("message");

        // 提取房间令牌（支持多种字段位置）
        let room_token = payload
            .get("object")
            .and_then(|obj| obj.get("token"))
            .and_then(|v| v.as_str())
            .or_else(|| message_obj.and_then(|msg| msg.get("token")).and_then(|v| v.as_str()))
            .or_else(|| {
                payload.get("target").and_then(|target| target.get("id")).and_then(|v| v.as_str())
            })
            .map(str::trim)
            .filter(|token| !token.is_empty());

        let Some(room_token) = room_token else {
            tracing::warn!("Nextcloud Talk: missing room token in webhook payload");
            return messages;
        };

        // 提取 actor 类型（支持多种字段位置）
        let actor_type = message_obj
            .and_then(|msg| msg.get("actorType"))
            .and_then(|v| v.as_str())
            .or_else(|| payload.get("actorType").and_then(|v| v.as_str()))
            .or_else(|| {
                payload.get("actor").and_then(|actor| actor.get("type")).and_then(|v| v.as_str())
            })
            .unwrap_or("");

        // 忽略机器人发起的消息，防止反馈循环
        if actor_type.eq_ignore_ascii_case("bots") || actor_type.eq_ignore_ascii_case("application")
        {
            tracing::debug!("Nextcloud Talk: skipping bot-originated message");
            return messages;
        }

        // 提取 actor ID（支持多种字段位置）
        let actor_id = message_obj
            .and_then(|msg| msg.get("actorId"))
            .and_then(|v| v.as_str())
            .or_else(|| payload.get("actorId").and_then(|v| v.as_str()))
            .or_else(|| {
                payload.get("actor").and_then(|actor| actor.get("id")).and_then(|v| v.as_str())
            })
            .map(str::trim)
            .filter(|id| !id.is_empty());

        let Some(actor_id) = actor_id else {
            tracing::warn!("Nextcloud Talk: missing actorId in webhook payload");
            return messages;
        };
        let sender_id = Self::canonical_actor_id(actor_id);

        // 权限检查
        if !self.is_user_allowed(actor_id) {
            tracing::warn!(
                "Nextcloud Talk: ignoring message from unauthorized actor: {actor_id}. \
                Add it to channels.nextcloud_talk.allowed_users in system settings."
            );
            return messages;
        }

        // 传统消息事件需要额外的 messageType 检查
        if is_legacy_message_event {
            let message_type = message_obj
                .and_then(|msg| msg.get("messageType"))
                .and_then(|v| v.as_str())
                .unwrap_or("comment");
            if !message_type.eq_ignore_ascii_case("comment") {
                tracing::debug!("Nextcloud Talk: skipping non-comment messageType: {message_type}");
                return messages;
            }
        }

        // 忽略纯系统消息（如用户加入、离开等通知）
        if is_legacy_message_event {
            let has_system_message = message_obj
                .and_then(|msg| msg.get("systemMessage"))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .is_some_and(|value| !value.is_empty());
            if has_system_message {
                tracing::debug!("Nextcloud Talk: skipping system message event");
                return messages;
            }
        }

        // 提取消息内容（优先从 message 字段，回退到 AS2 object.content）
        let content = message_obj
            .and_then(|msg| msg.get("message"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|content| !content.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| Self::extract_content_from_as2_object(payload));

        let Some(content) = content else {
            return messages;
        };

        // 提取消息 ID（回退到 UUID）
        let message_id = Self::value_to_string(message_obj.and_then(|msg| msg.get("id")))
            .or_else(|| Self::value_to_string(payload.get("object").and_then(|obj| obj.get("id"))))
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // 解析时间戳
        let timestamp = Self::parse_timestamp_secs(
            message_obj.and_then(|msg| msg.get("timestamp")).or_else(|| payload.get("timestamp")),
        );

        // 构建标准通道消息
        messages.push(ChannelMessage {
            id: message_id,
            reply_target: room_token.to_string(),
            sender: sender_id.to_string(),
            content,
            channel: "nextcloud_talk".to_string(),
            timestamp,
            thread_ts: None,
        });

        messages
    }

    /// 向指定聊天室发送消息
    ///
    /// 通过 Nextcloud Talk OCS API 发送消息到指定的聊天室。
    ///
    /// # 参数
    ///
    /// - `room_token`：目标聊天室的令牌（从 webhook payload 中提取）
    /// - `content`：要发送的消息内容
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：消息发送成功
    /// - `Err(...)`：API 调用失败（网络错误、认证失败、服务器错误等）
    ///
    /// # API 端点
    ///
    /// `POST /ocs/v2.php/apps/spreed/api/v1/chat/{room_token}`
    ///
    /// # 请求头
    ///
    /// - `Authorization: Bearer {app_token}`
    /// - `OCS-APIRequest: true`
    /// - `Accept: application/json`
    ///
    /// # 错误处理
    ///
    /// 失败时会记录包含净化后响应体的错误日志，避免泄漏敏感信息。
    async fn send_to_room(&self, room_token: &str, content: &str) -> anyhow::Result<()> {
        let encoded_room = urlencoding::encode(room_token);
        let url = format!(
            "{}/ocs/v2.php/apps/spreed/api/v1/chat/{}?format=json",
            self.base_url, encoded_room
        );

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.app_token)
            .header("OCS-APIRequest", "true")
            .header("Accept", "application/json")
            .json(&serde_json::json!({ "message": content }))
            .send()
            .await?;

        if response.status().is_success() {
            return Ok(());
        }

        // 错误响应处理：记录状态码和净化后的响应体
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let sanitized = crate::app::agent::providers::sanitize_api_error(&body);
        tracing::error!("Nextcloud Talk send failed: {status} — {sanitized}");
        anyhow::bail!("Nextcloud Talk API error: {status}");
    }
}

/// Channel trait 实现
///
/// 为 NextcloudTalkChannel 实现通用的 Channel 接口，使其能够被代理运行时统一调度。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for NextcloudTalkChannel {
    /// 返回通道标识符
    ///
    /// # 返回值
    ///
    /// 固定返回 `"nextcloud_talk"`
    fn name(&self) -> &str {
        "nextcloud_talk"
    }

    /// 发送出站消息
    ///
    /// 将代理生成的回复发送回 Nextcloud Talk 聊天室。
    ///
    /// # 参数
    ///
    /// - `message`：要发送的消息，其中 `recipient` 字段包含目标房间令牌
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：发送成功
    /// - `Err(...)`：发送失败
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.send_to_room(&message.recipient, &message.content).await
    }

    /// 启动消息监听（webhook 模式）
    ///
    /// Nextcloud Talk 通道使用 webhook 模式接收消息，因此此方法不执行实际的监听逻辑，
    /// 而是保持任务存活，实际的入站消息由网关的 `/nextcloud-talk` 端点处理。
    ///
    /// # 参数
    ///
    /// - `_tx`：消息发送通道（webhook 模式下未使用）
    ///
    /// # 返回值
    ///
    /// 该方法永远不会返回，通过无限循环保持任务存活
    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        tracing::info!(
            "Nextcloud Talk channel active (webhook mode). \
            Configure Nextcloud Talk bot webhook to POST to your gateway's /nextcloud-talk endpoint."
        );

        // 保持任务存活；入站事件由网关 webhook 处理器处理
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }

    /// 执行健康检查
    ///
    /// 通过请求 Nextcloud 的状态端点检查服务器是否可达。
    ///
    /// # 返回值
    ///
    /// - `true`：服务器健康且可达
    /// - `false`：服务器不可达或返回错误状态
    async fn health_check(&self) -> bool {
        let url = format!("{}/status.php", self.base_url);

        self.client.get(&url).send().await.map(|r| r.status().is_success()).unwrap_or(false)
    }
}

/// 验证 Nextcloud Talk webhook 签名
///
/// 根据 Nextcloud Talk 机器人官方文档验证 webhook 请求的签名，
/// 确保请求来自可信的 Nextcloud 服务器且未被篡改。
///
/// # 签名算法
///
/// 签名计算公式：`hex(hmac_sha256(secret, X-Nextcloud-Talk-Random + raw_body))`
///
/// 其中：
/// - `secret`：配置的 webhook 密钥
/// - `X-Nextcloud-Talk-Random`：请求头中的随机数
/// - `raw_body`：原始请求体
///
/// # 参数
///
/// - `secret`：配置的 webhook 密钥
/// - `random`：`X-Nextcloud-Talk-Random` 请求头的值
/// - `body`：原始请求体字符串
/// - `signature`：`X-Nextcloud-Talk-Signature` 请求头的值（支持 `sha256=` 前缀）
///
/// # 返回值
///
/// - `true`：签名验证通过
/// - `false`：签名验证失败（密钥不匹配、格式错误或随机数缺失）
///
/// # 示例
///
/// ```rust
/// use crate::app::agent::channels::nextcloud_talk::verify_nextcloud_talk_signature;
///
/// let is_valid = verify_nextcloud_talk_signature(
///     "my-webhook-secret",
///     "random-nonce-value",
///     r#"{"type":"message","message":{...}}"#,
///     "sha256=abc123...",
/// );
///
/// if is_valid {
///     println!("Webhook signature is valid");
/// } else {
///     println!("Invalid signature, rejecting request");
/// }
/// ```
///
/// # 安全注意事项
///
/// - 密钥应通过安全的方式存储和传递（如环境变量或加密配置）
/// - 验证失败时应拒绝处理请求，防止伪造消息
pub fn verify_nextcloud_talk_signature(
    secret: &str,
    random: &str,
    body: &str,
    signature: &str,
) -> bool {
    let random = random.trim();
    // 随机数是必需的，用于防止重放攻击
    if random.is_empty() {
        tracing::warn!("Nextcloud Talk: missing X-Nextcloud-Talk-Random header");
        return false;
    }

    // 移除可选的 "sha256=" 前缀
    let signature_hex = signature.trim().strip_prefix("sha256=").unwrap_or(signature).trim();

    // 解码十六进制签名
    let Ok(provided) = hex::decode(signature_hex) else {
        tracing::warn!("Nextcloud Talk: invalid signature format");
        return false;
    };

    // 计算 HMAC-SHA256
    let payload = format!("{random}{body}");
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(payload.as_bytes());

    // 验证签名（使用常量时间比较防止时序攻击）
    mac.verify_slice(&provided).is_ok()
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

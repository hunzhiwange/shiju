//! Linq 渠道模块
//!
//! 本模块实现了 Linq Partner V3 API 的集成，支持通过 Linq 服务发送和接收
//! iMessage、RCS 和 SMS 消息。
//!
//! # 架构模式
//!
//! Linq 渠道采用 Webhook 推送模式（而非轮询模式）：
//! - 消息通过网关的 `/linq` Webhook 端点接收
//! - `listen` 方法是一个保活占位符，实际的消息处理发生在网关层
//! - 当 Linq 发送 Webhook 事件时，由网关进行处理
//!
//! # 核心功能
//!
//! - 消息发送：通过 Linq API 发送文本消息到指定聊天或电话号码
//! - 消息接收：解析 Webhook 载荷，提取并规范化消息内容
//! - 发送者白名单：基于 E.164 格式电话号码的访问控制
//! - 类型指示器：支持"正在输入"状态的显示
//! - 签名验证：HMAC-SHA256 签名验证以防止伪造请求

use super::traits::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;
use uuid::Uuid;

/// Linq 渠道实现
///
/// 通过 Linq Partner V3 API 提供 iMessage、RCS 和 SMS 消息的收发能力。
/// 该渠道运行在 Webhook 模式（推送式）而非轮询模式。
///
/// # 工作原理
///
/// 1. 消息通过网关的 `/linq` Webhook 端点接收
/// 2. `listen` 方法作为保活占位符
/// 3. 实际消息处理发生在网关层，当 Linq 发送 Webhook 事件时触发
///
/// # 示例
///
/// ```rust,no_run
/// use vibe_agent::channels::linq::LinqChannel;
///
/// let channel = LinqChannel::new(
///     "your-api-token".to_string(),
///     "+1234567890".to_string(),
///     vec!["*".to_string()], // 允许所有发送者
/// );
/// ```
pub struct LinqChannel {
    /// Linq API 认证令牌
    api_token: String,
    /// 机器人的电话号码（E.164 格式，如 +1234567890）
    from_phone: String,
    /// 允许的发送者电话号码白名单，支持 "*" 表示允许所有
    allowed_senders: Vec<String>,
    /// HTTP 客户端，用于调用 Linq API
    client: reqwest::Client,
}

/// Linq API 基础 URL
const LINQ_API_BASE: &str = "https://api.linqapp.com/api/partner/v3";

impl LinqChannel {
    /// 创建新的 Linq 渠道实例
    ///
    /// # 参数
    ///
    /// - `api_token`: Linq Partner API 认证令牌
    /// - `from_phone`: 机器人使用的电话号码（E.164 格式）
    /// - `allowed_senders`: 允许发送消息的电话号码白名单
    ///   - 使用 "*" 允许所有发送者
    ///   - 使用完整 E.164 格式号码（如 "+1234567890"）限制特定发送者
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// let channel = LinqChannel::new(
    ///     "api_token_here".to_string(),
    ///     "+15551234567".to_string(),
    ///     vec!["+15559876543".to_string()], // 仅允许此号码
    /// );
    /// ```
    pub fn new(api_token: String, from_phone: String, allowed_senders: Vec<String>) -> Self {
        Self { api_token, from_phone, allowed_senders, client: reqwest::Client::new() }
    }

    /// 检查发送者电话号码是否在白名单中
    ///
    /// # 参数
    ///
    /// - `phone`: 要检查的电话号码（E.164 格式，如 +1234567890）
    ///
    /// # 返回值
    ///
    /// 如果发送者在白名单中返回 `true`，否则返回 `false`
    ///
    /// # 注意
    ///
    /// 白名单中的 "*" 表示允许所有发送者
    fn is_sender_allowed(&self, phone: &str) -> bool {
        self.allowed_senders.iter().any(|n| n == "*" || n == phone)
    }

    /// 获取机器人的电话号码
    ///
    /// # 返回值
    ///
    /// 返回机器人电话号码的字符串切片（E.164 格式）
    pub fn phone_number(&self) -> &str {
        &self.from_phone
    }

    /// 将媒体部分转换为图片标记
    ///
    /// 解析 Linq 消息中的媒体部分，如果是图片类型则生成标记字符串。
    ///
    /// # 参数
    ///
    /// - `part`: Linq 消息部分的 JSON 值
    ///
    /// # 返回值
    ///
    /// 如果是图片类型，返回 `Some("[IMAGE:url]")` 格式的标记；
    /// 如果不是图片或解析失败，返回 `None`
    ///
    /// # 处理逻辑
    ///
    /// 1. 从 `url` 或 `value` 字段获取媒体源
    /// 2. 检查 `mime_type` 是否为图片类型（以 "image/" 开头）
    /// 3. 仅对图片类型生成标记，其他媒体类型被跳过
    fn media_part_to_image_marker(part: &serde_json::Value) -> Option<String> {
        // 优先从 url 字段获取，其次从 value 字段获取
        let source = part
            .get("url")
            .or_else(|| part.get("value"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())?;

        // 获取并规范化 MIME 类型
        let mime_type = part
            .get("mime_type")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase();

        // 仅处理图片类型
        if !mime_type.starts_with("image/") {
            return None;
        }

        Some(format!("[IMAGE:{source}]"))
    }

    /// 解析来自 Linq 的 Webhook 载荷并提取消息
    ///
    /// 支持解析旧版和当前 Linq v3 Webhook 载荷格式：
    /// - 旧版格式：`data.from`、`data.chat_id`、`data.message.parts`
    /// - 当前格式：`data.sender_handle.handle`、`data.chat.id`、`data.parts`
    ///
    /// # 参数
    ///
    /// - `payload`: Linq Webhook 的 JSON 载荷
    ///
    /// # 返回值
    ///
    /// 返回解析后的消息向量，可能为空（如非消息事件、未授权发送者等）
    ///
    /// # Webhook 载荷示例（旧版格式）
    ///
    /// ```json
    /// {
    ///   "api_version": "v3",
    ///   "event_type": "message.received",
    ///   "event_id": "...",
    ///   "created_at": "...",
    ///   "trace_id": "...",
    ///   "data": {
    ///     "chat_id": "...",
    ///     "from": "+1...",
    ///     "recipient_phone": "+1...",
    ///     "is_from_me": false,
    ///     "service": "iMessage",
    ///     "message": {
    ///       "id": "...",
    ///       "parts": [{ "type": "text", "value": "..." }]
    ///     }
    ///   }
    /// }
    /// ```
    ///
    /// # 处理流程
    ///
    /// 1. 仅处理 `message.received` 事件类型
    /// 2. 跳过机器人自己发送的消息（通过多种标志识别）
    /// 3. 提取并规范化发送者电话号码为 E.164 格式
    /// 4. 验证发送者是否在白名单中
    /// 5. 提取聊天 ID 用于回复路由
    /// 6. 解析消息部分（文本和图片）
    /// 7. 构建标准化的 `ChannelMessage`
    pub fn parse_webhook_payload(&self, payload: &serde_json::Value) -> Vec<ChannelMessage> {
        let mut messages = Vec::new();

        // 仅处理 message.received 事件
        let event_type = payload.get("event_type").and_then(|e| e.as_str()).unwrap_or("");
        if event_type != "message.received" {
            tracing::debug!("Linq: skipping non-message event: {event_type}");
            return messages;
        }

        let Some(data) = payload.get("data") else {
            return messages;
        };

        // 跳过机器人自己发送的消息
        // Linq 可能通过以下方式表示：
        // - 旧版：data.is_from_me
        // - v3：data.sender_handle.is_me
        // - v3 方向标记：data.direction == "outbound"
        let is_from_me = data.get("is_from_me").and_then(|v| v.as_bool()).unwrap_or(false)
            || data
                .get("sender_handle")
                .and_then(|sender| sender.get("is_me"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || matches!(data.get("direction").and_then(|v| v.as_str()), Some("outbound"));
        if is_from_me {
            tracing::debug!("Linq: skipping is_from_me message");
            return messages;
        }

        // 获取发送者电话号码
        // 优先使用旧版的 `from`，然后尝试 v3 的 `sender_handle.handle`
        let Some(from) = data
            .get("from")
            .and_then(|f| f.as_str())
            .or_else(|| data.get("sender").and_then(|f| f.as_str()))
            .or_else(|| {
                data.get("sender_handle")
                    .and_then(|sender| sender.get("handle"))
                    .and_then(|h| h.as_str())
            })
        else {
            return messages;
        };

        // 规范化为 E.164 格式（确保以 + 开头）
        let normalized_from =
            if from.starts_with('+') { from.to_string() } else { format!("+{from}") };

        // 检查白名单
        if !self.is_sender_allowed(&normalized_from) {
            tracing::warn!(
                "Linq: ignoring message from unauthorized sender: {normalized_from}. \
                Add it to channels.linq.allowed_senders in system settings."
            );
            return messages;
        }

        // 获取聊天 ID 用于回复路由
        // 旧版：data.chat_id
        // v3：data.chat.id
        let chat_id = data
            .get("chat_id")
            .and_then(|c| c.as_str())
            .or_else(|| data.get("chat").and_then(|chat| chat.get("id")).and_then(|id| id.as_str()))
            .unwrap_or("")
            .to_string();

        // 从消息部分提取文本
        // 旧版：data.message.parts
        // v3：data.parts
        let Some(parts) = data
            .get("message")
            .and_then(|message| message.get("parts"))
            .and_then(|p| p.as_array())
            .or_else(|| data.get("parts").and_then(|p| p.as_array()))
        else {
            return messages;
        };

        // 解析消息部分：支持文本和图片类型
        let content_parts: Vec<String> = parts
            .iter()
            .filter_map(|part| {
                let part_type = part.get("type").and_then(|t| t.as_str())?;
                match part_type {
                    // 文本部分：直接提取值
                    "text" => part.get("value").and_then(|v| v.as_str()).map(ToString::to_string),
                    // 媒体/图片部分：转换为标记
                    "media" | "image" => {
                        if let Some(marker) = Self::media_part_to_image_marker(part) {
                            Some(marker)
                        } else {
                            tracing::debug!("Linq: skipping unsupported {part_type} part");
                            None
                        }
                    }
                    // 其他类型：跳过
                    _ => {
                        tracing::debug!("Linq: skipping {part_type} part");
                        None
                    }
                }
            })
            .collect();

        // 如果没有有效内容，返回空
        if content_parts.is_empty() {
            return messages;
        }

        // 合并所有内容部分
        let content = content_parts.join("\n").trim().to_string();

        if content.is_empty() {
            return messages;
        }

        // 从 created_at 获取时间戳，或使用当前时间
        let timestamp = payload
            .get("created_at")
            .and_then(|t| t.as_str())
            .and_then(|t| {
                chrono::DateTime::parse_from_rfc3339(t)
                    .ok()
                    .map(|dt| dt.timestamp().cast_unsigned())
            })
            .unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            });

        // 使用 chat_id 作为回复目标，确保回复发送到正确的会话
        let reply_target = if chat_id.is_empty() { normalized_from.clone() } else { chat_id };

        // 构建标准化的渠道消息
        messages.push(ChannelMessage {
            id: Uuid::new_v4().to_string(),
            reply_target,
            sender: normalized_from,
            content,
            channel: "linq".to_string(),
            timestamp,
            thread_ts: None,
        });

        messages
    }
}

/// Channel trait 实现
///
/// 为 LinqChannel 实现 Channel trait，提供标准化的渠道接口。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for LinqChannel {
    /// 获取渠道名称
    ///
    /// # 返回值
    ///
    /// 返回固定字符串 "linq"
    fn name(&self) -> &str {
        "linq"
    }

    /// 发送消息到指定接收者
    ///
    /// 该方法首先尝试向现有聊天发送消息（假设接收者是聊天 ID），
    /// 如果失败且返回 404，则尝试创建新聊天并使用电话号码作为接收者。
    ///
    /// # 参数
    ///
    /// - `message`: 要发送的消息，包含接收者和内容
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`，失败返回错误信息
    ///
    /// # 错误处理
    ///
    /// - 如果 API 调用失败，会记录错误并返回错误信息
    /// - 错误消息中的敏感信息会被清理（sanitized）
    ///
    /// # 发送流程
    ///
    /// 1. 构建消息 JSON 载荷
    /// 2. 尝试发送到现有聊天（POST /chats/{recipient}/messages）
    /// 3. 如果聊天不存在（404），创建新聊天（POST /chats）
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        // 如果 reply_target 看起来像聊天 ID，则发送到现有聊天
        // 否则使用接收者电话号码创建新聊天
        let recipient = &message.recipient;

        // 构建消息体
        let body = serde_json::json!({
            "message": {
                "parts": [{
                    "type": "text",
                    "value": message.content
                }]
            }
        });

        // 尝试发送到现有聊天（recipient 是聊天 ID）
        let url = format!("{LINQ_API_BASE}/chats/{recipient}/messages");

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_token)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if resp.status().is_success() {
            return Ok(());
        }

        // 如果基于聊天 ID 的发送失败且返回 404，尝试创建新聊天
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            let new_chat_body = serde_json::json!({
                "from": self.from_phone,
                "to": [recipient],
                "message": {
                    "parts": [{
                        "type": "text",
                        "value": message.content
                    }]
                }
            });

            let create_resp = self
                .client
                .post(format!("{LINQ_API_BASE}/chats"))
                .bearer_auth(&self.api_token)
                .header("Content-Type", "application/json")
                .json(&new_chat_body)
                .send()
                .await?;

            if !create_resp.status().is_success() {
                let status = create_resp.status();
                let error_body = create_resp.text().await.unwrap_or_default();
                let sanitized = crate::app::agent::providers::sanitize_api_error(&error_body);
                tracing::error!("Linq create chat failed: {status} — {sanitized}");
                anyhow::bail!("Linq API error: {status}");
            }

            return Ok(());
        }

        let status = resp.status();
        let error_body = resp.text().await.unwrap_or_default();
        let sanitized = crate::app::agent::providers::sanitize_api_error(&error_body);
        tracing::error!("Linq send failed: {status} — {sanitized}");
        anyhow::bail!("Linq API error: {status}");
    }

    /// 监听消息（Webhook 模式下的保活占位符）
    ///
    /// Linq 使用 Webhook（推送模式）而非轮询。
    /// 实际的消息通过网关的 /linq 端点接收。
    ///
    /// # 参数
    ///
    /// - `_tx`: 消息发送通道（在此实现中未使用）
    ///
    /// # 返回值
    ///
    /// 该方法会无限循环，直到渠道关闭时被取消
    ///
    /// # 注意
    ///
    /// 需要配置 Linq Webhook 以 POST 到网关的 /linq 端点
    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        tracing::info!(
            "Linq channel active (webhook mode). \
            Configure Linq webhook to POST to your gateway's /linq endpoint."
        );

        // 保持任务活跃 —— 当渠道关闭时会被取消
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }

    /// 健康检查
    ///
    /// 检查是否能够访问 Linq API。
    ///
    /// # 返回值
    ///
    /// - `true`: API 可访问且响应成功
    /// - `false`: API 不可访问或响应失败
    ///
    /// # 实现细节
    ///
    /// 通过调用 `/phonenumbers` 端点来验证 API 连接
    async fn health_check(&self) -> bool {
        let url = format!("{LINQ_API_BASE}/phonenumbers");

        self.client
            .get(&url)
            .bearer_auth(&self.api_token)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// 显示"正在输入"状态
    ///
    /// 在指定聊天中显示"正在输入"指示器。
    ///
    /// # 参数
    ///
    /// - `recipient`: 目标聊天 ID 或电话号码
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`，即使失败也返回 `Ok(())`（仅记录调试日志）
    ///
    /// # 注意
    ///
    /// 此操作失败不会导致消息发送失败
    async fn start_typing(&self, recipient: &str) -> anyhow::Result<()> {
        let url = format!("{LINQ_API_BASE}/chats/{recipient}/typing");

        let resp = self.client.post(&url).bearer_auth(&self.api_token).send().await?;

        if !resp.status().is_success() {
            tracing::debug!("Linq start_typing failed: {}", resp.status());
        }

        Ok(())
    }

    /// 隐藏"正在输入"状态
    ///
    /// 在指定聊天中隐藏"正在输入"指示器。
    ///
    /// # 参数
    ///
    /// - `recipient`: 目标聊天 ID 或电话号码
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`，即使失败也返回 `Ok(())`（仅记录调试日志）
    ///
    /// # 注意
    ///
    /// 此操作失败不会导致消息发送失败
    async fn stop_typing(&self, recipient: &str) -> anyhow::Result<()> {
        let url = format!("{LINQ_API_BASE}/chats/{recipient}/typing");

        let resp = self.client.delete(&url).bearer_auth(&self.api_token).send().await?;

        if !resp.status().is_success() {
            tracing::debug!("Linq stop_typing failed: {}", resp.status());
        }

        Ok(())
    }
}

/// 验证 Linq Webhook 签名
///
/// Linq 使用 HMAC-SHA256 对 `"{timestamp}.{body}"` 进行签名。
/// 签名通过 `X-Webhook-Signature` 头（十六进制编码）发送，
/// 时间戳通过 `X-Webhook-Timestamp` 头发送。
/// 超过 300 秒的时间戳会被拒绝。
///
/// # 参数
///
/// - `secret`: Webhook 签名密钥
/// - `body`: Webhook 请求体的原始字符串
/// - `timestamp`: `X-Webhook-Timestamp` 头的值
/// - `signature`: `X-Webhook-Signature` 头的值
///
/// # 返回值
///
/// - `true`: 签名验证通过且时间戳有效
/// - `false`: 签名验证失败或时间戳无效/过期
///
/// # 安全措施
///
/// 1. 时间戳验证：拒绝超过 300 秒的旧请求（防重放攻击）
/// 2. 常量时间比较：使用 HMAC 验证防止时序攻击
/// 3. 格式清理：支持可选的 "sha256=" 前缀
///
/// # 示例
///
/// ```rust,no_run
/// use vibe_agent::channels::linq::verify_linq_signature;
///
/// let valid = verify_linq_signature(
///     "webhook_secret",
///     r#"{"event_type":"message.received"}"#,
///     "1234567890",
///     "sha256=abc123...",
/// );
/// ```
pub fn verify_linq_signature(secret: &str, body: &str, timestamp: &str, signature: &str) -> bool {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    // 拒绝过期的时间戳（超过 300 秒）
    if let Ok(ts) = timestamp.parse::<i64>() {
        let now = chrono::Utc::now().timestamp();
        if (now - ts).unsigned_abs() > 300 {
            tracing::warn!("Linq: rejecting stale webhook timestamp ({ts}, now={now})");
            return false;
        }
    } else {
        tracing::warn!("Linq: invalid webhook timestamp: {timestamp}");
        return false;
    }

    // 对 "{timestamp}.{body}" 计算 HMAC-SHA256
    let message = format!("{timestamp}.{body}");
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(message.as_bytes());

    // 清理签名格式（移除可选的 "sha256=" 前缀）
    let signature_hex = signature.trim().strip_prefix("sha256=").unwrap_or(signature);
    let Ok(provided) = hex::decode(signature_hex.trim()) else {
        tracing::warn!("Linq: invalid webhook signature format");
        return false;
    };

    // 通过 HMAC 验证进行常量时间比较
    mac.verify_slice(&provided).is_ok()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

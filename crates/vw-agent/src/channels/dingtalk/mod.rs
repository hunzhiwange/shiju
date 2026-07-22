//! 钉钉（DingTalk）通道模块
//!
//! 本模块实现了钉钉机器人的消息通道功能，通过钉钉 Stream Mode WebSocket
//! 建立实时双向通信连接。
//!
//! # 架构概述
//!
//! 钉钉通道的工作流程如下：
//! 1. 向钉钉网关注册连接，获取 WebSocket 端点和票据（ticket）
//! 2. 使用票据建立 WebSocket 长连接
//! 3. 接收来自钉钉服务器的实时消息推送
//! 4. 通过消息中携带的 session webhook URL 发送回复
//!
//! # 消息处理流程
//!
//! - **SYSTEM 消息**：心跳/保活消息，需要回复 pong 保持连接
//! - **EVENT/CALLBACK 消息**：用户消息，解析后转发给上层处理
//!
//! # 会话管理
//!
//! 钉钉的回复机制采用"请求中携带回复地址"的模式：
//! 每条收到的消息都会携带一个 `sessionWebhook` URL，
//! 后续回复必须使用该 URL 发送。本模块通过 `session_webhooks`
//! 字段维护 chat_id 到 webhook URL 的映射。
//!
//! # 示例
//!
//! ```ignore
//! use crate::app::agent::channels::dingtalk::DingTalkChannel;
//! use crate::app::agent::channels::traits::Channel;
//!
//! let channel = DingTalkChannel::new(
//!     "your_client_id".to_string(),
//!     "your_client_secret".to_string(),
//!     vec!["*".to_string()], // 允许所有用户
//! );
//!
//! // 健康检查
//! let healthy = channel.health_check().await;
//!
//! // 发送消息（需要先建立会话）
//! channel.send(&SendMessage {
//!     recipient: "chat_id".to_string(),
//!     content: "Hello!".to_string(),
//!     subject: Some("Title".to_string()),
//! }).await?;
//! ```

use super::traits::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

/// 钉钉机器人回调消息的订阅主题
///
/// 该主题用于订阅钉钉机器人接收到的所有消息，
/// 包括单聊、群聊中的 @ 消息等。
const DINGTALK_BOT_CALLBACK_TOPIC: &str = "/v1.0/im/bot/messages/get";
const DINGTALK_GATEWAY_CONNECTIONS_OPEN_URL: &str =
    "https://api.dingtalk.com/v1.0/gateway/connections/open";

/// 钉钉通道实现
///
/// 通过钉钉 Stream Mode WebSocket 实现实时消息接收，
/// 并使用每条消息携带的 session webhook URL 进行回复。
///
/// # 字段说明
///
/// - `client_id`：钉钉应用的 Client ID
/// - `client_secret`：钉钉应用的 Client Secret
/// - `allowed_users`：允许与机器人交互的用户 ID 列表，
///   使用 `"*"` 表示允许所有用户
/// - `session_webhooks`：会话 webhook URL 缓存，键为 chat_id，
///   值为该会话的回复 webhook URL
///
/// # 安全说明
///
/// - 只处理来自 `allowed_users` 列表中的用户消息
/// - 错误信息会经过脱敏处理，避免泄露敏感数据
pub struct DingTalkChannel {
    /// 钉钉应用的 Client ID，用于身份认证
    client_id: String,
    /// 钉钉应用的 Client Secret，用于身份认证
    client_secret: String,
    /// 允许与机器人交互的用户 ID 列表，"*" 表示允许所有用户
    allowed_users: Vec<String>,
    /// 会话 webhook URL 映射表（chat_id -> webhook URL）
    ///
    /// 钉钉采用"请求中携带回复地址"的模式，每条收到的消息都会
    /// 携带一个唯一的 sessionWebhook URL，后续回复必须使用该 URL。
    /// 此字段维护 chat_id 到 webhook URL 的映射，支持消息回复。
    session_webhooks: Arc<RwLock<HashMap<String, String>>>,
    #[cfg(test)]
    gateway_endpoint: String,
}

/// 钉钉网关注册响应
///
/// 调用钉钉网关连接注册 API 后返回的数据结构，
/// 包含用于建立 WebSocket 连接的端点地址和票据。
#[derive(serde::Deserialize)]
struct GatewayResponse {
    /// WebSocket 端点地址
    endpoint: String,
    /// 连接票据，需要作为 URL 参数传递
    ticket: String,
}

impl DingTalkChannel {
    /// 创建新的钉钉通道实例
    ///
    /// # 参数
    ///
    /// - `client_id`：钉钉应用的 Client ID
    /// - `client_secret`：钉钉应用的 Client Secret
    /// - `allowed_users`：允许与机器人交互的用户 ID 列表，
    ///   传入 `vec!["*"]` 表示允许所有用户
    ///
    /// # 返回值
    ///
    /// 返回初始化后的 `DingTalkChannel` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = DingTalkChannel::new(
    ///     "client_id".to_string(),
    ///     "client_secret".to_string(),
    ///     vec!["user123".to_string(), "user456".to_string()],
    /// );
    /// ```
    pub fn new(client_id: String, client_secret: String, allowed_users: Vec<String>) -> Self {
        Self {
            client_id,
            client_secret,
            allowed_users,
            session_webhooks: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(test)]
            gateway_endpoint: DINGTALK_GATEWAY_CONNECTIONS_OPEN_URL.to_string(),
        }
    }

    #[cfg(test)]
    fn with_gateway_endpoint_for_test(mut self, endpoint: String) -> Self {
        self.gateway_endpoint = endpoint;
        self
    }

    fn gateway_endpoint(&self) -> &str {
        #[cfg(test)]
        {
            &self.gateway_endpoint
        }
        #[cfg(not(test))]
        {
            DINGTALK_GATEWAY_CONNECTIONS_OPEN_URL
        }
    }

    /// 获取配置好代理的 HTTP 客户端
    ///
    /// 使用全局配置的代理设置创建 HTTP 客户端，
    /// 用于与钉钉 API 通信。
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `reqwest::Client` 实例
    fn http_client(&self) -> reqwest::Client {
        crate::app::agent::config::build_runtime_proxy_client("channel.dingtalk")
    }

    /// 检查用户是否在允许列表中
    ///
    /// # 参数
    ///
    /// - `user_id`：待检查的用户 ID
    ///
    /// # 返回值
    ///
    /// 如果用户在允许列表中或允许列表包含 `"*"`，返回 `true`；
    /// 否则返回 `false`
    fn is_user_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.iter().any(|u| u == "*" || u == user_id)
    }

    fn gateway_registration_body(&self) -> serde_json::Value {
        serde_json::json!({
            "clientId": self.client_id,
            "clientSecret": self.client_secret,
            "subscriptions": [
                {
                    "type": "CALLBACK",
                    "topic": DINGTALK_BOT_CALLBACK_TOPIC,
                }
            ],
        })
    }

    fn websocket_url(gw: &GatewayResponse) -> String {
        format!("{}?ticket={}", gw.endpoint, gw.ticket)
    }

    fn webhook_reply_body(message: &SendMessage) -> serde_json::Value {
        let title = message.subject.as_deref().unwrap_or("VibeWindow");
        serde_json::json!({
            "msgtype": "markdown",
            "markdown": {
                "title": title,
                "text": message.content,
            }
        })
    }

    fn frame_message_id(frame: &serde_json::Value) -> &str {
        frame.get("headers").and_then(|h| h.get("messageId")).and_then(|m| m.as_str()).unwrap_or("")
    }

    fn stream_response_frame(message_id: &str) -> serde_json::Value {
        serde_json::json!({
            "code": 200,
            "headers": {
                "contentType": "application/json",
                "messageId": message_id,
            },
            "message": "OK",
            "data": "",
        })
    }

    fn channel_message_from_data(&self, data: &serde_json::Value) -> Option<ChannelMessage> {
        let content = data
            .get("text")
            .and_then(|t| t.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .trim();

        if content.is_empty() {
            return None;
        }

        let sender_id = data.get("senderStaffId").and_then(|s| s.as_str()).unwrap_or("unknown");

        if !self.is_user_allowed(sender_id) {
            tracing::warn!("钉钉：忽略未授权用户的消息: {sender_id}");
            return None;
        }

        let chat_id = Self::resolve_chat_id(data, sender_id);

        Some(ChannelMessage {
            id: Uuid::new_v4().to_string(),
            sender: sender_id.to_string(),
            reply_target: chat_id,
            content: content.to_string(),
            channel: "dingtalk".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            thread_ts: None,
        })
    }

    async fn cache_session_webhook(
        &self,
        chat_id: &str,
        sender_id: &str,
        data: &serde_json::Value,
    ) {
        if let Some(webhook) = data.get("sessionWebhook").and_then(|w| w.as_str()) {
            let webhook = webhook.to_string();
            let mut webhooks = self.session_webhooks.write().await;
            webhooks.insert(chat_id.to_string(), webhook.clone());
            webhooks.insert(sender_id.to_string(), webhook);
        }
    }

    /// 解析流数据帧中的业务数据
    ///
    /// 钉钉 WebSocket 帧的 `data` 字段可能是 JSON 字符串或 JSON 对象，
    /// 此方法统一处理这两种情况。
    ///
    /// # 参数
    ///
    /// - `frame`：原始 WebSocket 帧的 JSON 值
    ///
    /// # 返回值
    ///
    /// - `Some(serde_json::Value)`：成功解析后的业务数据
    /// - `None`：解析失败或数据格式不正确
    fn parse_stream_data(frame: &serde_json::Value) -> Option<serde_json::Value> {
        match frame.get("data") {
            // data 字段是字符串时，需要再次解析为 JSON
            Some(serde_json::Value::String(raw)) => serde_json::from_str(raw).ok(),
            // data 字段已经是对象时，直接返回
            Some(serde_json::Value::Object(_)) => frame.get("data").cloned(),
            _ => None,
        }
    }

    /// 解析会话的聊天 ID
    ///
    /// 根据会话类型确定用于回复的聊天 ID：
    /// - 单聊：使用发送者 ID（sender_id）
    /// - 群聊：使用会话 ID（conversationId）
    ///
    /// # 参数
    ///
    /// - `data`：消息数据对象
    /// - `sender_id`：消息发送者的用户 ID
    ///
    /// # 返回值
    ///
    /// 返回用于回复的目标聊天 ID
    ///
    /// # 说明
    ///
    /// `conversationType` 字段：
    /// - 值为 "1" 或 1 表示单聊
    /// - 其他值表示群聊
    fn resolve_chat_id(data: &serde_json::Value, sender_id: &str) -> String {
        // 判断是否为单聊会话
        // conversationType 为 "1" 或 1 表示单聊
        let is_private_chat = data
            .get("conversationType")
            .and_then(|value| {
                value.as_str().map(|v| v == "1").or_else(|| value.as_i64().map(|v| v == 1))
            })
            .unwrap_or(true); // 默认视为单聊

        if is_private_chat {
            // 单聊直接使用发送者 ID
            sender_id.to_string()
        } else {
            // 群聊使用会话 ID，若无则回退到发送者 ID
            data.get("conversationId").and_then(|c| c.as_str()).unwrap_or(sender_id).to_string()
        }
    }

    /// 向钉钉网关注册连接
    ///
    /// 调用钉钉 API 注册一个 Stream Mode 连接，获取 WebSocket 端点和票据。
    /// 该票据用于后续建立 WebSocket 长连接。
    ///
    /// # 返回值
    ///
    /// - `Ok(GatewayResponse)`：注册成功，包含端点和票据
    /// - `Err`：注册失败，可能原因包括：
    ///   - 网络错误
    ///   - 认证失败（client_id/client_secret 无效）
    ///   - API 返回错误
    ///
    /// # 错误处理
    ///
    /// API 错误信息会经过脱敏处理（`sanitize_api_error`），
    /// 避免在日志中泄露敏感数据。
    async fn register_connection(&self) -> anyhow::Result<GatewayResponse> {
        // 构建注册请求体
        let body = self.gateway_registration_body();

        // 发送注册请求到钉钉网关
        let resp = self.http_client().post(self.gateway_endpoint()).json(&body).send().await?;

        // 处理非成功响应
        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            // 脱敏处理错误信息，避免泄露敏感数据
            let sanitized = crate::app::agent::providers::sanitize_api_error(&err);
            anyhow::bail!("钉钉网关注册失败 ({status}): {sanitized}");
        }

        // 解析响应
        let gw: GatewayResponse = resp.json().await?;
        Ok(gw)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for DingTalkChannel {
    /// 获取通道名称
    ///
    /// # 返回值
    ///
    /// 返回固定字符串 `"dingtalk"`
    fn name(&self) -> &str {
        "dingtalk"
    }

    /// 发送消息到钉钉
    ///
    /// 通过会话 webhook URL 发送回复消息。Webhook URL 是在收到用户消息时
    /// 由钉钉服务器提供的，因此发送前必须先有用户发起的会话。
    ///
    /// # 参数
    ///
    /// - `message`：待发送的消息，包含：
    ///   - `recipient`：目标聊天 ID（用于查找对应的 webhook URL）
    ///   - `content`：消息内容（Markdown 格式）
    ///   - `subject`：消息标题（可选，默认为 "VibeWindow"）
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：消息发送成功
    /// - `Err`：发送失败，可能原因包括：
    ///   - 未找到对应会话的 webhook URL（用户未先发送消息）
    ///   - 网络/HTTP 错误
    ///   - 钉钉 API 返回错误
    ///
    /// # 错误处理
    ///
    /// 如果未找到会话 webhook，会返回明确的错误提示，
    /// 说明需要用户先发送消息建立会话。
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        // 从缓存中获取会话对应的 webhook URL
        let webhooks = self.session_webhooks.read().await;
        let webhook_url = webhooks.get(&message.recipient).ok_or_else(|| {
            anyhow::anyhow!(
                "未找到聊天 {} 的会话 webhook。用户必须先发送消息以建立会话。",
                message.recipient
            )
        })?;

        // 构建消息体，使用 Markdown 格式
        let body = Self::webhook_reply_body(message);

        // 发送消息到 webhook
        let resp = self.http_client().post(webhook_url).json(&body).send().await?;

        // 处理非成功响应
        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            let sanitized = crate::app::agent::providers::sanitize_api_error(&err);
            anyhow::bail!("钉钉 webhook 回复失败 ({status}): {sanitized}");
        }

        Ok(())
    }

    /// 启动消息监听循环
    ///
    /// 建立与钉钉服务器的 WebSocket 长连接，持续监听并处理消息。
    /// 该方法会阻塞直到连接断开或发生不可恢复的错误。
    ///
    /// # 参数
    ///
    /// - `tx`：消息发送通道，用于将收到的消息转发给上层处理
    ///
    /// # 返回值
    ///
    /// - `Err`：连接断开或发生错误（正常情况下该方法不应返回）
    ///
    /// # 消息处理流程
    ///
    /// 1. 向钉钉网关注册连接，获取 WebSocket URL
    /// 2. 建立 WebSocket 连接
    /// 3. 循环处理收到的消息帧：
    ///    - **SYSTEM**：心跳消息，回复 pong 保持连接
    ///    - **EVENT/CALLBACK**：用户消息，解析后转发
    /// 4. 提取并缓存 session webhook URL
    /// 5. 进行用户权限检查
    /// 6. 发送确认（ACK）响应
    /// 7. 将消息通过通道发送给上层
    ///
    /// # 安全说明
    ///
    /// - 只处理来自 `allowed_users` 列表中的用户消息
    /// - 错误信息经过脱敏处理
    /// - 不记录敏感的 webhook URL
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        tracing::info!("钉钉：正在注册网关连接...");

        // 第一步：注册连接获取 WebSocket 端点和票据
        let gw = self.register_connection().await?;
        let ws_url = Self::websocket_url(&gw);

        // 第二步：建立 WebSocket 连接
        tracing::info!("钉钉：正在连接到 Stream WebSocket...");
        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
        let (mut write, mut read) = ws_stream.split();

        tracing::info!("钉钉：已连接，正在监听消息...");

        // 第三步：消息处理循环
        while let Some(msg) = read.next().await {
            // 解析 WebSocket 消息
            let msg = match msg {
                Ok(Message::Text(t)) => t,
                Ok(Message::Close(_)) => break, // 连接关闭
                Err(e) => {
                    // 错误信息脱敏处理
                    let sanitized =
                        crate::app::agent::providers::sanitize_api_error(&e.to_string());
                    tracing::warn!("钉钉 WebSocket 错误: {sanitized}");
                    break;
                }
                _ => continue, // 忽略非文本消息
            };

            // 解析 JSON 帧
            let frame: serde_json::Value = match serde_json::from_str(msg.as_ref()) {
                Ok(v) => v,
                Err(_) => continue, // 忽略无效 JSON
            };

            // 获取帧类型
            let frame_type = frame.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match frame_type {
                "SYSTEM" => {
                    // 系统消息（心跳）：回复 pong 保持连接活跃
                    let pong = Self::stream_response_frame(Self::frame_message_id(&frame));

                    // 发送 pong，失败则断开连接
                    if let Err(e) = write.send(Message::Text(pong.to_string().into())).await {
                        tracing::warn!("钉钉：发送 pong 失败: {e}");
                        break;
                    }
                }
                "EVENT" | "CALLBACK" => {
                    // 事件/回调消息：解析业务数据
                    let data = match Self::parse_stream_data(&frame) {
                        Some(v) => v,
                        None => {
                            tracing::debug!("钉钉：帧无可解析的数据载荷");
                            continue;
                        }
                    };

                    let channel_msg = match self.channel_message_from_data(&data) {
                        Some(message) => message,
                        None => continue,
                    };
                    let sender_id = channel_msg.sender.clone();
                    let chat_id = channel_msg.reply_target.clone();

                    // 缓存会话 webhook URL，用于后续回复
                    self.cache_session_webhook(&chat_id, &sender_id, &data).await;

                    // 发送确认（ACK）响应
                    let ack = Self::stream_response_frame(Self::frame_message_id(&frame));
                    // ACK 发送失败不中断流程
                    let _ = write.send(Message::Text(ack.to_string().into())).await;

                    // 发送到处理通道，失败则断开连接
                    if tx.send(channel_msg).await.is_err() {
                        tracing::warn!("钉钉：消息通道已关闭");
                        break;
                    }
                }
                _ => {} // 忽略其他类型的帧
            }
        }

        // WebSocket 流结束，返回错误
        anyhow::bail!("钉钉 WebSocket 流已结束")
    }

    /// 健康检查
    ///
    /// 通过尝试向钉钉网关注册连接来验证通道是否正常工作。
    ///
    /// # 返回值
    ///
    /// - `true`：注册成功，通道正常
    /// - `false`：注册失败，通道不可用
    ///
    /// # 说明
    ///
    /// 健康检查仅验证认证凭据是否有效以及网络是否通畅，
    /// 不会建立持久的 WebSocket 连接。
    async fn health_check(&self) -> bool {
        self.register_connection().await.is_ok()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

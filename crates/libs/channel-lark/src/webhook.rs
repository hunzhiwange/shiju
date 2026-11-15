//! 飞书 Webhook 事件回调处理模块
//!
//! 本模块实现了飞书（Lark）平台的 HTTP Webhook 回调服务器，用于接收和处理飞书推送的事件消息。
//! 这是飞书集成的传统方式，需要部署在具有公网可访问地址的服务器上。
//!
//! # 主要功能
//!
//! - **URL 验证**：处理飞书开发后台的 URL 验证请求，确保回调地址有效
//! - **事件接收**：监听飞书推送的各类事件消息
//! - **消息解析**：将飞书事件载荷解析为标准化的频道消息格式
//! - **确认反应**：为接收到的消息添加表情反应作为确认
//!
//! # 使用场景
//!
//! 适用于：
//! - 需要处理飞书机器人消息的场景
//! - 需要与飞书群聊进行集成的应用
//! - 需要响应飞书用户@机器人等事件
//!
//! # 注意事项
//!
//! - 此方式需要公网可访问的 HTTP 端点
//! - 新部署建议使用 WebSocket 长连接方式（通过 `listen()` 方法）
//! - 需要在飞书开放平台配置事件订阅地址
//!
//! # 示例
//!
//! ```ignore
//! let channel = LarkChannel::new(config);
//! let (tx, rx) = tokio::sync::mpsc::channel(100);
//! channel.listen_http(tx).await?;
//! ```

use super::LarkChannel;
use super::ack::random_lark_ack_reaction;
use axum::{Json, Router, extract::State, routing::post};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    verification_token: String,
    channel: Arc<LarkChannel>,
    tx: tokio::sync::mpsc::Sender<super::ChannelMessage>,
}

fn build_app_state(
    channel: &LarkChannel,
    tx: tokio::sync::mpsc::Sender<super::ChannelMessage>,
) -> AppState {
    AppState {
        verification_token: channel.verification_token.clone(),
        channel: Arc::new(channel.clone()),
        tx,
    }
}

fn build_lark_webhook_router(state: AppState) -> Router {
    Router::new().route("/lark", post(handle_event)).with_state(state)
}

async fn handle_event(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    if let Some(challenge) = payload.get("challenge").and_then(|c| c.as_str()) {
        let token_ok = payload
            .get("token")
            .and_then(|t| t.as_str())
            .map_or(true, |t| t == state.verification_token);

        if !token_ok {
            return (StatusCode::FORBIDDEN, "invalid token").into_response();
        }

        let resp = serde_json::json!({ "challenge": challenge });
        return (StatusCode::OK, Json(resp)).into_response();
    }

    let messages = state.channel.parse_event_payload_async(&payload).await;

    if !messages.is_empty() {
        if let Some(message_id) =
            payload.pointer("/event/message/message_id").and_then(|m| m.as_str())
        {
            let ack_text = messages.first().map_or("", |msg| msg.content.as_str());
            let ack_emoji = random_lark_ack_reaction(payload.get("event"), ack_text).to_string();
            spawn_ack_reaction(Arc::clone(&state.channel), message_id.to_string(), ack_emoji);
        }
    }

    for msg in messages {
        if state.tx.send(msg).await.is_err() {
            tracing::warn!("Lark: message channel closed");
            break;
        }
    }

    (StatusCode::OK, "ok").into_response()
}

#[cfg(not(test))]
fn spawn_ack_reaction(channel: Arc<LarkChannel>, message_id: String, ack_emoji: String) {
    tokio::spawn(async move {
        channel.try_add_ack_reaction(&message_id, &ack_emoji).await;
    });
}

#[cfg(test)]
fn spawn_ack_reaction(_channel: Arc<LarkChannel>, _message_id: String, _ack_emoji: String) {}

impl LarkChannel {
    /// 启动 HTTP 回调服务器监听飞书事件（传统方式）
    ///
    /// 创建一个 HTTP 服务器，监听飞书平台推送的事件回调。
    /// 这是一个传统的集成方式，需要服务器具有公网可访问的地址。
    ///
    /// # 参数
    ///
    /// * `tx` - 消息发送通道，用于将解析后的事件消息发送到上游处理器
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，服务器将持续运行直到发生错误或被关闭。
    /// 失败时返回错误信息，常见错误包括：
    /// - 端口未配置
    /// - 端口绑定失败
    /// - 服务器运行时错误
    ///
    /// # 配置要求
    ///
    /// 必须在 `[channels_config.lark]` 中配置以下字段：
    /// - `port`: 监听端口号
    /// - `verification_token`: 飞书事件验证令牌（可选，用于安全验证）
    ///
    /// # 工作流程
    ///
    /// 1. 在指定端口启动 HTTP 服务器，监听 `/lark` 路径
    /// 2. 接收飞书推送的事件回调
    /// 3. 处理 URL 验证请求（首次配置时飞书会发送验证请求）
    /// 4. 解析事件载荷为标准消息格式
    /// 5. 为接收到的消息添加确认反应表情
    /// 6. 将消息通过通道发送到上游处理器
    ///
    /// # 迁移建议
    ///
    /// 对于新部署，建议使用 `listen()` 方法（WebSocket 长连接方式），
    /// 该方式无需公网地址，部署更简单。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = LarkChannel::new(config);
    /// let (tx, mut rx) = tokio::sync::mpsc::channel::<ChannelMessage>(100);
    ///
    /// // 在后台启动监听
    /// tokio::spawn(async move {
    ///     if let Err(e) = channel.listen_http(tx).await {
    ///         eprintln!("Lark webhook error: {}", e);
    ///     }
    /// });
    ///
    /// // 接收消息
    /// while let Some(msg) = rx.recv().await {
    ///     println!("Received: {:?}", msg);
    /// }
    /// ```
    pub async fn listen_http(
        &self,
        tx: tokio::sync::mpsc::Sender<super::ChannelMessage>,
    ) -> anyhow::Result<()> {
        // 确保机器人 Open ID 已初始化
        self.ensure_bot_open_id().await;

        // 获取配置的监听端口
        let port = self.port.ok_or_else(|| {
            anyhow::anyhow!("Lark webhook mode requires `port` to be set in [channels_config.lark]")
        })?;

        // 初始化应用状态
        let state = build_app_state(self, tx);

        // 创建 Axum 路由，监听 POST /lark 路径
        let app = build_lark_webhook_router(state);

        // 绑定到指定端口的所有网络接口
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
        tracing::info!("Lark event callback server listening on {addr}");

        // 启动 TCP 监听器
        let listener = tokio::net::TcpListener::bind(addr).await?;

        // 运行 HTTP 服务器（阻塞直到服务器停止）
        Ok(axum::serve(listener, app).await?)
    }
}

#[cfg(test)]
#[path = "webhook_tests.rs"]
mod webhook_tests;

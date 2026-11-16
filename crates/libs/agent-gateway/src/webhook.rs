//! Webhook 处理模块
//!
//! 本模块提供 Webhook HTTP 端点的处理逻辑，作为 VibeWindow 代理系统接收外部事件的主要入口点之一。
//!
//! ## 功能概述
//!
//! - 接收并处理来自外部系统的 HTTP POST 请求
//! - 提取请求元数据（客户端地址、HTTP 头、请求体）
//! - 将请求委托给内部处理逻辑进行进一步处理
//!
//! ## 端点路由
//!
//! - `POST /webhook` - 主 Webhook 端点，用于接收外部事件通知
//!
//! ## 安全考虑
//!
//! Webhook 端点应当配合适当的安全措施使用（如签名验证、IP 白名单等），
//! 具体实现在 `webhook_ingress` 模块中完成。

use super::webhook_ingress::{WebhookState, WebhookChat};
use gateway_types::WebhookBody;
use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;

/// 处理 Webhook POST 请求
///
/// 这是 Webhook 端点的主入口函数，负责接收外部系统发送的 HTTP POST 请求。
/// 该函数提取请求的所有必要信息，并将其委托给内部处理逻辑。
///
/// # 端点
///
/// `POST /webhook`
///
/// # 参数
///
/// - `state`: 应用状态，通过 Axum 的 `State` 提取器注入，包含代理运行时的共享状态
/// - `peer_addr`: 客户端套接字地址，通过 `ConnectInfo` 提取器获取，用于日志记录和安全审计
/// - `headers`: HTTP 请求头映射，可能包含认证信息、内容类型、自定义元数据等
/// - `body`: 请求体，解析为 `WebhookBody` 结构体；如果解析失败则包含错误信息
///
/// # 返回值
///
/// 返回 `Response` 对象，根据处理结果可能包含：
/// - 成功响应（200 OK）：Webhook 事件已成功接收并处理
/// - 客户端错误（4xx）：请求格式错误、验证失败等
/// - 服务器错误（5xx）：内部处理错误
///
/// # 示例
///
/// ```text
/// curl -X POST http://localhost:8080/webhook \
///   -H "Content-Type: application/json" \
///   -d '{"event_type": "notification", "data": {"message": "Hello"}}'
/// ```
///
/// # 注意事项
///
/// - 该函数仅负责请求接收和参数提取，实际业务逻辑在 `webhook_ingress` 模块中实现
/// - 所有错误处理和响应生成均由内部逻辑完成，确保一致的错误响应格式
pub async fn handle_webhook<S, R: Clone, W: Clone, H>(
    State(state): State<WebhookState<S, R, W, H>>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Json<WebhookBody>, axum::extract::rejection::JsonRejection>,
    chat: WebhookChat<S, R, W, H>,
) -> Response {
    // 委托给内部处理逻辑，传入所有提取的请求信息
    // 内部逻辑负责验证、处理和生成响应
    super::webhook_ingress::handle_webhook_inner(state, peer_addr, headers, body, chat)
        .await
        // 将内部处理结果转换为 HTTP 响应
        .into_response()
}

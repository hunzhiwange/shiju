//! Gateway 模块的类型定义
//!
//! 本模块定义了网关（Gateway）子系统所需的核心数据类型。
//! 这些类型主要用于 HTTP API 请求的反序列化，支持代理系统的外部接口。
//!
//! # 主要功能
//!
//! - 定义 Webhook 请求体结构，用于接收外部系统发送的消息
//! - 定义 Agent 请求体结构，用于接收对代理的直接调用
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use gateway_types::{WebhookBody, AgentBody};
//!
//! // 反序列化 Webhook 请求
//! let webhook: WebhookBody = serde_json::from_str(r#"{"message": "Hello"}"#)?;
//!
//! // 反序列化 Agent 请求
//! let agent: AgentBody = serde_json::from_str(r#"{"message": "Process this"}"#)?;
//! ```

/// Webhook 请求体结构
///
/// 该结构体用于接收和处理来自外部系统的 Webhook 请求。
/// 它是 HTTP POST 请求的反序列化目标类型。
///
/// # 字段说明
///
/// - `message`: 外部系统发送的消息内容，UTF-8 编码的字符串
///
/// # 反序列化要求
///
/// 该结构体通过 `#[derive(serde::Deserialize)]` 自动实现反序列化，
/// 要求输入 JSON 必须包含 `message` 字段且值为字符串类型。
///
/// # 示例
///
/// ```rust,ignore
/// // 从 JSON 字符串解析
/// let json = r#"{"message": "Trigger event from external system"}"#;
/// let body: WebhookBody = serde_json::from_str(json)?;
/// assert_eq!(body.message, "Trigger event from external system");
///
/// // 从 HTTP 请求体解析（在 Axum 处理器中）
/// async fn handle_webhook(
///     Json(body): Json<WebhookBody>
/// ) -> impl IntoResponse {
///     println!("Received webhook message: {}", body.message);
///     StatusCode::OK
/// }
/// ```
#[derive(serde::Deserialize)]
pub struct WebhookBody {
    /// 外部系统发送的消息内容
    ///
    /// 该字段包含 Webhook 触发时传递的完整消息文本。
    /// 消息内容的具体格式和含义由发送方定义。
    pub message: String,
}

/// Agent 请求体结构
///
/// 该结构体用于接收和处理对代理系统的直接调用请求。
/// 它是 HTTP API 端点的反序列化目标类型。
///
/// # 字段说明
///
/// - `message`: 请求代理处理的指令或消息内容，UTF-8 编码的字符串
///
/// # 反序列化要求
///
/// 该结构体通过 `#[derive(serde::Deserialize)]` 自动实现反序列化，
/// 要求输入 JSON 必须包含 `message` 字段且值为字符串类型。
///
/// # 与 WebhookBody 的区别
///
/// 虽然 `AgentBody` 和 `WebhookBody` 的结构相同，但它们在语义上有区别：
/// - `WebhookBody`: 用于接收外部系统的推送通知和事件触发
/// - `AgentBody`: 用于接收客户端对代理的主动调用请求
///
/// 这种分离允许在未来根据需要为两种请求类型添加不同的字段，
/// 同时保持接口的清晰性和可扩展性。
///
/// # 示例
///
/// ```rust,ignore
/// // 从 JSON 字符串解析
/// let json = r#"{"message": "Please help me analyze this data"}"#;
/// let body: AgentBody = serde_json::from_str(json)?;
/// assert_eq!(body.message, "Please help me analyze this data");
///
/// // 在 HTTP 处理器中使用
/// async fn handle_agent_request(
///     Json(body): Json<AgentBody>
/// ) -> impl IntoResponse {
///     // 将消息传递给代理系统处理
///     let response = agent.process(&body.message).await;
///     Json(response)
/// }
/// ```
#[derive(serde::Deserialize)]
pub struct AgentBody {
    /// 请求代理处理的指令或消息内容
    ///
    /// 该字段包含客户端希望代理处理的完整指令文本。
    /// 代理系统将根据此消息内容执行相应的操作或生成响应。
    pub message: String,
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;

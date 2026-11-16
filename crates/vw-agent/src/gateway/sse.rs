//! 服务端推送事件（Server-Sent Events, SSE）流模块，用于实时事件推送。
//!
//! 本模块提供 SSE 实时事件流功能，将 AppState 中的广播通道（broadcast channel）
//! 包装为 HTTP SSE 响应，供 Web 仪表板客户端订阅和接收实时事件。
//!
//! # 主要功能
//!
//! - **SSE 事件流端点**：提供 `GET /api/events` 端点，客户端可通过该端点建立长连接，
//!   实时接收代理运行时产生的各类事件
//! - **认证保护**：支持可选 skey Bearer 鉴权，未认证的客户端将被拒绝访问
//! - **事件广播观察器**：`BroadcastObserver` 实现了 `Observer` trait，将观测事件
//!   转发到广播通道，供所有已连接的 SSE 客户端消费
//!
//! # 架构位置
//!
//! 该模块位于网关层（gateway），是面向 Web 客户端的主要实时通信接口之一。
//! 它依赖 `observability` 模块的 `Observer` trait 进行事件收集和转发。
//!
//! # 使用示例
//!
//! ```text
//! 客户端请求：
//!   GET /api/events
//!   Authorization: Bearer <skey>
//!
//! 服务器响应：
//!   Content-Type: text/event-stream
//!
//!   data: {"type":"agent_start","provider":"openai","model":"gpt-4","timestamp":"..."}
//!   data: {"type":"tool_call","tool":"shell","duration_ms":150,"success":true,"timestamp":"..."}
//! ```

pub use agent_gateway::sse::{BroadcastObserver, handle_sse_events};

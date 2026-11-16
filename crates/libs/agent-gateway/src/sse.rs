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

use gateway_types::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
};
use std::convert::Infallible;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

/// 处理 SSE 事件流的 HTTP 端点处理器。
///
/// 该函数处理 `GET /api/events` 请求，建立服务器推送事件（SSE）连接，
/// 将代理运行时产生的各类事件实时推送给客户端。
///
/// # 认证机制
///
/// 如果系统启用了 skey 鉴权，客户端必须在请求头中提供有效的
/// `Authorization: Bearer <skey>`。未认证的请求将返回 401 未授权错误。
///
/// # 参数
///
/// - `State(state)`: Axum 状态提取器，包含应用程序共享状态 `AppState`
///   - `state.pairing`: skey 鉴权管理器，用于验证客户端身份
///   - `state.event_tx`: 广播通道发送端，用于订阅事件流
/// - `headers`: HTTP 请求头，用于提取 skey
///
/// # 返回值
///
/// - 成功：返回 SSE 流响应，客户端可持续接收事件
/// - 未授权：返回 `(StatusCode::UNAUTHORIZED, 错误消息)`
///
/// # 事件流特性
///
/// - 使用 `KeepAlive` 保持连接活跃，防止被中间代理断开
/// - 自动跳过因客户端处理过慢导致的滞后消息（lagged messages）
/// - 所有事件以 JSON 字符串格式通过 `data:` 字段发送
///
/// # 示例
///
/// ```text
/// // 客户端请求
/// GET /api/events HTTP/1.1
/// Host: localhost:3000
/// Authorization: Bearer abc123skey
///
/// // 服务器响应（SSE 格式）
/// HTTP/1.1 200 OK
/// Content-Type: text/event-stream
///
/// data: {"type":"llm_request","provider":"openai","model":"gpt-4","timestamp":"2024-01-15T10:30:00Z"}
///
/// data: {"type":"tool_call_start","tool":"shell","timestamp":"2024-01-15T10:30:01Z"}
/// ```
pub async fn handle_sse_events<S, C, H>(
    State(state): State<AppState<S, C, H>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if state.pairing.auth_enabled()
        && gateway_types::auth::extract_auth_skey(&headers)
            .is_none_or(|skey| !state.pairing.is_authenticated(skey))
    {
        return (StatusCode::UNAUTHORIZED, "Unauthorized — provide Authorization: Bearer <skey>")
            .into_response();
    }

    // 订阅广播通道，获取事件接收端
    let rx = state.event_tx.subscribe();

    // 构建事件流：将广播接收端转换为 SSE 流
    // 使用 filter_map 过滤错误，跳过因客户端处理过慢导致的滞后消息
    let stream = BroadcastStream::new(rx).filter_map(
        |result: Result<
            serde_json::Value,
            tokio_stream::wrappers::errors::BroadcastStreamRecvError,
        >| {
            match result {
                // 成功接收事件，包装为 SSE Event
                Ok(value) => Some(Ok::<_, Infallible>(Event::default().data(value.to_string()))),
                // 跳过滞后消息（客户端处理过慢导致的消息丢失）
                Err(_) => None,
            }
        },
    );

    // 返回带有心跳保活的 SSE 响应
    Sse::new(stream).keep_alive(KeepAlive::default()).into_response()
}

/// 广播观察器，将观测事件转发到 SSE 广播通道。
///
/// 该结构体实现了 `Observer` trait，作为观测系统的一个中间层，
/// 在记录事件的同时将事件广播给所有已连接的 SSE 客户端。
///
/// # 设计模式
///
/// 采用装饰器模式（Decorator Pattern）：
/// - `inner`: 内层观察器，可能是日志观察器、指标收集器等
/// - `tx`: 广播通道发送端，用于将事件推送到 SSE 流
///
/// # 事件类型
///
/// 仅转发特定类型的事件到 SSE 通道：
/// - `LlmRequest`: LLM 请求事件
/// - `ToolCall`: 工具调用完成事件
/// - `ToolCallStart`: 工具调用开始事件
/// - `Error`: 错误事件
/// - `AgentStart`: 代理启动事件
/// - `AgentEnd`: 代理结束事件
///
/// 其他事件类型将被忽略，不进行广播。
///
/// # 线程安全
///
/// 该结构体可以安全地在多线程环境中使用，内部的广播通道是线程安全的。
pub struct BroadcastObserver {
    /// 内层观察器，用于实际的日志记录或指标收集
    inner: Box<dyn observer_types::Observer>,
    /// 广播通道发送端，用于将事件推送到所有 SSE 客户端
    tx: tokio::sync::broadcast::Sender<serde_json::Value>,
}

impl BroadcastObserver {
    /// 创建新的广播观察器实例。
    ///
    /// # 参数
    ///
    /// - `inner`: 内层观察器，将接收所有事件的副本进行实际记录
    /// - `tx`: 广播通道发送端，用于将事件推送到 SSE 客户端
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let (tx, _) = tokio::sync::broadcast::channel(100);
    /// let inner = Box::new(LoggingObserver::new());
    /// let observer = BroadcastObserver::new(inner, tx);
    /// ```
    pub fn new(
        inner: Box<dyn observer_types::Observer>,
        tx: tokio::sync::broadcast::Sender<serde_json::Value>,
    ) -> Self {
        Self { inner, tx }
    }
}

impl observer_types::Observer for BroadcastObserver {
    /// 记录并广播观测事件。
    ///
    /// 该方法执行两个操作：
    /// 1. 将事件转发给内层观察器进行实际记录（如日志、指标等）
    /// 2. 将事件转换为 JSON 格式并广播到 SSE 通道
    ///
    /// # 广播的事件格式
    ///
    /// 所有广播的事件都包含 `type` 和 `timestamp` 字段：
    ///
    /// - **llm_request**: 包含 `provider`、`model`
    /// - **tool_call**: 包含 `tool`、`duration_ms`、`success`
    /// - **tool_call_start**: 包含 `tool`
    /// - **error**: 包含 `component`、`message`
    /// - **agent_start**: 包含 `provider`、`model`
    /// - **agent_end**: 包含 `provider`、`model`、`duration_ms`、`tokens_used`、`cost_usd`
    ///
    /// # 参数
    ///
    /// - `event`: 观测事件，包含代理运行时的各类状态变更信息
    fn record_event(&self, event: &observer_types::ObserverEvent) {
        // 先将事件转发给内层观察器进行实际记录
        self.inner.record_event(event);

        // 根据事件类型构建要广播的 JSON 对象
        let json = match event {
            // LLM 请求事件：记录提供商、模型和当前时间戳
            observer_types::ObserverEvent::LlmRequest {
                provider, model, ..
            } => serde_json::json!({
                "type": "llm_request",
                "provider": provider,
                "model": model,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),

            // 工具调用完成事件：记录工具名称、执行时长和成功状态
            observer_types::ObserverEvent::ToolCall {
                tool,
                duration,
                success,
            } => serde_json::json!({
                "type": "tool_call",
                "tool": tool,
                "duration_ms": duration.as_millis(),
                "success": success,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),

            // 工具调用开始事件：记录工具名称和开始时间
            observer_types::ObserverEvent::ToolCallStart { tool } => {
                serde_json::json!({
                    "type": "tool_call_start",
                    "tool": tool,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })
            }

            // 错误事件：记录发生错误的组件和错误消息
            observer_types::ObserverEvent::Error { component, message } => {
                serde_json::json!({
                    "type": "error",
                    "component": component,
                    "message": message,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })
            }

            // 代理启动事件：记录使用的提供商和模型
            observer_types::ObserverEvent::AgentStart { provider, model } => {
                serde_json::json!({
                    "type": "agent_start",
                    "provider": provider,
                    "model": model,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })
            }

            // 代理结束事件：记录完整的执行统计信息
            observer_types::ObserverEvent::AgentEnd {
                provider,
                model,
                duration,
                tokens_used,
                cost_usd,
            } => serde_json::json!({
                "type": "agent_end",
                "provider": provider,
                "model": model,
                "duration_ms": duration.as_millis(),
                "tokens_used": tokens_used,
                "cost_usd": cost_usd,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),

            // 不在广播范围内的事件类型，直接返回不进行广播
            _ => return,
        };

        // 将事件广播到 SSE 通道，忽略可能的发送错误（如没有订阅者）
        let _ = self.tx.send(json);
    }

    /// 记录观测指标。
    ///
    /// 直接将指标转发给内层观察器处理，不进行广播。
    /// 这是因为指标数据通常不适合通过 SSE 实时推送。
    ///
    /// # 参数
    ///
    /// - `metric`: 观测指标，如性能计数器、直方图等
    fn record_metric(&self, metric: &observer_types::ObserverMetric) {
        self.inner.record_metric(metric);
    }

    /// 刷新观察器缓冲区。
    ///
    /// 调用内层观察器的 flush 方法，确保所有缓冲的数据都已持久化。
    /// 这通常在程序关闭或需要强制同步时调用。
    fn flush(&self) {
        self.inner.flush();
    }

    /// 返回观察器名称。
    ///
    /// 标识此观察器为 "broadcast" 类型，用于日志和调试。
    ///
    /// # 返回值
    ///
    /// 固定返回 `"broadcast"` 字符串
    fn name(&self) -> &str {
        "gateway_broadcast"
    }

    /// 返回 `Any` trait 对象引用。
    ///
    /// 允许运行时类型检查和向下转型，用于需要访问具体类型的场景。
    ///
    /// # 返回值
    ///
    /// 返回 `self` 的 `&dyn Any` 引用
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
#[path = "sse_tests.rs"]
mod sse_tests;

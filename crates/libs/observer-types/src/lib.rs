//! 观测系统的核心事件、指标和后端 trait 定义。
//!
//! 该模块只描述观测契约，不绑定具体传输、存储或导出实现。具体后端通过实现 `Observer`
//! 接入，保持业务路径与 Prometheus、日志、OTEL 等集成解耦。

use std::time::Duration;

/// 运行时可记录的离散观测事件。
///
/// 事件用于描述“发生了什么”，适合被后端映射为日志、计数器或追踪记录。字段应保持低敏感度，
/// 避免携带原始 prompt、令牌或上游响应正文。
#[derive(Debug, Clone)]
pub enum ObserverEvent {
    /// 代理调用开始。
    AgentStart {
        /// 模型提供商标识。
        provider: String,
        /// 模型名。
        model: String,
    },
    /// LLM 请求开始。
    LlmRequest {
        /// 模型提供商标识。
        provider: String,
        /// 模型名。
        model: String,
        /// 本次请求包含的消息数量。
        messages_count: usize,
    },
    /// LLM 请求结束。
    LlmResponse {
        /// 模型提供商标识。
        provider: String,
        /// 模型名。
        model: String,
        /// 请求耗时。
        duration: Duration,
        /// 请求是否成功。
        success: bool,
        /// 简短错误信息。
        error_message: Option<String>,
        /// 输入 token 数。
        input_tokens: Option<u64>,
        /// 输出 token 数。
        output_tokens: Option<u64>,
        /// 缓存命中 token 数。
        cached_tokens: Option<u64>,
        /// 推理 token 数。
        reasoning_tokens: Option<u64>,
    },
    /// 代理调用结束。
    AgentEnd {
        /// 模型提供商标识。
        provider: String,
        /// 模型名。
        model: String,
        /// 调用总耗时。
        duration: Duration,
        /// 本轮使用的 token 数。
        tokens_used: Option<u64>,
        /// 估算成本。
        cost_usd: Option<f64>,
    },
    /// 工具调用开始。
    ToolCallStart {
        /// 工具名。
        tool: String,
    },
    /// 工具调用结束。
    ToolCall {
        /// 工具名。
        tool: String,
        /// 调用耗时。
        duration: Duration,
        /// 调用是否成功。
        success: bool,
    },
    /// 单个 turn 已完成。
    TurnComplete,
    /// 通道消息收发事件。
    ChannelMessage {
        /// 通道名。
        channel: String,
        /// 方向标识。
        direction: String,
    },
    /// 心跳调度 tick。
    HeartbeatTick,
    /// 组件级错误事件。
    Error {
        /// 组件名。
        component: String,
        /// 简短错误信息。
        message: String,
    },
}

/// 运行时可记录的数值型观测指标。
///
/// 指标用于描述当前值或延迟等可聚合数据，后端可以按自身能力映射为 gauge、histogram 或 counter。
#[derive(Debug, Clone)]
pub enum ObserverMetric {
    /// 请求延迟。
    RequestLatency(Duration),
    /// 最近一次请求使用的 token 数。
    TokensUsed(u64),
    /// 当前活跃 session 数。
    ActiveSessions(u64),
    /// 当前队列深度。
    QueueDepth(u64),
}

/// 观测后端的最小契约。
///
/// 实现者负责把事件和指标写入具体目标，例如日志、Prometheus registry 或 noop。trait 保持窄接口，
/// 避免把导出协议、存储策略和业务语义耦合到调用方。
pub trait Observer: Send + Sync + 'static {
    /// 记录一个离散事件。
    ///
    /// # 参数
    ///
    /// - `event`: 待记录的观测事件。
    ///
    /// # 错误处理
    ///
    /// 本方法不返回错误；实现应在内部降级处理失败，避免观测链路影响主流程。
    fn record_event(&self, event: &ObserverEvent);
    /// 记录一个数值指标。
    ///
    /// # 参数
    ///
    /// - `metric`: 待记录的指标。
    ///
    /// # 错误处理
    ///
    /// 本方法不返回错误；实现应把导出失败限制在后端内部。
    fn record_metric(&self, metric: &ObserverMetric);
    /// 刷新后端缓冲区。
    ///
    /// # 错误处理
    ///
    /// 默认实现为空操作；需要刷新的后端应自行处理失败并保持调用方无感。
    fn flush(&self) {}
    /// 返回后端稳定名称。
    ///
    /// # 返回值
    ///
    /// 返回小写、面向用户的后端名称。
    fn name(&self) -> &str;
    /// 返回 `Any` 引用以支持测试或少量受控的向下转型。
    ///
    /// # 返回值
    ///
    /// 返回当前实现的动态类型引用。
    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(test)]
mod tests;

//! 日志观察器模块
//!
//! 本模块提供了基于 `tracing` 框架的日志观察器实现，用于记录代理运行时的各类事件和指标。
//! 作为 [`Observer`] trait 的一个具体实现，`LogObserver` 将所有观测数据输出为结构化日志。
//!
//! # 主要功能
//!
//! - **事件记录**：捕获代理生命周期事件（启动、结束、工具调用等）并输出为 INFO 级别日志
//! - **指标记录**：将性能指标（延迟、令牌使用量、会话数等）以结构化方式记录
//! - **零开销设计**：日志观察器本身不持有状态，实例化成本极低
//!
//! # 日志格式
//!
//! 所有日志均使用 `tracing` 宏输出，字段以结构化键值对形式附加，便于日志聚合系统解析和查询。
//!
//! # 示例
//!
//! ```ignore
//! use observer_backends::LogObserver;
//! use observer_types::Observer;
//!
//! let observer = LogObserver::new();
//! observer.name(); // 返回 "log"
//! ```

use observer_types::{Observer, ObserverEvent, ObserverMetric};
use std::any::Any;
use tracing::info;

/// 基于日志的观察器实现
///
/// `LogObserver` 是 [`Observer`] trait 的一个轻量级实现，将所有观测事件和指标
/// 通过 `tracing` 框架输出为结构化日志。该观察器不持有任何内部状态，适用于
/// 开发调试、本地运行或需要将观测数据与现有日志基础设施集成的场景。
///
/// # 特性
///
/// - **无状态**：观察器实例不存储任何数据，所有事件立即输出
/// - **结构化输出**：使用 `tracing` 的结构化日志格式，支持字段查询
/// - **INFO 级别**：所有日志统一使用 INFO 级别输出
///
/// # 适用场景
///
/// - 开发和调试环境
/// - 与现有日志系统集成
/// - 不需要持久化存储观测数据的场景
pub struct LogObserver;

impl LogObserver {
    /// 创建新的日志观察器实例
    ///
    /// # 返回值
    ///
    /// 返回一个新创建的 `LogObserver` 实例。由于观察器不持有状态，
    /// 多次调用将返回功能等效的实例。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use observer_backends::LogObserver;
    ///
    /// let observer = LogObserver::new();
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl Observer for LogObserver {
    /// 记录代理运行时事件
    ///
    /// 根据事件类型输出相应的结构化日志。每种事件类型都有其特定的字段集，
    /// 便于后续日志查询和分析。
    ///
    /// # 参数
    ///
    /// - `event`：要记录的观察器事件，包含事件类型及相关元数据
    ///
    /// # 事件类型与输出格式
    ///
    /// | 事件类型 | 日志消息 | 关键字段 |
    /// |---------|---------|---------|
    /// | `AgentStart` | `agent.start` | `provider`, `model` |
    /// | `AgentEnd` | `agent.end` | `provider`, `model`, `duration_ms`, `tokens`, `cost_usd` |
    /// | `ToolCallStart` | `tool.start` | `tool` |
    /// | `ToolCall` | `tool.call` | `tool`, `duration_ms`, `success` |
    /// | `TurnComplete` | `turn.complete` | (无) |
    /// | `ChannelMessage` | `channel.message` | `channel`, `direction` |
    /// | `HeartbeatTick` | `heartbeat.tick` | (无) |
    /// | `Error` | `error` | `component`, `error` |
    /// | `LlmRequest` | `llm.request` | `provider`, `model`, `messages_count` |
    /// | `LlmResponse` | `llm.response` | `provider`, `model`, `duration_ms`, `success`, `error`, `input_tokens`, `output_tokens` |
    fn record_event(&self, event: &ObserverEvent) {
        match event {
            // 代理启动事件：记录使用的 provider 和模型名称
            ObserverEvent::AgentStart { provider, model } => {
                info!(provider = %provider, model = %model, "agent.start");
            }
            // 代理结束事件：记录完整的执行摘要，包括耗时、令牌使用和成本
            ObserverEvent::AgentEnd { provider, model, duration, tokens_used, cost_usd } => {
                // 将 Duration 转换为毫秒，溢出时使用 u64::MAX 作为安全回退
                let ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
                info!(provider = %provider, model = %model, duration_ms = ms, tokens = ?tokens_used, cost_usd = ?cost_usd, "agent.end");
            }
            // 工具调用开始事件：仅记录工具名称
            ObserverEvent::ToolCallStart { tool } => {
                info!(tool = %tool, "tool.start");
            }
            // 工具调用完成事件：记录工具名称、执行耗时和成功状态
            ObserverEvent::ToolCall { tool, duration, success } => {
                let ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
                info!(tool = %tool, duration_ms = ms, success = success, "tool.call");
            }
            // 轮次完成事件：标记代理完成了一个对话轮次
            ObserverEvent::TurnComplete => {
                info!("turn.complete");
            }
            // 通道消息事件：记录消息来源通道和方向（入站/出站）
            ObserverEvent::ChannelMessage { channel, direction } => {
                info!(channel = %channel, direction = %direction, "channel.message");
            }
            // 心跳事件：用于存活检测和健康监控
            ObserverEvent::HeartbeatTick => {
                info!("heartbeat.tick");
            }
            // 错误事件：记录发生错误的组件和错误消息
            ObserverEvent::Error { component, message } => {
                info!(component = %component, error = %message, "error");
            }
            // LLM 请求事件：记录发往 LLM 的请求元数据
            ObserverEvent::LlmRequest { provider, model, messages_count } => {
                info!(
                    provider = %provider,
                    model = %model,
                    messages_count = messages_count,
                    "llm.request"
                );
            }
            // LLM 响应事件：记录 LLM 响应的完整信息，包括成功状态和令牌统计
            ObserverEvent::LlmResponse {
                provider,
                model,
                duration,
                success,
                error_message,
                input_tokens,
                output_tokens,
                cached_tokens,
                reasoning_tokens,
            } => {
                let ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
                info!(
                    provider = %provider,
                    model = %model,
                    duration_ms = ms,
                    success = success,
                    error = ?error_message,
                    input_tokens = ?input_tokens,
                    output_tokens = ?output_tokens,
                    cached_tokens = ?cached_tokens,
                    reasoning_tokens = ?reasoning_tokens,
                    "llm.response"
                );
            }
        }
    }

    /// 记录性能指标
    ///
    /// 将指标数据以结构化日志格式输出。每类指标都有其特定的日志消息标识，
    /// 便于在日志系统中进行聚合和查询。
    ///
    /// # 参数
    ///
    /// - `metric`：要记录的指标，包含指标类型和具体数值
    ///
    /// # 指标类型与输出格式
    ///
    /// | 指标类型 | 日志消息 | 字段名 |
    /// |---------|---------|--------|
    /// | `RequestLatency` | `metric.request_latency` | `latency_ms` |
    /// | `TokensUsed` | `metric.tokens_used` | `tokens` |
    /// | `ActiveSessions` | `metric.active_sessions` | `sessions` |
    /// | `QueueDepth` | `metric.queue_depth` | `depth` |
    fn record_metric(&self, metric: &ObserverMetric) {
        match metric {
            // 请求延迟指标：记录请求处理耗时（毫秒）
            ObserverMetric::RequestLatency(d) => {
                let ms = u64::try_from(d.as_millis()).unwrap_or(u64::MAX);
                info!(latency_ms = ms, "metric.request_latency");
            }
            // 令牌使用量指标：记录消耗的令牌总数
            ObserverMetric::TokensUsed(t) => {
                info!(tokens = t, "metric.tokens_used");
            }
            // 活跃会话数指标：记录当前正在进行的会话数量
            ObserverMetric::ActiveSessions(s) => {
                info!(sessions = s, "metric.active_sessions");
            }
            // 队列深度指标：记录等待处理的任务队列长度
            ObserverMetric::QueueDepth(d) => {
                info!(depth = d, "metric.queue_depth");
            }
        }
    }

    /// 返回观察器名称
    ///
    /// # 返回值
    ///
    /// 返回固定字符串 `"log"`，用于标识此观察器的类型。
    ///
    /// # 用途
    ///
    /// 该名称用于在多个观察器中进行区分，以及在配置和日志中标识观察器类型。
    fn name(&self) -> &str {
        "log"
    }

    /// 将观察器转换为 `Any` 类型引用
    ///
    /// # 返回值
    ///
    /// 返回 `self` 的 `dyn Any` 引用，支持运行时类型检查和向下转型。
    ///
    /// # 用途
    ///
    /// 该方法允许调用者在需要时获取观察器的具体类型信息，
    /// 通常用于需要根据观察器类型执行特定逻辑的高级场景。
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests;

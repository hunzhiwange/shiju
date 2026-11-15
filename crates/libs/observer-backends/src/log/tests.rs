//! 日志观察者模块的单元测试
//!
//! 本模块提供了对 `LogObserver` 实现的全面测试覆盖，确保所有事件和指标记录
//! 功能正常工作且不会引发 panic。
//!
//! # 测试范围
//!
//! - **基本功能测试**：验证观察者名称配置正确
//! - **事件记录测试**：测试所有类型的 `ObserverEvent` 变体
//! - **指标记录测试**：测试所有类型的 `ObserverMetric` 变体
//!
//! # 测试策略
//!
//! 采用"不崩溃"（no panic）测试策略，即验证各种输入情况下观察者能够稳定运行，
//! 即使面对边界值和错误场景也不会崩溃。

use super::*;
use std::time::Duration;

/// 测试 LogObserver 的名称标识符
///
/// 验证新创建的 `LogObserver` 实例返回正确的观察者名称 "log"。
/// 这个名称用于在多观察者系统中识别和路由日志事件。
#[test]
fn log_observer_name() {
    assert_eq!(LogObserver::new().name(), "log");
}

/// 测试 LogObserver 记录所有事件类型不会引发 panic
///
/// 本测试验证 `LogObserver` 能够安全处理所有 `ObserverEvent` 变体：
///
/// # 测试场景
///
/// 1. **Agent 生命周期事件**
///    - `AgentStart`：代理启动，包含 provider 和 model 信息
///    - `AgentEnd`：代理结束，包含完整的执行统计（duration、tokens、cost）
///
/// 2. **LLM 响应事件**
///    - 成功场景：包含 token 使用统计
///    - 失败场景：包含错误信息（如 rate limiting）
///
/// 3. **工具调用事件**
///    - `ToolCall`：记录工具执行结果（成功/失败）
///
/// 4. **通道消息事件**
///    - `ChannelMessage`：记录消息方向（入站/出站）
///
/// 5. **系统事件**
///    - `HeartbeatTick`：健康检查心跳
///    - `Error`：组件错误报告
///
/// # 边界测试
///
/// - Duration::ZERO（零持续时间）
/// - None 值（可选字段的缺失）
/// - Some 值（正常数据）
#[test]
fn log_observer_all_events_no_panic() {
    let obs = LogObserver::new();

    // 测试代理启动事件
    obs.record_event(&ObserverEvent::AgentStart {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
    });

    // 测试代理结束事件 - 完整数据场景
    obs.record_event(&ObserverEvent::AgentEnd {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        duration: Duration::from_millis(500),
        tokens_used: Some(100),
        cost_usd: Some(0.0015),
    });

    // 测试代理结束事件 - 边界值场景（零持续时间、无统计）
    obs.record_event(&ObserverEvent::AgentEnd {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        duration: Duration::ZERO,
        tokens_used: None,
        cost_usd: None,
    });

    // 测试 LLM 响应事件 - 成功场景
    obs.record_event(&ObserverEvent::LlmResponse {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        duration: Duration::from_millis(150),
        success: true,
        error_message: None,
        input_tokens: Some(100),
        output_tokens: Some(50),
        cached_tokens: Some(20),
        reasoning_tokens: Some(10),
    });

    // 测试 LLM 响应事件 - 失败场景（如限流）
    obs.record_event(&ObserverEvent::LlmResponse {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        duration: Duration::from_millis(200),
        success: false,
        error_message: Some("rate limited".into()),
        input_tokens: None,
        output_tokens: None,
        cached_tokens: None,
        reasoning_tokens: None,
    });

    // 测试工具调用事件 - 失败场景
    obs.record_event(&ObserverEvent::ToolCall {
        tool: "shell".into(),
        duration: Duration::from_millis(10),
        success: false,
    });

    // 测试通道消息事件
    obs.record_event(&ObserverEvent::ChannelMessage {
        channel: "telegram".into(),
        direction: "outbound".into(),
    });

    // 测试心跳事件
    obs.record_event(&ObserverEvent::HeartbeatTick);

    // 测试错误事件
    obs.record_event(&ObserverEvent::Error {
        component: "provider".into(),
        message: "timeout".into(),
    });
}

/// 测试 LogObserver 记录所有指标类型不会引发 panic
///
/// 本测试验证 `LogObserver` 能够安全处理所有 `ObserverMetric` 变体：
///
/// # 测试场景
///
/// 1. **请求延迟指标**
///    - `RequestLatency`：记录请求处理时间
///
/// 2. **Token 使用量指标**
///    - 零值：`TokensUsed(0)`
///    - 最大值：`TokensUsed(u64::MAX)` - 边界测试
///
/// 3. **会话状态指标**
///    - `ActiveSessions`：当前活跃会话数
///
/// 4. **队列深度指标**
///    - `QueueDepth`：等待处理的请求数
///
/// # 边界测试
///
/// - 0 值（最小有效值）
/// - u64::MAX（最大有效值，测试数值溢出处理）
/// - 大数值（999）
#[test]
fn log_observer_all_metrics_no_panic() {
    let obs = LogObserver::new();

    // 测试请求延迟指标
    obs.record_metric(&ObserverMetric::RequestLatency(Duration::from_secs(2)));

    // 测试 Token 使用量指标 - 零值
    obs.record_metric(&ObserverMetric::TokensUsed(0));

    // 测试 Token 使用量指标 - 最大值（边界测试）
    obs.record_metric(&ObserverMetric::TokensUsed(u64::MAX));

    // 测试活跃会话数指标
    obs.record_metric(&ObserverMetric::ActiveSessions(1));

    // 测试队列深度指标
    obs.record_metric(&ObserverMetric::QueueDepth(999));
}

#[test]
fn log_observer_records_placeholder_events_and_supports_downcast() {
    let obs = LogObserver::new();

    obs.record_event(&ObserverEvent::ToolCallStart { tool: "shell".into() });
    obs.record_event(&ObserverEvent::TurnComplete);
    obs.record_event(&ObserverEvent::LlmRequest {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        messages_count: 0,
    });

    assert!(obs.as_any().downcast_ref::<LogObserver>().is_some());
}

#[test]
fn log_observer_duration_overflow_falls_back_without_panicking() {
    let obs = LogObserver::new();

    obs.record_event(&ObserverEvent::AgentEnd {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        duration: Duration::MAX,
        tokens_used: None,
        cost_usd: None,
    });
    obs.record_event(&ObserverEvent::ToolCall {
        tool: "shell".into(),
        duration: Duration::MAX,
        success: true,
    });
    obs.record_event(&ObserverEvent::LlmResponse {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        duration: Duration::MAX,
        success: true,
        error_message: None,
        input_tokens: None,
        output_tokens: None,
        cached_tokens: None,
        reasoning_tokens: None,
    });
    obs.record_metric(&ObserverMetric::RequestLatency(Duration::MAX));
}

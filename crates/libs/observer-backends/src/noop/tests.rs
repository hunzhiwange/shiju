//! NoopObserver 单元测试模块
//!
//! 本模块包含针对 `NoopObserver` 实现的所有单元测试。
//! NoopObserver 是一个空操作的观察者实现，用于在不需要观测能力时提供零开销的占位符。
//!
//! # 测试覆盖范围
//!
//! - 观察者标识符验证
//! - 事件记录接口的各种场景
//! - 指标记录接口的各种场景
//! - 刷新操作的稳定性

use super::*;
use std::time::Duration;

/// 测试 NoopObserver 的标识符返回值
///
/// 验证 NoopObserver 实现的 `name()` 方法正确返回 "noop" 字符串。
/// 这确保观察者能够被正确识别为空操作实现。
#[test]
fn noop_name() {
    assert_eq!(NoopObserver.name(), "noop");
}

/// 测试 record_event 方法在所有事件类型下都不会 panic
///
/// 该测试覆盖 ObserverEvent 枚举的所有主要变体，确保：
/// - 方法调用本身不会引发 panic
/// - 各种事件字段组合都能被安全处理
/// - 包含可选字段的事件能够正确处理 Some 和 None 情况
///
/// # 测试场景
///
/// - `HeartbeatTick`: 心跳事件
/// - `AgentStart`: 代理启动事件（包含 provider 和 model 信息）
/// - `AgentEnd`: 代理结束事件（测试完整字段和部分字段为 None 的情况）
/// - `ToolCall`: 工具调用事件（包含执行时长和成功状态）
/// - `ChannelMessage`: 通道消息事件（包含通道名称和方向）
/// - `Error`: 错误事件（包含组件名称和错误消息）
#[test]
fn noop_record_event_does_not_panic() {
    let obs = NoopObserver;

    // 测试心跳事件
    obs.record_event(&ObserverEvent::HeartbeatTick);

    // 测试代理启动事件
    obs.record_event(&ObserverEvent::AgentStart { provider: "test".into(), model: "test".into() });

    // 测试代理结束事件（完整字段：包含 token 使用量和成本）
    obs.record_event(&ObserverEvent::AgentEnd {
        provider: "test".into(),
        model: "test".into(),
        duration: Duration::from_millis(100),
        tokens_used: Some(42),
        cost_usd: Some(0.001),
    });

    // 测试代理结束事件（部分字段：token 和成本均为 None）
    obs.record_event(&ObserverEvent::AgentEnd {
        provider: "test".into(),
        model: "test".into(),
        duration: Duration::ZERO,
        tokens_used: None,
        cost_usd: None,
    });

    // 测试工具调用事件
    obs.record_event(&ObserverEvent::ToolCall {
        tool: "shell".into(),
        duration: Duration::from_secs(1),
        success: true,
    });

    // 测试通道消息事件
    obs.record_event(&ObserverEvent::ChannelMessage {
        channel: "cli".into(),
        direction: "inbound".into(),
    });

    // 测试错误事件
    obs.record_event(&ObserverEvent::Error { component: "test".into(), message: "boom".into() });
}

/// 测试 record_metric 方法在所有指标类型下都不会 panic
///
/// 该测试覆盖 ObserverMetric 枚举的主要变体，确保：
/// - 方法调用本身不会引发 panic
/// - 不同类型的指标值都能被安全处理
///
/// # 测试场景
///
/// - `RequestLatency`: 请求延迟指标（Duration 类型）
/// - `TokensUsed`: Token 使用量指标（数值类型）
/// - `ActiveSessions`: 活跃会话数指标（数值类型）
/// - `QueueDepth`: 队列深度指标（数值类型）
#[test]
fn noop_record_metric_does_not_panic() {
    let obs = NoopObserver;

    // 测试请求延迟指标
    obs.record_metric(&ObserverMetric::RequestLatency(Duration::from_millis(50)));

    // 测试 Token 使用量指标
    obs.record_metric(&ObserverMetric::TokensUsed(1000));

    // 测试活跃会话数指标
    obs.record_metric(&ObserverMetric::ActiveSessions(5));

    // 测试队列深度指标（包括零值情况）
    obs.record_metric(&ObserverMetric::QueueDepth(0));
}

/// 测试 flush 方法不会 panic
///
/// 验证 NoopObserver 的 `flush()` 方法能够安全调用。
/// 对于空操作观察者，flush 应该是空操作，但调用本身不应引发任何错误。
#[test]
fn noop_flush_does_not_panic() {
    NoopObserver.flush();
}

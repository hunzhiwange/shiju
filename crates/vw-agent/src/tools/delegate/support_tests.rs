//! 验证委派工具的空观察器事件与指标接口契约。

use super::*;
use std::time::Duration;

#[test]
fn noop_observer_reports_stable_name() {
    assert_eq!(NoopObserver.name(), "noop");
}

#[test]
fn noop_observer_exposes_concrete_type_through_any() {
    assert!(NoopObserver.as_any().is::<NoopObserver>());
}

#[test]
fn noop_observer_accepts_all_event_variants() {
    let observer = NoopObserver;
    let events = [
        ObserverEvent::AgentStart { provider: "provider".into(), model: "model".into() },
        ObserverEvent::LlmRequest {
            provider: "provider".into(),
            model: "model".into(),
            messages_count: 0,
        },
        ObserverEvent::LlmResponse {
            provider: "provider".into(),
            model: "model".into(),
            duration: Duration::ZERO,
            success: false,
            error_message: Some("redacted error".into()),
            input_tokens: None,
            output_tokens: Some(0),
            cached_tokens: Some(u64::MAX),
            reasoning_tokens: None,
        },
        ObserverEvent::AgentEnd {
            provider: "provider".into(),
            model: "model".into(),
            duration: Duration::MAX,
            tokens_used: Some(u64::MAX),
            cost_usd: None,
        },
        ObserverEvent::ToolCallStart { tool: "tool".into() },
        ObserverEvent::ToolCall { tool: "tool".into(), duration: Duration::ZERO, success: true },
        ObserverEvent::TurnComplete,
        ObserverEvent::ChannelMessage { channel: "channel".into(), direction: "inbound".into() },
        ObserverEvent::HeartbeatTick,
        ObserverEvent::Error { component: "component".into(), message: "redacted error".into() },
    ];

    for event in &events {
        observer.record_event(event);
    }
}

#[test]
fn noop_observer_accepts_metric_boundaries() {
    let observer = NoopObserver;
    let metrics = [
        ObserverMetric::RequestLatency(Duration::MAX),
        ObserverMetric::TokensUsed(0),
        ObserverMetric::ActiveSessions(u64::MAX),
        ObserverMetric::QueueDepth(0),
    ];

    for metric in &metrics {
        observer.record_metric(metric);
    }
}

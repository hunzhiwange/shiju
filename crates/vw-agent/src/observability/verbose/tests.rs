//! verbose 观测器的基础回归测试。
//!
//! 这些测试只验证观测器的稳定外部行为：名称保持不变，并且各类事件
//! 都能被记录而不触发 panic。输出内容由观测器实现负责，这里避免把
//! 测试耦合到日志格式。

use super::*;
use std::time::Duration;

#[test]
fn verbose_name() {
    let observer = VerboseObserver::new();
    assert_eq!(observer.name(), "verbose");
    assert!(observer.as_any().is::<VerboseObserver>());
}

#[test]
fn verbose_events_do_not_panic() {
    let obs = VerboseObserver::new();
    // 覆盖 LLM、工具调用与回合结束事件，确保新增事件字段时仍保持宽容处理。
    obs.record_event(&ObserverEvent::LlmRequest {
        provider: "openrouter".into(),
        model: "claude".into(),
        messages_count: 3,
    });
    obs.record_event(&ObserverEvent::LlmResponse {
        provider: "openrouter".into(),
        model: "claude".into(),
        duration: Duration::from_millis(12),
        success: true,
        error_message: None,
        input_tokens: Some(50),
        output_tokens: Some(25),
        cached_tokens: Some(5),
        reasoning_tokens: Some(2),
    });
    obs.record_event(&ObserverEvent::ToolCallStart { tool: "shell".into() });
    obs.record_event(&ObserverEvent::ToolCall {
        tool: "shell".into(),
        duration: Duration::from_millis(2),
        success: true,
    });
    obs.record_event(&ObserverEvent::TurnComplete);
    obs.record_event(&ObserverEvent::HeartbeatTick);
}

#[test]
fn verbose_metrics_and_flush_do_not_panic() {
    let obs = VerboseObserver::new();
    obs.record_metric(&ObserverMetric::TokensUsed(123));
    obs.flush();
}

#[test]
fn verbose_llm_response_duration_overflow_is_capped() {
    let obs = VerboseObserver::new();

    obs.record_event(&ObserverEvent::LlmResponse {
        provider: "openrouter".into(),
        model: "claude".into(),
        duration: Duration::from_secs(u64::MAX),
        success: false,
        error_message: Some("timeout".into()),
        input_tokens: None,
        output_tokens: None,
        cached_tokens: None,
        reasoning_tokens: None,
    });
}

#[test]
fn verbose_tool_call_duration_overflow_is_capped() {
    let obs = VerboseObserver::new();

    obs.record_event(&ObserverEvent::ToolCall {
        tool: "shell".into(),
        duration: Duration::from_secs(u64::MAX),
        success: false,
    });
}

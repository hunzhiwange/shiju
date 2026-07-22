//! Prometheus 观察者单元测试模块
//!
//! 本模块包含 `PrometheusObserver` 的完整测试套件，验证其作为 VibeWindow
//! 可观测性系统的 Prometheus 指标导出器的正确性。
//!
//! # 测试范围
//!
//! - **基础功能测试**：验证观察者初始化、名称获取等基本功能
//! - **事件记录测试**：验证各类观察者事件（AgentStart、AgentEnd、ToolCall 等）的记录能力
//! - **指标记录测试**：验证请求延迟、令牌使用量、活跃会话数等指标的记录能力
//! - **格式导出测试**：验证 Prometheus 文本格式的正确生成
//! - **计数器测试**：验证计数器类型的正确递增和标签分组
//! - **仪表测试**：验证仪表类型反映最新值的行为
//!
//! # 测试策略
//!
//! 所有测试遵循"无 panic"原则：即使输入边界条件（如零值、None 值），
//! 观察者也应优雅处理而不崩溃。测试通过断言 Prometheus 输出字符串来
//! 验证指标的正确性。

use super::*;
use std::time::Duration;

/// 测试 PrometheusObserver 的名称返回值
///
/// 验证 `PrometheusObserver::name()` 方法返回正确的标识符字符串 "prometheus"，
/// 该标识符用于在观察者注册表中唯一标识此 Prometheus 导出器。
#[test]
fn prometheus_observer_name() {
    assert_eq!(PrometheusObserver::new().name(), "prometheus");
}

/// 测试记录所有类型的事件而不发生 panic
///
/// 此测试验证 `PrometheusObserver` 能够安全处理所有定义的 `ObserverEvent` 变体，
/// 包括边界条件（如零时长、None 值的可选字段）。测试覆盖以下事件类型：
///
/// - `AgentStart`：代理启动事件
/// - `AgentEnd`：代理结束事件（包含 token 使用量和可选成本）
/// - `ToolCall`：工具调用事件（成功和失败场景）
/// - `ChannelMessage`：通道消息事件（入站方向）
/// - `HeartbeatTick`：心跳计数事件
/// - `Error`：错误事件（包含组件和消息）
///
/// # 设计意图
///
/// 确保即使某些字段为零值或 None，观察者也不会 panic，
/// 这对于生产环境的稳定性至关重要。
#[test]
fn records_all_events_without_panic() {
    let obs = PrometheusObserver::new();

    // 记录代理启动事件
    obs.record_event(&ObserverEvent::AgentStart {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
    });

    // 记录代理结束事件（包含 token 使用信息）
    obs.record_event(&ObserverEvent::AgentEnd {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        duration: Duration::from_millis(500),
        tokens_used: Some(100),
        cost_usd: None, // 成本字段可选
    });

    // 记录代理结束事件（边界条件：零时长、None token）
    obs.record_event(&ObserverEvent::AgentEnd {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        duration: Duration::ZERO,
        tokens_used: None,
        cost_usd: None,
    });

    // 记录成功的工具调用
    obs.record_event(&ObserverEvent::ToolCall {
        tool: "shell".into(),
        duration: Duration::from_millis(10),
        success: true,
    });

    // 记录失败的工具调用
    obs.record_event(&ObserverEvent::ToolCall {
        tool: "file_read".into(),
        duration: Duration::from_millis(5),
        success: false,
    });

    // 记录通道入站消息
    obs.record_event(&ObserverEvent::ChannelMessage {
        channel: "telegram".into(),
        direction: "inbound".into(),
    });

    // 记录心跳计数
    obs.record_event(&ObserverEvent::HeartbeatTick);

    // 记录组件错误
    obs.record_event(&ObserverEvent::Error {
        component: "provider".into(),
        message: "timeout".into(),
    });
}

/// 测试记录所有类型的指标而不发生 panic
///
/// 此测试验证 `PrometheusObserver` 能够安全处理所有定义的 `ObserverMetric` 变体，
/// 包括零值场景。测试覆盖以下指标类型：
///
/// - `RequestLatency`：请求延迟时长
/// - `TokensUsed`：使用的令牌数量
/// - `ActiveSessions`：活跃会话数
/// - `QueueDepth`：队列深度
///
/// # 设计意图
///
/// 确保即使指标值为零，观察者也能正常记录，
/// 验证指标记录路径的健壮性。
#[test]
fn records_all_metrics_without_panic() {
    let obs = PrometheusObserver::new();

    // 记录请求延迟指标
    obs.record_metric(&ObserverMetric::RequestLatency(Duration::from_secs(2)));

    // 记录令牌使用量（正常值）
    obs.record_metric(&ObserverMetric::TokensUsed(500));

    // 记录令牌使用量（零值边界条件）
    obs.record_metric(&ObserverMetric::TokensUsed(0));

    // 记录活跃会话数
    obs.record_metric(&ObserverMetric::ActiveSessions(3));

    // 记录队列深度
    obs.record_metric(&ObserverMetric::QueueDepth(42));
}

/// 测试编码输出符合 Prometheus 文本格式
///
/// 此测试验证 `encode()` 方法能够生成符合 Prometheus 文本格式的输出，
/// 包含正确的指标名称和标签。测试场景：
///
/// 1. 记录代理启动事件
/// 2. 记录工具调用事件
/// 3. 记录心跳事件
/// 4. 记录请求延迟指标
///
/// # 验证项
///
/// 输出应包含以下指标前缀：
/// - `vibewindow_agent_starts_total`：代理启动计数器
/// - `vibewindow_tool_calls_total`：工具调用计数器
/// - `vibewindow_heartbeat_ticks_total`：心跳计数器
/// - `vibewindow_request_latency_seconds`：请求延迟直方图/摘要
#[test]
fn encode_produces_prometheus_text_format() {
    let obs = PrometheusObserver::new();

    // 记录多种类型的事件和指标
    obs.record_event(&ObserverEvent::AgentStart {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
    });
    obs.record_event(&ObserverEvent::ToolCall {
        tool: "shell".into(),
        duration: Duration::from_millis(100),
        success: true,
    });
    obs.record_event(&ObserverEvent::HeartbeatTick);
    obs.record_metric(&ObserverMetric::RequestLatency(Duration::from_millis(250)));

    // 编码并验证输出包含预期的指标名称
    let output = obs.encode();
    assert!(output.contains("vibewindow_agent_starts_total"));
    assert!(output.contains("vibewindow_tool_calls_total"));
    assert!(output.contains("vibewindow_heartbeat_ticks_total"));
    assert!(output.contains("vibewindow_request_latency_seconds"));
}

/// 测试计数器正确递增
///
/// 此测试验证计数器类型的指标能够正确累加多次记录的值。
/// 测试场景：连续记录 3 次心跳事件，验证计数器值为 3。
///
/// # 验证项
///
/// 输出应包含 `vibewindow_heartbeat_ticks_total 3`，
/// 证明计数器正确累加了 3 次事件记录。
#[test]
fn counters_increment_correctly() {
    let obs = PrometheusObserver::new();

    // 连续记录 3 次心跳事件
    for _ in 0..3 {
        obs.record_event(&ObserverEvent::HeartbeatTick);
    }

    // 验证计数器累加到 3
    let output = obs.encode();
    assert!(output.contains("vibewindow_heartbeat_ticks_total 3"));
}

/// 测试工具调用按成功/失败状态分别追踪
///
/// 此测试验证工具调用计数器能够通过 `success` 标签区分成功和失败的调用。
/// 测试场景：
///
/// 1. 记录 2 次成功的 shell 工具调用
/// 2. 记录 1 次失败的 shell 工具调用
///
/// # 验证项
///
/// 输出应包含两个独立的计数器：
/// - `vibewindow_tool_calls_total{success="true",tool="shell"} 2`
/// - `vibewindow_tool_calls_total{success="false",tool="shell"} 1`
///
/// 这验证了标签分组的正确性，允许在 Prometheus 中按成功率聚合分析。
#[test]
fn tool_calls_track_success_and_failure_separately() {
    let obs = PrometheusObserver::new();

    // 记录 2 次成功的 shell 工具调用
    obs.record_event(&ObserverEvent::ToolCall {
        tool: "shell".into(),
        duration: Duration::from_millis(10),
        success: true,
    });
    obs.record_event(&ObserverEvent::ToolCall {
        tool: "shell".into(),
        duration: Duration::from_millis(10),
        success: true,
    });

    // 记录 1 次失败的 shell 工具调用
    obs.record_event(&ObserverEvent::ToolCall {
        tool: "shell".into(),
        duration: Duration::from_millis(10),
        success: false,
    });

    // 验证成功和失败的调用被分别计数
    let output = obs.encode();
    assert!(output.contains(r#"vibewindow_tool_calls_total{success="true",tool="shell"} 2"#));
    assert!(output.contains(r#"vibewindow_tool_calls_total{success="false",tool="shell"} 1"#));
}

/// 测试错误按组件类型追踪
///
/// 此测试验证错误计数器能够通过 `component` 标签区分不同组件的错误。
/// 测试场景：
///
/// 1. 记录 2 次 provider 组件错误（不同消息）
/// 2. 记录 1 次 channels 组件错误
///
/// # 验证项
///
/// 输出应包含两个独立的计数器：
/// - `vibewindow_errors_total{component="provider"} 2`
/// - `vibewindow_errors_total{component="channels"} 1`
///
/// 注意：错误消息（message 字段）不作为标签，避免标签基数爆炸。
#[test]
fn errors_track_by_component() {
    let obs = PrometheusObserver::new();

    // 记录 provider 组件的 2 个不同错误
    obs.record_event(&ObserverEvent::Error {
        component: "provider".into(),
        message: "timeout".into(),
    });
    obs.record_event(&ObserverEvent::Error {
        component: "provider".into(),
        message: "rate limit".into(),
    });

    // 记录 channels 组件的错误
    obs.record_event(&ObserverEvent::Error {
        component: "channels".into(),
        message: "disconnected".into(),
    });

    // 验证错误按组件正确分组
    let output = obs.encode();
    assert!(output.contains(r#"vibewindow_errors_total{component="provider"} 2"#));
    assert!(output.contains(r#"vibewindow_errors_total{component="channels"} 1"#));
}

/// 测试仪表类型反映最新值
///
/// 此测试验证仪表（Gauge）类型的指标能够正确反映最近一次记录的值，
/// 而不是累加所有记录的值（与计数器不同）。
///
/// 测试场景：连续记录两次令牌使用量（100 和 200），验证最终值为 200。
///
/// # 验证项
///
/// 输出应包含 `vibewindow_tokens_used_last 200`，
/// 证明仪表只保留了最新值而非累加值。
#[test]
fn gauge_reflects_latest_value() {
    let obs = PrometheusObserver::new();

    // 先记录令牌使用量 100
    obs.record_metric(&ObserverMetric::TokensUsed(100));

    // 再记录令牌使用量 200（覆盖之前的值）
    obs.record_metric(&ObserverMetric::TokensUsed(200));

    // 验证仪表只保留了最新值 200
    let output = obs.encode();
    assert!(output.contains("vibewindow_tokens_used_last 200"));
}

/// 测试 LLM 响应追踪请求数和令牌统计
///
/// 此测试验证 LLM 响应事件能够同时追踪多个指标：
///
/// 1. **请求计数器**：按 provider、model 和 success 标签分组
/// 2. **输入令牌总数**：累加所有请求的输入令牌数
/// 3. **输出令牌总数**：累加所有请求的输出令牌数
///
/// 测试场景：记录 2 次成功的 LLM 响应，验证各项指标正确累加。
///
/// # 验证项
///
/// 输出应包含：
/// - 请求计数：`vibewindow_llm_requests_total{...success="true"} 2`
/// - 输入令牌总数：`vibewindow_tokens_input_total{...} 300` (100 + 200)
/// - 输出令牌总数：`vibewindow_tokens_output_total{...} 130` (50 + 80)
#[test]
fn llm_response_tracks_request_count_and_tokens() {
    let obs = PrometheusObserver::new();

    // 记录第一个 LLM 响应（100 输入 + 50 输出令牌）
    obs.record_event(&ObserverEvent::LlmResponse {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        duration: Duration::from_millis(200),
        success: true,
        error_message: None,
        input_tokens: Some(100),
        output_tokens: Some(50),
        cached_tokens: Some(30),
        reasoning_tokens: Some(15),
    });

    // 记录第二个 LLM 响应（200 输入 + 80 输出令牌）
    obs.record_event(&ObserverEvent::LlmResponse {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        duration: Duration::from_millis(300),
        success: true,
        error_message: None,
        input_tokens: Some(200),
        output_tokens: Some(80),
        cached_tokens: Some(40),
        reasoning_tokens: Some(20),
    });

    // 验证各项指标正确累加
    let output = obs.encode();
    assert!(output.contains(
        r#"vibewindow_llm_requests_total{model="claude-sonnet",provider="openrouter",success="true"} 2"#
    ));
    assert!(output.contains(
        r#"vibewindow_tokens_input_total{model="claude-sonnet",provider="openrouter"} 300"#
    ));
    assert!(output.contains(
        r#"vibewindow_tokens_output_total{model="claude-sonnet",provider="openrouter"} 130"#
    ));
}

/// 测试无令牌信息的 LLM 响应只增加请求计数
///
/// 此测试验证当 LLM 响应事件不包含令牌信息（token 字段为 None）时，
/// 观察者仍能正确处理：
///
/// 1. **请求计数器正常递增**：即使令牌信息缺失
/// 2. **令牌指标不生成**：避免输出 None 值的令牌统计
///
/// 测试场景：记录 1 次失败的 LLM 响应，输入/输出令牌均为 None。
///
/// # 验证项
///
/// 输出应包含：
/// - 请求计数：`vibewindow_llm_requests_total{...success="false"} 1`
/// - 不包含令牌输入指标：无 `vibewindow_tokens_input_total{`
/// - 不包含令牌输出指标：无 `vibewindow_tokens_output_total{`
///
/// # 设计意图
///
/// 某些 LLM 提供者可能不返回令牌使用信息，观察者应优雅处理此场景。
#[test]
fn llm_response_without_tokens_increments_request_only() {
    let obs = PrometheusObserver::new();

    // 记录一个不包含令牌信息的失败 LLM 响应
    obs.record_event(&ObserverEvent::LlmResponse {
        provider: "ollama".into(),
        model: "llama3".into(),
        duration: Duration::from_millis(100),
        success: false,
        error_message: Some("timeout".into()),
        input_tokens: None,
        output_tokens: None,
        cached_tokens: None,
        reasoning_tokens: None,
    });

    // 验证请求计数正常，但令牌指标不存在
    let output = obs.encode();
    assert!(output.contains(
        r#"vibewindow_llm_requests_total{model="llama3",provider="ollama",success="false"} 1"#
    ));
    assert!(!output.contains("vibewindow_tokens_input_total{"));
    assert!(!output.contains("vibewindow_tokens_output_total{"));
}

#[test]
fn prometheus_observer_supports_runtime_downcast() {
    let obs = PrometheusObserver::new();

    assert!(obs.as_any().downcast_ref::<PrometheusObserver>().is_some());
}

//! OpenTelemetry 观察器测试模块
//!
//! 本模块提供 `OtelObserver` 的单元测试套件，验证以下功能：
//! - 观察器创建和初始化
//! - 事件记录（代理生命周期、LLM 交互、工具调用等）
//! - 指标记录（延迟、令牌使用、会话状态等）
//! - 刷新操作的健壮性和幂等性
//! - 边界条件处理（零值、错误场景、不可达端点）
//!
//! 所有测试使用本地回环地址作为端点，不需要实际的 OpenTelemetry Collector 运行。
//! 测试重点在于验证代码路径不会 panic，而非验证遥测数据的实际传输。

use super::*;
use std::time::Duration;

/// 创建用于测试的 OtelObserver 实例
///
/// # 返回值
///
/// 返回一个配置好的 `OtelObserver` 实例，使用：
/// - 端点：`http://127.0.0.1:19999`（本地测试端点，无需真实服务）
/// - 服务名称：`vibewindow-test`
///
/// # Panic
///
/// 如果观察器创建失败，将 panic 并显示错误信息。
/// 这在测试中是预期行为，表示配置格式无效。
fn test_observer() -> OtelObserver {
    OtelObserver::new(Some("http://127.0.0.1:19999"), Some("vibewindow-test"))
        .expect("observer creation should not fail with valid endpoint format")
}

/// 测试 OtelObserver 的名称标识符
///
/// # 验证点
///
/// - 观察器的 `name()` 方法应返回 `"otel"`
/// - 用于在多观察器场景中识别 OpenTelemetry 后端
#[test]
fn otel_observer_name() {
    let obs = test_observer();
    assert_eq!(obs.name(), "otel");
}

/// 测试所有事件类型的记录功能不会导致 panic
///
/// # 验证场景
///
/// 本测试依次记录以下事件类型，验证每种类型都能正常处理：
/// - `AgentStart`：代理启动，包含 provider 和 model 信息
/// - `LlmRequest`：LLM 请求开始，记录消息数量
/// - `LlmResponse`：LLM 响应完成，包含令牌统计和执行时长
/// - `AgentEnd`：代理结束（成功和零值两种情况）
/// - `ToolCallStart`：工具调用开始
/// - `ToolCall`：工具调用完成（成功和失败两种情况）
/// - `TurnComplete`：对话轮次完成
/// - `ChannelMessage`：通道消息（入站方向）
/// - `HeartbeatTick`：心跳信号
/// - `Error`：错误事件，包含组件和消息
///
/// # 目的
///
/// 确保观察器能够处理所有定义的事件变体，不会因为事件类型或字段值而 panic。
#[test]
fn records_all_events_without_panic() {
    let obs = test_observer();
    // 记录代理启动事件
    obs.record_event(&ObserverEvent::AgentStart {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
    });
    // 记录 LLM 请求事件
    obs.record_event(&ObserverEvent::LlmRequest {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        messages_count: 2,
    });
    // 记录成功的 LLM 响应事件，包含令牌统计
    obs.record_event(&ObserverEvent::LlmResponse {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        duration: Duration::from_millis(250),
        success: true,
        error_message: None,
        input_tokens: Some(100),
        output_tokens: Some(50),
        cached_tokens: Some(25),
        reasoning_tokens: Some(12),
    });
    // 记录代理正常结束事件，包含完整的统计信息
    obs.record_event(&ObserverEvent::AgentEnd {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        duration: Duration::from_millis(500),
        tokens_used: Some(100),
        cost_usd: Some(0.0015),
    });
    // 记录代理结束事件（零值/None 情况），验证边界条件
    obs.record_event(&ObserverEvent::AgentEnd {
        provider: "openrouter".into(),
        model: "claude-sonnet".into(),
        duration: Duration::ZERO,
        tokens_used: None,
        cost_usd: None,
    });
    // 记录工具调用开始事件
    obs.record_event(&ObserverEvent::ToolCallStart { tool: "shell".into() });
    // 记录成功的工具调用事件
    obs.record_event(&ObserverEvent::ToolCall {
        tool: "shell".into(),
        duration: Duration::from_millis(10),
        success: true,
    });
    // 记录失败的工具调用事件
    obs.record_event(&ObserverEvent::ToolCall {
        tool: "file_read".into(),
        duration: Duration::from_millis(5),
        success: false,
    });
    // 记录对话轮次完成事件
    obs.record_event(&ObserverEvent::TurnComplete);
    // 记录通道消息事件
    obs.record_event(&ObserverEvent::ChannelMessage {
        channel: "telegram".into(),
        direction: "inbound".into(),
    });
    // 记录心跳事件
    obs.record_event(&ObserverEvent::HeartbeatTick);
    // 记录错误事件
    obs.record_event(&ObserverEvent::Error {
        component: "provider".into(),
        message: "timeout".into(),
    });
}

/// 测试所有指标类型的记录功能不会导致 panic
///
/// # 验证场景
///
/// 本测试记录以下指标类型：
/// - `RequestLatency`：请求延迟（2秒）
/// - `TokensUsed`：令牌使用量（500 和 0 两种情况）
/// - `ActiveSessions`：活跃会话数（3）
/// - `QueueDepth`：队列深度（42）
///
/// # 目的
///
/// 确保观察器能够处理所有定义的指标变体，包括不同的数值范围。
#[test]
fn records_all_metrics_without_panic() {
    let obs = test_observer();
    // 记录请求延迟指标
    obs.record_metric(&ObserverMetric::RequestLatency(Duration::from_secs(2)));
    // 记录令牌使用指标（非零值）
    obs.record_metric(&ObserverMetric::TokensUsed(500));
    // 记录令牌使用指标（零值），验证边界条件
    obs.record_metric(&ObserverMetric::TokensUsed(0));
    // 记录活跃会话数指标
    obs.record_metric(&ObserverMetric::ActiveSessions(3));
    // 记录队列深度指标
    obs.record_metric(&ObserverMetric::QueueDepth(42));
}

/// 测试 flush 操作不会导致 panic
///
/// # 验证场景
///
/// - 记录一个心跳事件后调用 flush
/// - 验证 flush 操作能够正常完成
///
/// # 目的
///
/// 确保 flush 方法在正常使用场景下稳定运行，不会抛出异常或 panic。
#[test]
fn flush_does_not_panic() {
    let obs = test_observer();
    obs.record_event(&ObserverEvent::HeartbeatTick);
    obs.flush();
}

/// 测试记录错误事件不会导致 panic
///
/// # 验证场景
///
/// 记录一个 `Error` 事件，包含：
/// - 组件标识：`provider`
/// - 错误消息：`connection refused to model endpoint`
///
/// # 目的
///
/// 确保观察器能够正确处理错误事件，即使在生产环境中出现连接失败等异常情况，
/// 观察器本身也不会成为故障源。
#[test]
fn otel_records_error_event_without_panic() {
    let obs = test_observer();
    obs.record_event(&ObserverEvent::Error {
        component: "provider".into(),
        message: "connection refused to model endpoint".into(),
    });
}

/// 测试记录 LLM 失败响应不会导致 panic
///
/// # 验证场景
///
/// 记录一个失败的 `LlmResponse` 事件，包含：
/// - 模型：`missing-model`（不存在的模型）
/// - 持续时间：0 毫秒（立即失败）
/// - 成功标志：`false`
/// - 错误消息：`404 Not Found`
/// - 令牌统计：`None`（未产生令牌消耗）
///
/// # 目的
///
/// 确保观察器能够正确处理 LLM 调用失败的场景，包括：
/// - 零持续时间的快速失败
/// - 缺失的令牌统计信息
/// - 详细的错误消息记录
#[test]
fn otel_records_llm_failure_without_panic() {
    let obs = test_observer();
    obs.record_event(&ObserverEvent::LlmResponse {
        provider: "openrouter".into(),
        model: "missing-model".into(),
        duration: Duration::from_millis(0),
        success: false,
        error_message: Some("404 Not Found".into()),
        input_tokens: None,
        output_tokens: None,
        cached_tokens: None,
        reasoning_tokens: None,
    });
}

/// 测试在端点不可达情况下的 flush 幂等性
///
/// # 验证场景
///
/// - 连续调用 flush 方法三次
/// - 测试端点 `http://127.0.0.1:19999` 不可达（无真实 Collector）
///
/// # 目的
///
/// 验证以下行为：
/// - flush 操作可以重复调用，不会累积错误状态
/// - 即使后端不可达，多次 flush 也不会导致 panic 或资源泄漏
/// - 观察器在降级模式下仍能保持稳定
#[test]
fn otel_flush_idempotent_with_unreachable_endpoint() {
    let obs = test_observer();
    obs.flush();
    obs.flush();
    obs.flush();
}

/// 测试记录零值指标不会导致 panic
///
/// # 验证场景
///
/// 记录以下零值指标：
/// - `RequestLatency`：零延迟（`Duration::ZERO`）
/// - `TokensUsed`：零令牌使用
/// - `ActiveSessions`：零活跃会话
/// - `QueueDepth`：零队列深度
///
/// # 目的
///
/// 确保观察器能够正确处理零值边界情况：
/// - 零延迟可能出现在快速失败或缓存命中的场景
/// - 零令牌可能出现在非 LLM 操作中
/// - 零会话/队列是合法的系统状态
/// - 所有零值都应被正确记录而非被忽略或导致异常
#[test]
fn otel_records_zero_duration_metrics() {
    let obs = test_observer();
    obs.record_metric(&ObserverMetric::RequestLatency(Duration::ZERO));
    obs.record_metric(&ObserverMetric::TokensUsed(0));
    obs.record_metric(&ObserverMetric::ActiveSessions(0));
    obs.record_metric(&ObserverMetric::QueueDepth(0));
}

/// 测试使用有效端点创建观察器能够成功
///
/// # 验证场景
///
/// - 端点：`http://127.0.0.1:12345`（本地测试端口，无需真实服务）
/// - 服务名称：`vibewindow-test`
/// - 验证创建结果为 `Ok`
///
/// # 目的
///
/// 确认观察器创建逻辑：
/// - 只要端点格式有效（符合 URL 规范），创建就会成功
/// - 不要求端点实际可达（允许延迟连接或失败重试）
/// - 这是 OpenTelemetry SDK 的标准行为，创建和连接是分离的
#[test]
fn otel_observer_creation_with_valid_endpoint_succeeds() {
    let result = OtelObserver::new(Some("http://127.0.0.1:12345"), Some("vibewindow-test"));
    assert!(result.is_ok(), "observer creation must succeed even with unreachable endpoint");
}

#[test]
fn otel_observer_defaults_and_downcast_are_available() {
    let obs =
        OtelObserver::new(None, None).expect("default observer configuration should be valid");

    assert_eq!(obs.name(), "otel");
    assert!(obs.as_any().downcast_ref::<OtelObserver>().is_some());
}

#[test]
fn otel_extreme_durations_use_safe_start_time_fallbacks() {
    let obs = test_observer();

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
        success: false,
    });
}

//! OpenTelemetry 观测者实现模块
//!
//! 本模块提供了基于 OpenTelemetry 协议（OTLP）的观测者实现，用于收集和导出
//! VibeWindow 代理运行时的遥测数据（traces 和 metrics）。
//!
//! # 主要功能
//!
//! - **分布式追踪（Tracing）**: 记录代理调用、LLM 请求、工具执行等操作的完整追踪链
//! - **指标收集（Metrics）**: 收集计数器、直方图、仪表盘等类型的运行时指标
//! - **OTLP 导出**: 通过 HTTP 协议将遥测数据导出到兼容 OpenTelemetry 的后端
//!
//! # 使用示例
//!
//! ```ignore
//! use observer_otel::OtelObserver;
//! use observer_types::{Observer, ObserverEvent};
//!
//! // 创建 OTLP 观测者实例，连接到本地 Collector
//! let observer = OtelObserver::new(
//!     Some("http://localhost:4318"),
//!     Some("vibewindow-agent")
//! )?;
//!
//! // 记录代理启动事件
//! observer.record_event(&ObserverEvent::AgentStart {
//!     provider: "openai".to_string(),
//!     model: "gpt-4".to_string(),
//! });
//!
//! // 刷新缓冲区，确保数据已发送
//! observer.flush();
//! ```
//!
//! # 架构说明
//!
//! 本模块实现了 [`Observer`] trait，作为观测子系统的具体实现之一。
//! 它可以与其他观测者实现（如日志观测者）并存，通过工厂模式注册和选择。

#![cfg(feature = "observability-otel")]

use observer_types::{Observer, ObserverEvent, ObserverMetric};
use opentelemetry::metrics::{Counter, Gauge, Histogram};
use opentelemetry::trace::{Span, SpanKind, Status, Tracer};
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::any::Any;
use std::time::SystemTime;

/// OpenTelemetry 观测者实现
///
/// 该结构体实现了 [`Observer`] trait，通过 OpenTelemetry 协议收集和导出
/// VibeWindow 代理的遥测数据。它维护了追踪和指标的提供者实例，以及
/// 各种预创建的指标工具（instruments）。
///
/// # 内部组件
///
/// - **Tracer Provider**: 管理分布式追踪的导出和采样
/// - **Meter Provider**: 管理指标的收集和周期性导出
/// - **预创建的指标工具**: 为了避免运行时开销，所有指标工具在初始化时创建
///
/// # 指标类型说明
///
/// - `Counter`: 单调递增的计数器，用于统计总次数（如请求次数、错误次数）
/// - `Histogram`: 分布统计，用于记录值的分布情况（如延迟分布）
/// - `Gauge`: 当前值测量，用于记录某一时刻的状态（如活跃会话数）
///
/// # 线程安全性
///
/// 该结构体内部的所有组件都是线程安全的，可以在多线程环境中共享使用。
pub struct OtelObserver {
    /// 追踪提供者，负责管理和导出分布式追踪数据
    tracer_provider: SdkTracerProvider,

    /// 指标提供者，负责收集和周期性导出指标数据
    meter_provider: SdkMeterProvider,

    // ===== 代理相关指标 =====
    /// 代理启动计数器：记录代理被调用的总次数
    agent_starts: Counter<u64>,

    /// 代理执行时长直方图：记录每次代理调用的耗时分布
    agent_duration: Histogram<f64>,

    // ===== LLM 相关指标 =====
    /// LLM 调用计数器：记录 LLM 提供者被调用的总次数
    llm_calls: Counter<u64>,

    /// LLM 调用时长直方图：记录每次 LLM 调用的耗时分布
    llm_duration: Histogram<f64>,

    // ===== 工具相关指标 =====
    /// 工具调用计数器：记录工具执行的总次数
    tool_calls: Counter<u64>,

    /// 工具执行时长直方图：记录每次工具执行的耗时分布
    tool_duration: Histogram<f64>,

    // ===== 通道相关指标 =====
    /// 通道消息计数器：记录通过各通道发送/接收的消息总数
    channel_messages: Counter<u64>,

    /// 心跳计数器：记录心跳信号的触发次数
    heartbeat_ticks: Counter<u64>,

    // ===== 错误相关指标 =====
    /// 错误计数器：按组件分类记录错误发生的总次数
    errors: Counter<u64>,

    // ===== 通用性能指标 =====
    /// 请求延迟直方图：记录请求处理的延迟分布
    request_latency: Histogram<f64>,

    /// Token 使用计数器：记录消耗的 token 总数（单调递增）
    tokens_used: Counter<u64>,

    // ===== 状态指标（Gauge）=====
    /// 活跃会话数仪表盘：记录当前活跃的会话数量
    active_sessions: Gauge<u64>,

    /// 队列深度仪表盘：记录当前消息队列的积压深度
    queue_depth: Gauge<u64>,
}

impl OtelObserver {
    /// 创建新的 OpenTelemetry 观测者实例
    ///
    /// 该方法会初始化 OTLP 导出器，连接到指定的 Collector 端点，
    /// 并创建所有必要的追踪和指标工具。
    ///
    /// # 参数
    ///
    /// - `endpoint`: OTLP Collector 的基础地址（可选）
    ///   - 默认值: `"http://localhost:4318"`
    ///   - 追踪端点: `{endpoint}/v1/traces`
    ///   - 指标端点: `{endpoint}/v1/metrics`
    ///
    /// - `service_name`: 服务标识名称（可选）
    ///   - 默认值: `"vibewindow"`
    ///   - 用于在追踪后端标识和过滤服务
    ///
    /// # 返回值
    ///
    /// - `Ok(Self)`: 成功创建的观测者实例
    /// - `Err(String)`: 初始化失败时的错误描述
    ///
    /// # 错误情况
    ///
    /// - OTLP span exporter 创建失败
    /// - OTLP metric exporter 创建失败
    /// - 网络连接问题（在后续导出时才会体现）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 使用默认配置
    /// let observer = OtelObserver::new(None, None)?;
    ///
    /// // 指定自定义端点和服务名
    /// let observer = OtelObserver::new(
    ///     Some("http://collector.example.com:4318"),
    ///     Some("production-agent")
    /// )?;
    /// ```
    ///
    /// # 全局状态
    ///
    /// 该方法会设置全局的 tracer provider 和 meter provider，
    /// 多次调用会覆盖之前的全局设置。
    pub fn new(endpoint: Option<&str>, service_name: Option<&str>) -> Result<Self, String> {
        // 确定基础端点地址，移除尾部斜杠以避免重复
        let base_endpoint = endpoint.unwrap_or("http://localhost:4318");
        let traces_endpoint = format!("{}/v1/traces", base_endpoint.trim_end_matches('/'));
        let metrics_endpoint = format!("{}/v1/metrics", base_endpoint.trim_end_matches('/'));

        // 确定服务名称
        let service_name = service_name.unwrap_or("vibewindow");

        // ===== 初始化追踪导出器和提供者 =====

        // 创建 HTTP 方式的 Span 导出器，将追踪数据发送到 /v1/traces 端点
        let span_exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(&traces_endpoint)
            .build()
            .map_err(|e| format!("Failed to create OTLP span exporter: {e}"))?;

        // 构建 Tracer Provider，配置批处理导出和资源标签
        let tracer_provider = SdkTracerProvider::builder()
            .with_batch_exporter(span_exporter) // 使用批处理模式提高性能
            .with_resource(
                opentelemetry_sdk::Resource::builder()
                    .with_service_name(service_name.to_string())
                    .build(),
            )
            .build();

        // 设置为全局 tracer provider，供全局 API 使用
        global::set_tracer_provider(tracer_provider.clone());

        // ===== 初始化指标导出器和提供者 =====

        // 创建 HTTP 方式的 Metric 导出器，将指标数据发送到 /v1/metrics 端点
        let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_http()
            .with_endpoint(&metrics_endpoint)
            .build()
            .map_err(|e| format!("Failed to create OTLP metric exporter: {e}"))?;

        // 创建周期性读取器，定时将指标数据推送到后端
        let metric_reader =
            opentelemetry_sdk::metrics::PeriodicReader::builder(metric_exporter).build();

        // 构建 Meter Provider，配置周期性读取和资源标签
        let meter_provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
            .with_reader(metric_reader) // 添加周期性读取器
            .with_resource(
                opentelemetry_sdk::Resource::builder()
                    .with_service_name(service_name.to_string())
                    .build(),
            )
            .build();

        // 保留一份克隆用于返回，原始实例设置为全局 meter provider
        let meter_provider_clone = meter_provider.clone();
        global::set_meter_provider(meter_provider);

        // ===== 创建指标工具 =====

        // 获取 meter 实例，用于创建各种指标工具
        let meter = global::meter("vibewindow");

        // 创建代理相关指标
        let agent_starts = meter
            .u64_counter("vibewindow.agent.starts")
            .with_description("Total agent invocations")
            .build();

        let agent_duration = meter
            .f64_histogram("vibewindow.agent.duration")
            .with_description("Agent invocation duration in seconds")
            .with_unit("s")
            .build();

        // 创建 LLM 相关指标
        let llm_calls = meter
            .u64_counter("vibewindow.llm.calls")
            .with_description("Total LLM provider calls")
            .build();

        let llm_duration = meter
            .f64_histogram("vibewindow.llm.duration")
            .with_description("LLM provider call duration in seconds")
            .with_unit("s")
            .build();

        // 创建工具相关指标
        let tool_calls =
            meter.u64_counter("vibewindow.tool.calls").with_description("Total tool calls").build();

        let tool_duration = meter
            .f64_histogram("vibewindow.tool.duration")
            .with_description("Tool execution duration in seconds")
            .with_unit("s")
            .build();

        // 创建通道和心跳指标
        let channel_messages = meter
            .u64_counter("vibewindow.channel.messages")
            .with_description("Total channel messages")
            .build();

        let heartbeat_ticks = meter
            .u64_counter("vibewindow.heartbeat.ticks")
            .with_description("Total heartbeat ticks")
            .build();

        // 创建错误计数指标
        let errors = meter
            .u64_counter("vibewindow.errors")
            .with_description("Total errors by component")
            .build();

        // 创建性能相关指标
        let request_latency = meter
            .f64_histogram("vibewindow.request.latency")
            .with_description("Request latency in seconds")
            .with_unit("s")
            .build();

        let tokens_used = meter
            .u64_counter("vibewindow.tokens.used")
            .with_description("Total tokens consumed (monotonic)")
            .build();

        // 创建状态指标（Gauge 类型）
        let active_sessions = meter
            .u64_gauge("vibewindow.sessions.active")
            .with_description("Current number of active sessions")
            .build();

        let queue_depth = meter
            .u64_gauge("vibewindow.queue.depth")
            .with_description("Current message queue depth")
            .build();

        // 返回初始化完成的观测者实例
        Ok(Self {
            tracer_provider,
            meter_provider: meter_provider_clone,
            agent_starts,
            agent_duration,
            llm_calls,
            llm_duration,
            tool_calls,
            tool_duration,
            channel_messages,
            heartbeat_ticks,
            errors,
            request_latency,
            tokens_used,
            active_sessions,
            queue_depth,
        })
    }
}

impl Observer for OtelObserver {
    /// 记录观测事件
    ///
    /// 根据事件类型，更新相应的指标并创建追踪 Span。
    /// 该方法是观测系统的核心，处理所有类型的运行时事件。
    ///
    /// # 参数
    ///
    /// - `event`: 要记录的观测事件，包含事件类型和相关属性
    ///
    /// # 事件处理说明
    ///
    /// - `AgentStart`: 增加代理启动计数，带 provider 和 model 标签
    /// - `LlmRequest/ToolCallStart/TurnComplete`: 当前仅作为占位符，无实际操作
    /// - `LlmResponse`: 记录 LLM 调用指标和追踪 Span
    /// - `AgentEnd`: 记录代理完成指标和追踪 Span
    /// - `ToolCall`: 记录工具调用指标和追踪 Span
    /// - `ChannelMessage`: 增加通道消息计数
    /// - `HeartbeatTick`: 增加心跳计数
    /// - `Error`: 创建错误追踪 Span 并增加错误计数
    fn record_event(&self, event: &ObserverEvent) {
        // 获取全局 tracer 实例
        let tracer = global::tracer("vibewindow");

        match event {
            // 代理启动事件：仅更新计数器
            ObserverEvent::AgentStart { provider, model } => {
                self.agent_starts.add(
                    1,
                    &[
                        KeyValue::new("provider", provider.clone()),
                        KeyValue::new("model", model.clone()),
                    ],
                );
            }

            // 占位符事件：暂无具体实现
            ObserverEvent::LlmRequest { .. }
            | ObserverEvent::ToolCallStart { .. }
            | ObserverEvent::TurnComplete => {}

            // LLM 响应事件：记录调用次数、延迟和追踪 Span
            ObserverEvent::LlmResponse {
                provider,
                model,
                duration,
                success,
                error_message: _, // 未使用，但保留在事件定义中
                input_tokens: _,  // 未来可添加到指标
                output_tokens: _, // 未来可添加到指标
                cached_tokens: _,
                reasoning_tokens: _,
            } => {
                let secs = duration.as_secs_f64();

                // 构建指标属性
                let attrs = [
                    KeyValue::new("provider", provider.clone()),
                    KeyValue::new("model", model.clone()),
                    KeyValue::new("success", success.to_string()),
                ];

                // 更新计数器和直方图
                self.llm_calls.add(1, &attrs);
                self.llm_duration.record(secs, &attrs);

                // 计算 Span 的起始时间（当前时间减去持续时间）
                let start_time =
                    SystemTime::now().checked_sub(*duration).unwrap_or(SystemTime::now());

                // 构建 LLM 调用的追踪 Span
                let mut span = tracer.build(
                    opentelemetry::trace::SpanBuilder::from_name("llm.call")
                        .with_kind(SpanKind::Internal)
                        .with_start_time(start_time)
                        .with_attributes(vec![
                            KeyValue::new("provider", provider.clone()),
                            KeyValue::new("model", model.clone()),
                            KeyValue::new("success", *success),
                            KeyValue::new("duration_s", secs),
                        ]),
                );

                // 根据成功/失败设置 Span 状态
                if *success {
                    span.set_status(Status::Ok);
                } else {
                    span.set_status(Status::error(""));
                }

                // 结束 Span，标记为已完成
                span.end();
            }

            // 代理结束事件：记录执行时长、token 使用和成本
            ObserverEvent::AgentEnd { provider, model, duration, tokens_used, cost_usd } => {
                let secs = duration.as_secs_f64();

                // 计算回溯的起始时间
                let start_time =
                    SystemTime::now().checked_sub(*duration).unwrap_or(SystemTime::now());

                // 构建代理调用的追踪 Span
                let mut span = tracer.build(
                    opentelemetry::trace::SpanBuilder::from_name("agent.invocation")
                        .with_kind(SpanKind::Internal)
                        .with_start_time(start_time)
                        .with_attributes(vec![
                            KeyValue::new("provider", provider.clone()),
                            KeyValue::new("model", model.clone()),
                            KeyValue::new("duration_s", secs),
                        ]),
                );

                // 可选：添加 token 使用量属性
                if let Some(t) = tokens_used {
                    span.set_attribute(KeyValue::new("tokens_used", *t as i64));
                }

                // 可选：添加成本属性（美元）
                if let Some(c) = cost_usd {
                    span.set_attribute(KeyValue::new("cost_usd", *c));
                }

                span.end();

                // 记录代理执行时长到直方图
                self.agent_duration.record(
                    secs,
                    &[
                        KeyValue::new("provider", provider.clone()),
                        KeyValue::new("model", model.clone()),
                    ],
                );
            }

            // 工具调用事件：记录执行次数、时长和结果
            ObserverEvent::ToolCall { tool, duration, success } => {
                let secs = duration.as_secs_f64();

                // 计算回溯的起始时间
                let start_time =
                    SystemTime::now().checked_sub(*duration).unwrap_or(SystemTime::now());

                // 确定状态（成功或错误）
                let status = if *success { Status::Ok } else { Status::error("") };

                // 构建工具调用的追踪 Span
                let mut span = tracer.build(
                    opentelemetry::trace::SpanBuilder::from_name("tool.call")
                        .with_kind(SpanKind::Internal)
                        .with_start_time(start_time)
                        .with_attributes(vec![
                            KeyValue::new("tool.name", tool.clone()),
                            KeyValue::new("tool.success", *success),
                            KeyValue::new("duration_s", secs),
                        ]),
                );
                span.set_status(status);
                span.end();

                // 更新工具调用指标
                let attrs = [
                    KeyValue::new("tool", tool.clone()),
                    KeyValue::new("success", success.to_string()),
                ];
                self.tool_calls.add(1, &attrs);
                self.tool_duration.record(secs, &[KeyValue::new("tool", tool.clone())]);
            }

            // 通道消息事件：记录消息传输
            ObserverEvent::ChannelMessage { channel, direction } => {
                self.channel_messages.add(
                    1,
                    &[
                        KeyValue::new("channel", channel.clone()),
                        KeyValue::new("direction", direction.clone()),
                    ],
                );
            }

            // 心跳事件：记录心跳信号
            ObserverEvent::HeartbeatTick => {
                self.heartbeat_ticks.add(1, &[]);
            }

            // 错误事件：创建错误 Span 并增加错误计数
            ObserverEvent::Error { component, message } => {
                // 构建错误追踪 Span
                let mut span = tracer.build(
                    opentelemetry::trace::SpanBuilder::from_name("error")
                        .with_kind(SpanKind::Internal)
                        .with_attributes(vec![
                            KeyValue::new("component", component.clone()),
                            KeyValue::new("error.message", message.clone()),
                        ]),
                );
                span.set_status(Status::error(message.clone()));
                span.end();

                // 按组件分类记录错误次数
                self.errors.add(1, &[KeyValue::new("component", component.clone())]);
            }
        }
    }

    /// 记录观测指标
    ///
    /// 该方法处理独立的指标记录请求，不涉及追踪 Span 的创建。
    /// 主要用于记录系统运行时的性能和状态指标。
    ///
    /// # 参数
    ///
    /// - `metric`: 要记录的指标，包含指标类型和值
    ///
    /// # 指标类型说明
    ///
    /// - `RequestLatency`: 请求延迟（直方图）
    /// - `TokensUsed`: Token 使用量（计数器）
    /// - `ActiveSessions`: 活跃会话数（仪表盘）
    /// - `QueueDepth`: 队列深度（仪表盘）
    fn record_metric(&self, metric: &ObserverMetric) {
        match metric {
            // 记录请求延迟到直方图
            ObserverMetric::RequestLatency(d) => {
                self.request_latency.record(d.as_secs_f64(), &[]);
            }

            // 累加 token 使用量
            ObserverMetric::TokensUsed(t) => {
                self.tokens_used.add(*t as u64, &[]);
            }

            // 更新当前活跃会话数
            ObserverMetric::ActiveSessions(s) => {
                self.active_sessions.record(*s as u64, &[]);
            }

            // 更新当前队列深度
            ObserverMetric::QueueDepth(d) => {
                self.queue_depth.record(*d as u64, &[]);
            }
        }
    }

    /// 刷新缓冲区
    ///
    /// 强制将所有缓存的追踪和指标数据导出到后端。
    /// 通常在应用关闭前调用，确保数据不丢失。
    ///
    /// # 行为说明
    ///
    /// - 尝试强制刷新 tracer provider 的缓冲区
    /// - 尝试强制刷新 meter provider 的缓冲区
    /// - 刷新失败时记录警告日志，但不返回错误
    ///
    /// # 注意事项
    ///
    /// - 该方法是同步阻塞的，可能会花费一些时间
    /// - 即使刷新失败，也不会影响后续的数据收集
    fn flush(&self) {
        // 强制刷新追踪数据
        if let Err(e) = self.tracer_provider.force_flush() {
            tracing::warn!("OTel trace flush failed: {e}");
        }

        // 强制刷新指标数据
        if let Err(e) = self.meter_provider.force_flush() {
            tracing::warn!("OTel metric flush failed: {e}");
        }
    }

    /// 返回观测者名称
    ///
    /// 返回用于标识此观测者类型的字符串常量。
    ///
    /// # 返回值
    ///
    /// 总是返回 `"otel"`，表示这是一个 OpenTelemetry 观测者实现。
    fn name(&self) -> &str {
        "otel"
    }

    /// 返回类型擦除的自引用
    ///
    /// 提供 `Any` trait 的向下转型能力，允许调用者在需要时
    /// 将 trait 对象转换回具体类型。
    ///
    /// # 使用场景
    ///
    /// - 需要访问 OpenTelemetry 特定功能时
    /// - 进行类型检查或运行时类型识别时
    ///
    /// # 返回值
    ///
    /// 返回指向 `self` 的 `dyn Any` 引用。
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests;

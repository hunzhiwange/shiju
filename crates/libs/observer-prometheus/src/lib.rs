//! Prometheus 指标观测器。
//!
//! 本模块把运行时事件映射为 Prometheus counter、gauge 和 histogram，并提供文本编码入口。
//! 指标只记录聚合标签和值，不保存原始请求内容或敏感载荷。

use observer_types::{Observer, ObserverEvent, ObserverMetric};
use prometheus::{
    Encoder, GaugeVec, Histogram, HistogramOpts, HistogramVec, IntCounterVec, Registry, TextEncoder,
};

/// 基于 Prometheus registry 的观测后端。
///
/// 每个实例维护独立 `Registry`，便于测试和多工作区运行时隔离指标集合。
pub struct PrometheusObserver {
    registry: Registry,

    agent_starts: IntCounterVec,
    llm_requests: IntCounterVec,
    tokens_input_total: IntCounterVec,
    tokens_output_total: IntCounterVec,
    tool_calls: IntCounterVec,
    channel_messages: IntCounterVec,
    heartbeat_ticks: prometheus::IntCounter,
    errors: IntCounterVec,

    agent_duration: HistogramVec,
    tool_duration: HistogramVec,
    request_latency: Histogram,

    tokens_used: prometheus::IntGauge,
    active_sessions: GaugeVec,
    queue_depth: GaugeVec,
}

impl PrometheusObserver {
    /// 创建并注册所有 VibeWindow 运行时指标。
    ///
    /// # 返回值
    ///
    /// 返回可直接传入观测工厂的 `PrometheusObserver`。
    ///
    /// # 错误处理
    ///
    /// Prometheus 指标定义是静态常量，构造失败表示代码内指标名或标签非法，因此这里使用
    /// `expect` 在开发期暴露问题；重复注册则被忽略以保持初始化幂等。
    pub fn new() -> Self {
        let registry = Registry::new();

        let agent_starts = IntCounterVec::new(
            prometheus::Opts::new("vibewindow_agent_starts_total", "Total agent invocations"),
            &["provider", "model"],
        )
        .expect("valid metric");

        let llm_requests = IntCounterVec::new(
            prometheus::Opts::new("vibewindow_llm_requests_total", "Total LLM provider requests"),
            &["provider", "model", "success"],
        )
        .expect("valid metric");

        let tokens_input_total = IntCounterVec::new(
            prometheus::Opts::new("vibewindow_tokens_input_total", "Total input tokens consumed"),
            &["provider", "model"],
        )
        .expect("valid metric");

        let tokens_output_total = IntCounterVec::new(
            prometheus::Opts::new("vibewindow_tokens_output_total", "Total output tokens consumed"),
            &["provider", "model"],
        )
        .expect("valid metric");

        let tool_calls = IntCounterVec::new(
            prometheus::Opts::new("vibewindow_tool_calls_total", "Total tool calls"),
            &["tool", "success"],
        )
        .expect("valid metric");

        let channel_messages = IntCounterVec::new(
            prometheus::Opts::new("vibewindow_channel_messages_total", "Total channel messages"),
            &["channel", "direction"],
        )
        .expect("valid metric");

        let heartbeat_ticks = prometheus::IntCounter::new(
            "vibewindow_heartbeat_ticks_total",
            "Total heartbeat ticks",
        )
        .expect("valid metric");

        let errors = IntCounterVec::new(
            prometheus::Opts::new("vibewindow_errors_total", "Total errors by component"),
            &["component"],
        )
        .expect("valid metric");

        let agent_duration = HistogramVec::new(
            HistogramOpts::new(
                "vibewindow_agent_duration_seconds",
                "Agent invocation duration in seconds",
            )
            .buckets(vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0]),
            &["provider", "model"],
        )
        .expect("valid metric");

        let tool_duration = HistogramVec::new(
            HistogramOpts::new(
                "vibewindow_tool_duration_seconds",
                "Tool execution duration in seconds",
            )
            .buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
            &["tool"],
        )
        .expect("valid metric");

        let request_latency = Histogram::with_opts(
            HistogramOpts::new("vibewindow_request_latency_seconds", "Request latency in seconds")
                .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
        )
        .expect("valid metric");

        let tokens_used = prometheus::IntGauge::new(
            "vibewindow_tokens_used_last",
            "Tokens used in the last request",
        )
        .expect("valid metric");

        let active_sessions = GaugeVec::new(
            prometheus::Opts::new("vibewindow_active_sessions", "Number of active sessions"),
            &[],
        )
        .expect("valid metric");

        let queue_depth = GaugeVec::new(
            prometheus::Opts::new("vibewindow_queue_depth", "Message queue depth"),
            &[],
        )
        .expect("valid metric");

        // 独立 registry 初始为空；忽略注册错误可以避免测试重复初始化时把后端置为不可用。
        registry.register(Box::new(agent_starts.clone())).ok();
        registry.register(Box::new(llm_requests.clone())).ok();
        registry.register(Box::new(tokens_input_total.clone())).ok();
        registry.register(Box::new(tokens_output_total.clone())).ok();
        registry.register(Box::new(tool_calls.clone())).ok();
        registry.register(Box::new(channel_messages.clone())).ok();
        registry.register(Box::new(heartbeat_ticks.clone())).ok();
        registry.register(Box::new(errors.clone())).ok();
        registry.register(Box::new(agent_duration.clone())).ok();
        registry.register(Box::new(tool_duration.clone())).ok();
        registry.register(Box::new(request_latency.clone())).ok();
        registry.register(Box::new(tokens_used.clone())).ok();
        registry.register(Box::new(active_sessions.clone())).ok();
        registry.register(Box::new(queue_depth.clone())).ok();

        Self {
            registry,
            agent_starts,
            llm_requests,
            tokens_input_total,
            tokens_output_total,
            tool_calls,
            channel_messages,
            heartbeat_ticks,
            errors,
            agent_duration,
            tool_duration,
            request_latency,
            tokens_used,
            active_sessions,
            queue_depth,
        }
    }

    /// 将当前指标编码为 Prometheus 文本格式。
    ///
    /// # 返回值
    ///
    /// 返回可直接暴露给抓取端的文本；编码或 UTF-8 转换失败时返回空字符串。
    ///
    /// # 错误处理
    ///
    /// 观测路径不应影响主业务流程，因此编码失败被降级为空输出。
    pub fn encode(&self) -> String {
        let encoder = TextEncoder::new();
        let families = self.registry.gather();
        let mut buf = Vec::new();
        encoder.encode(&families, &mut buf).unwrap_or_default();
        String::from_utf8(buf).unwrap_or_default()
    }
}

impl Observer for PrometheusObserver {
    fn record_event(&self, event: &ObserverEvent) {
        match event {
            ObserverEvent::AgentStart { provider, model } => {
                self.agent_starts.with_label_values(&[provider, model]).inc();
            }
            ObserverEvent::AgentEnd { provider, model, duration, tokens_used, cost_usd: _ } => {
                self.agent_duration
                    .with_label_values(&[provider, model])
                    .observe(duration.as_secs_f64());
                if let Some(t) = tokens_used {
                    self.tokens_used.set(i64::try_from(*t).unwrap_or(i64::MAX));
                }
            }
            ObserverEvent::LlmResponse {
                provider,
                model,
                success,
                input_tokens,
                output_tokens,
                cached_tokens: _,
                reasoning_tokens: _,
                ..
            } => {
                let success_str = if *success { "true" } else { "false" };
                // 只把成功状态作为低基数标签，错误详情不进入指标，避免泄露请求或供应商响应内容。
                self.llm_requests
                    .with_label_values(&[provider.as_str(), model.as_str(), success_str])
                    .inc();
                if let Some(input) = input_tokens {
                    self.tokens_input_total
                        .with_label_values(&[provider.as_str(), model.as_str()])
                        .inc_by(*input);
                }
                if let Some(output) = output_tokens {
                    self.tokens_output_total
                        .with_label_values(&[provider.as_str(), model.as_str()])
                        .inc_by(*output);
                }
            }
            ObserverEvent::ToolCallStart { tool: _ }
            | ObserverEvent::TurnComplete
            | ObserverEvent::LlmRequest { .. } => {}
            ObserverEvent::ToolCall { tool, duration, success } => {
                let success_str = if *success { "true" } else { "false" };
                self.tool_calls.with_label_values(&[tool.as_str(), success_str]).inc();
                self.tool_duration
                    .with_label_values(&[tool.as_str()])
                    .observe(duration.as_secs_f64());
            }
            ObserverEvent::ChannelMessage { channel, direction } => {
                self.channel_messages.with_label_values(&[channel, direction]).inc();
            }
            ObserverEvent::HeartbeatTick => {
                self.heartbeat_ticks.inc();
            }
            ObserverEvent::Error { component, message: _ } => {
                // 错误消息可能含路径或上游文本，只按组件聚合计数。
                self.errors.with_label_values(&[component]).inc();
            }
        }
    }

    fn record_metric(&self, metric: &ObserverMetric) {
        match metric {
            ObserverMetric::RequestLatency(d) => {
                self.request_latency.observe(d.as_secs_f64());
            }
            ObserverMetric::TokensUsed(t) => {
                self.tokens_used.set(i64::try_from(*t).unwrap_or(i64::MAX));
            }
            ObserverMetric::ActiveSessions(s) => {
                self.active_sessions.with_label_values(&[] as &[&str]).set(*s as f64);
            }
            ObserverMetric::QueueDepth(d) => {
                self.queue_depth.with_label_values(&[] as &[&str]).set(*d as f64);
            }
        }
    }

    fn name(&self) -> &str {
        "prometheus"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests;

//! # 可观测性模块 (Observability)
//!
//! 本模块为 VibeWindow 代理系统提供统一的可观测性基础设施，包括日志、指标和分布式追踪能力。
//!
//! ## 架构概述
//!
//! 本模块采用 **trait + 工厂** 架构，所有观测后端均实现 [`Observer`] trait，
//! 通过 [`create_observer`] 工厂函数根据配置动态选择具体实现。这种设计允许：
//!
//! - **零成本抽象**：未启用的后端不会带来运行时开销
//! - **可扩展性**：新增观测后端只需实现 trait 并注册到工厂
//! - **运行时切换**：通过配置文件灵活选择观测策略
//!
//! ## 子模块
//!
//! | 模块 | 用途 | 适用场景 |
//! |------|------|----------|
//! | [`log`] | 日志输出观察者 | 开发调试、简单场景 |
//! | [`prometheus`] | Prometheus 指标导出器 | 生产环境指标采集 |
//! | [`otel`] | OpenTelemetry 集成 | 分布式追踪与标准化可观测性 |
//! | [`multi`] | 多观察者组合器 | 同时输出到多个后端 |
//! | [`noop`] | 空操作观察者 | 禁用观测或回退场景 |
//! | [`verbose`] | 详细日志观察者 | 深度调试 |
//! | [`runtime_trace`] | 运行时追踪工具 | 性能分析与调用链追踪 |
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use vibe_window::app::agent::observability::{create_observer, Observer};
//! use vibe_window::app::agent::config::ObservabilityConfig;
//!
//! // 从配置创建观察者
//! let config = ObservabilityConfig {
//!     backend: "prometheus".to_string(),
//!     ..Default::default()
//! };
//! let observer = create_observer(&config);
//!
//! // 记录事件
//! observer.record_event(ObserverEvent::AgentStarted { agent_id: "agent-001" });
//! ```
//!
//! ## 特性标志
//!
//! - `observability-otel`：启用 OpenTelemetry 支持，需要额外的依赖项

pub mod log;
pub mod multi;
pub mod noop;
#[cfg(feature = "observability-otel")]
pub mod otel;
pub mod prometheus;
pub mod runtime_trace;
pub mod traits;
pub mod verbose;

// ============================================================================
// 类型重导出
// ============================================================================

/// 日志观察者 - 将观测事件输出到标准日志流
///
/// 适用于开发环境或不需要结构化指标的场景。
#[allow(unused_imports)]
pub use self::log::LogObserver;

/// 多观察者组合器 - 允许同时使用多个观测后端
///
/// 使用场景示例：同时输出到日志和 Prometheus
#[allow(unused_imports)]
pub use self::multi::MultiObserver;

/// 空操作观察者 - 不执行任何观测动作
///
/// 用于以下场景：
/// - 明确禁用可观测性
/// - 当请求的后端不可用时的回退选项
/// - 测试环境中隔离观测副作用
pub use noop::NoopObserver;

/// OpenTelemetry 观察者 - 标准化的分布式追踪与指标导出
///
/// 仅在启用 `observability-otel` 特性时可用。
/// 支持导出到任何兼容 OTLP 的后端（如 Jaeger、Zipkin、Grafana Tempo）。
#[cfg(feature = "observability-otel")]
pub use otel::OtelObserver;

/// Prometheus 观察者 - 暴露 Prometheus 格式的指标端点
///
/// 提供与 Prometheus 生态系统无缝集成的指标导出能力。
pub use prometheus::PrometheusObserver;

/// 核心观测 trait 与事件类型定义
///
/// - [`Observer`]：所有观测后端必须实现的 trait
/// - [`ObserverEvent`]：统一的事件抽象
pub use traits::{Observer, ObserverEvent};

/// 详细日志观察者 - 提供更丰富的调试信息
///
/// 相比 [`LogObserver`]，输出更详细的上下文和调用栈信息。
#[allow(unused_imports)]
pub use verbose::VerboseObserver;

use crate::app::agent::config::ObservabilityConfig;

// ============================================================================
// 工厂函数
// ============================================================================

/// 根据配置创建观察者实例
///
/// 这是本模块的主入口点，负责解析 `backend` 配置并实例化对应的观察者实现。
/// 该函数封装了所有条件编译逻辑，调用者无需关心具体后端的可用性。
///
/// # 参数
///
/// * `config` - 可观测性配置引用，主要使用以下字段：
///   - `backend`：后端类型标识符（必需）
///   - `otel_endpoint`：OpenTelemetry Collector 端点（仅 otel 后端）
///   - `otel_service_name`：OpenTelemetry 服务名（仅 otel 后端）
///
/// # 返回值
///
/// 返回装箱的 trait 对象 `Box<dyn Observer>`，保证调用者可使用统一接口。
///
/// # 支持的后端类型
///
/// | backend 值 | 返回类型 | 备注 |
/// |-----------|---------|------|
/// | `"log"` | [`LogObserver`] | 基础日志输出 |
/// | `"prometheus"` | [`PrometheusObserver`] | Prometheus 指标 |
/// | `"otel"`, `"opentelemetry"`, `"otlp"` | [`OtelObserver`] 或 [`NoopObserver`] | 需 `observability-otel` 特性 |
/// | `"none"`, `"noop"` | [`NoopObserver`] | 显式禁用 |
/// | 其他 | [`NoopObserver`] | 未知后端，记录警告并回退 |
///
/// # 回退行为
///
/// 在以下情况下会自动回退到 [`NoopObserver`]：
/// 1. 请求的 `otel` 后端但编译时未启用 `observability-otel` 特性
/// 2. `otel` 后端初始化失败（如网络连接问题）
/// 3. 指定了未知后端类型
///
/// 所有回退都会通过 [`tracing`] 宏记录警告或错误信息。
///
/// # 示例
///
/// ```rust,ignore
/// use vibe_window::app::agent::config::ObservabilityConfig;
///
/// // 创建 Prometheus 观察者
/// let config = ObservabilityConfig {
///     backend: "prometheus".to_string(),
///     ..Default::default()
/// };
/// let observer = create_observer(&config);
///
/// // 创建 OpenTelemetry 观察者（带自定义端点）
/// let otel_config = ObservabilityConfig {
///     backend: "otel".to_string(),
///     otel_endpoint: Some("http://jaeger:4318".to_string()),
///     otel_service_name: Some("vibewindow-agent".to_string()),
///     ..Default::default()
/// };
/// let otel_observer = create_observer(&otel_config);
/// ```
///
/// # 线程安全
///
/// 返回的观察者实例是 `Send + Sync` 的（由 trait 约束保证），
/// 可安全地在多线程环境中共享使用。
pub fn create_observer(config: &ObservabilityConfig) -> Box<dyn Observer> {
    // 根据配置的 backend 字段分发到对应的观察者实现
    // 每个 match 分支返回一个装箱的 Observer trait 对象
    match config.backend.as_str() {
        // 日志后端：最简单的实现，将事件输出到标准日志
        "log" => Box::new(LogObserver::new()),

        // Prometheus 后端：暴露 /metrics 端点供 Prometheus 抓取
        "prometheus" => Box::new(PrometheusObserver::new()),

        // OpenTelemetry 家族：支持多种命名别名以提高可用性
        "otel" | "opentelemetry" | "otlp" => {
            // 条件编译：仅在启用 observability-otel 特性时包含 OTel 实现
            #[cfg(feature = "observability-otel")]
            {
                // 尝试初始化 OpenTelemetry 观察者
                // 传入可选的端点和服务名配置
                match OtelObserver::new(
                    config.otel_endpoint.as_deref(),
                    config.otel_service_name.as_deref(),
                ) {
                    Ok(obs) => {
                        // 初始化成功：记录配置的端点信息（回退显示默认值）
                        let endpoint =
                            config.otel_endpoint.as_deref().unwrap_or("http://localhost:4318");
                        tracing::info!(endpoint = endpoint, "OpenTelemetry observer initialized");
                        Box::new(obs)
                    }
                    Err(e) => {
                        // 初始化失败：记录错误并回退到空操作观察者
                        // 这样可以确保系统继续运行，而不是因为观测层失败而崩溃
                        tracing::error!(
                            "Failed to create OTel observer: {e}. Falling back to noop."
                        );
                        Box::new(NoopObserver)
                    }
                }
            }

            // 当请求 OpenTelemetry 但编译时未启用该特性时的处理
            #[cfg(not(feature = "observability-otel"))]
            {
                // 记录警告：帮助运维人员识别配置问题
                tracing::warn!(
                    "OpenTelemetry backend requested but this build was compiled without `observability-otel`; falling back to noop."
                );
                Box::new(NoopObserver)
            }
        }

        // 显式禁用观测的场景
        "none" | "noop" => Box::new(NoopObserver),

        // 未知后端：防御性编程，避免因拼写错误导致系统崩溃
        _ => {
            tracing::warn!(
                "Unknown observability backend '{}', falling back to noop",
                config.backend
            );
            Box::new(NoopObserver)
        }
    }
}

/// 可观测性模块的单元测试
///
/// 测试覆盖以下场景：
/// - 各种后端类型的创建
/// - 回退行为的正确性
/// - 条件编译路径
#[cfg(test)]
mod tests;

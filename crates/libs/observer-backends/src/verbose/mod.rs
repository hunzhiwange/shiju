//! 详细输出观察器模块
//!
//! 本模块提供 `VerboseObserver` 实现，用于在控制台输出详细的代理运行时事件信息。
//! 主要用于调试、开发和演示场景，帮助开发者理解代理的执行流程和性能特征。
//!
//! # 功能特性
//!
//! - **事件跟踪**：记录 LLM 请求/响应、工具调用、轮次完成等关键事件
//! - **性能监控**：输出操作耗时（毫秒级精度）
//! - **状态可视化**：通过标准错误流（stderr）输出带格式的状态信息
//! - **轻量级**：无状态实现，不依赖外部存储或网络服务
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use observer_backends::VerboseObserver;
//! use observer_types::{Observer, ObserverEvent};
//!
//! let observer = VerboseObserver::new();
//! observer.record_event(&ObserverEvent::TurnComplete);
//! ```

use observer_types::{Observer, ObserverEvent, ObserverMetric};
use std::any::Any;

/// 详细输出观察器
///
/// 一个无状态的观察器实现，通过标准错误流（stderr）输出代理运行时事件的详细信息。
/// 适用于需要实时查看代理行为的场景，如开发调试、演示或问题排查。
///
/// # 输出格式
///
/// 所有输出都以符号前缀标识：
/// - `>` 表示开始/发送事件
/// - `<` 表示完成/接收事件
///
/// # 线程安全性
///
/// 由于 `VerboseObserver` 不包含任何内部状态，可以安全地在多线程环境中共享。
pub struct VerboseObserver;

impl VerboseObserver {
    /// 创建一个新的详细输出观察器实例
    ///
    /// # 返回值
    ///
    /// 返回一个无状态的 `VerboseObserver` 实例，可以立即用于记录事件。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let observer = VerboseObserver::new();
    /// ```
    pub fn new() -> Self {
        Self
    }
}

/// 实现 `Observer` trait，为代理运行时事件提供详细的控制台输出
impl Observer for VerboseObserver {
    /// 记录并输出观察器事件
    ///
    /// 根据事件类型，在标准错误流（stderr）输出相应的格式化信息。
    /// 不同事件类型会触发不同的输出：
    ///
    /// # 支持的事件类型
    ///
    /// - **LlmRequest**：输出提供商、模型和消息数量，显示为 `> Send`
    /// - **LlmResponse**：输出成功状态和耗时（毫秒），显示为 `< Receive`
    /// - **ToolCallStart**：输出工具名称，显示为 `> Tool`
    /// - **ToolCall**：输出工具执行结果和耗时，显示为 `< Tool`
    /// - **TurnComplete**：输出轮次完成标记，显示为 `< Complete`
    /// - **其他事件**：忽略（不产生输出）
    ///
    /// # 参数
    ///
    /// - `event`：要记录的观察器事件引用
    fn record_event(&self, event: &ObserverEvent) {
        match event {
            // LLM 请求事件：输出提供商、模型和消息数量
            ObserverEvent::LlmRequest { provider, model, messages_count } => {
                eprintln!("> Thinking");
                eprintln!(
                    "> Send (provider={}, model={}, messages={})",
                    provider, model, messages_count
                );
            }
            // LLM 响应事件：输出成功状态和耗时
            ObserverEvent::LlmResponse { duration, success, .. } => {
                // 将 Duration 转换为毫秒，如果溢出则使用 u64::MAX
                let ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
                eprintln!("< Receive (success={success}, duration_ms={ms})");
            }
            // 工具调用开始事件：输出工具名称
            ObserverEvent::ToolCallStart { tool } => {
                eprintln!("> Tool {tool}");
            }
            // 工具调用完成事件：输出工具名称、成功状态和耗时
            ObserverEvent::ToolCall { tool, duration, success } => {
                // 将 Duration 转换为毫秒，如果溢出则使用 u64::MAX
                let ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
                eprintln!("< Tool {tool} (success={success}, duration_ms={ms})");
            }
            // 轮次完成事件：输出完成标记
            ObserverEvent::TurnComplete => {
                eprintln!("< Complete");
            }
            // 其他事件类型：忽略不输出
            _ => {}
        }
    }

    /// 记录观察器指标
    ///
    /// 此实现为空操作，详细观察器不处理指标数据。
    /// 指标记录由专门的指标观察器实现（如 Prometheus 观察器）处理。
    ///
    /// # 参数
    ///
    /// - `_metric`：要记录的指标（被忽略）
    ///
    /// # 性能说明
    ///
    /// 使用 `#[inline(always)]` 标记以鼓励编译器内联，
    /// 确保此空操作不会产生运行时开销。
    #[inline(always)]
    fn record_metric(&self, _metric: &ObserverMetric) {}

    /// 返回观察器的名称标识
    ///
    /// # 返回值
    ///
    /// 返回字符串 `"verbose"`，用于在日志和配置中标识此观察器类型。
    fn name(&self) -> &str {
        "verbose"
    }

    /// 将观察器转换为 `Any` 类型引用
    ///
    /// 允许运行时类型检查和向下转型，用于需要动态类型操作的场景。
    ///
    /// # 返回值
    ///
    /// 返回 `&dyn Any` 引用，支持类型检查和安全的向下转型。
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests;

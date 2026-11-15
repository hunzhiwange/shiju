//! 无操作观测器实现。
//!
//! 该模块提供默认拒绝式的观测后端：调用方可以统一依赖 `Observer` trait，
//! 但在关闭观测能力或配置无效时不会产生指标、日志或外部网络副作用。

use observer_types::{Observer, ObserverEvent, ObserverMetric};
use std::any::Any;

/// 丢弃所有观测事件和指标的后端。
///
/// 该类型没有内部状态，适合作为禁用观测或功能不可用时的安全回退。
pub struct NoopObserver;

impl Observer for NoopObserver {
    #[inline(always)]
    fn record_event(&self, _event: &ObserverEvent) {}

    #[inline(always)]
    fn record_metric(&self, _metric: &ObserverMetric) {}

    fn name(&self) -> &str {
        "noop"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests;

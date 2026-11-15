use observer_types::{Observer, ObserverEvent, ObserverMetric};

/// 空操作观察器
pub struct NoopObserver;

impl Observer for NoopObserver {
    fn record_event(&self, _event: &ObserverEvent) {}

    fn record_metric(&self, _metric: &ObserverMetric) {}

    fn name(&self) -> &str {
        "noop"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}


#[cfg(test)]
#[path = "support_tests.rs"]
mod support_tests;

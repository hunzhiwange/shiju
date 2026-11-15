//! 可观测性 traits 的单元测试模块
//!
//! 本模块包含对 `Observer` trait 及其相关类型（`ObserverEvent`、`ObserverMetric`）
//! 的测试用例。主要验证：
//! - Observer 的事件和指标记录功能
//! - 默认方法（flush、as_any）的正确行为
//! - 事件和指标类型的克隆能力
//!
//! # 测试策略
//!
//! 使用一个简单的 `DummyObserver` 实现来验证 trait 契约，
//! 通过互斥锁保护的计数器来跟踪调用次数。

use super::*;
use parking_lot::Mutex;
use std::time::Duration;

/// 虚拟观察者实现，用于测试目的
///
/// 该结构体提供了一个最小化的 `Observer` trait 实现，
/// 通过内部计数器跟踪 `record_event` 和 `record_metric` 的调用次数。
/// 使用 `Mutex` 确保线程安全的计数操作。
#[derive(Default)]
struct DummyObserver {
    /// 已记录的事件数量
    events: Mutex<u64>,
    /// 已记录的指标数量
    metrics: Mutex<u64>,
}

/// 为 DummyObserver 实现 Observer trait
///
/// 该实现仅统计调用次数，不进行实际的事件或指标处理，
/// 适用于验证 trait 方法是否被正确调用。
impl Observer for DummyObserver {
    /// 记录一个观察事件
    ///
    /// 该方法每次被调用时，将事件计数器加 1。
    /// 实际的事件内容被忽略，仅用于计数。
    ///
    /// # 参数
    ///
    /// * `_event` - 要记录的观察事件（在此实现中被忽略）
    fn record_event(&self, _event: &ObserverEvent) {
        let mut guard = self.events.lock();
        *guard += 1;
    }

    /// 记录一个观察指标
    ///
    /// 该方法每次被调用时，将指标计数器加 1。
    /// 实际的指标内容被忽略，仅用于计数。
    ///
    /// # 参数
    ///
    /// * `_metric` - 要记录的观察指标（在此实现中被忽略）
    fn record_metric(&self, _metric: &ObserverMetric) {
        let mut guard = self.metrics.lock();
        *guard += 1;
    }

    /// 返回观察者的名称
    ///
    /// # 返回值
    ///
    /// 总是返回 `"dummy-observer"` 作为标识符
    fn name(&self) -> &str {
        "dummy-observer"
    }

    /// 将 self 转换为 `Any` trait 对象
    ///
    /// 允许在运行时进行类型检查和向下转型。
    ///
    /// # 返回值
    ///
    /// 返回 `self` 作为 `dyn Any` 引用
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// 测试 Observer 的事件和指标记录功能
///
/// 验证以下行为：
/// - `record_event` 方法正确递增事件计数器
/// - `record_metric` 方法正确递增指标计数器
/// - 不同类型的事件（HeartbeatTick、Error）都能被正确记录
/// - 计数器准确反映调用次数
#[test]
fn observer_records_events_and_metrics() {
    // 创建一个默认的 DummyObserver 实例
    let observer = DummyObserver::default();

    // 记录两个不同类型的事件
    // 第一个事件：心跳滴答
    observer.record_event(&ObserverEvent::HeartbeatTick);
    // 第二个事件：错误事件，包含组件名和错误消息
    observer
        .record_event(&ObserverEvent::Error { component: "test".into(), message: "boom".into() });

    // 记录一个指标：令牌使用量
    observer.record_metric(&ObserverMetric::TokensUsed(42));

    // 验证事件计数器为 2（记录了 2 个事件）
    assert_eq!(*observer.events.lock(), 2);
    // 验证指标计数器为 1（记录了 1 个指标）
    assert_eq!(*observer.metrics.lock(), 1);
}

/// 测试 Observer 的默认方法行为
///
/// 验证以下行为：
/// - `flush` 方法可以正常调用（无 panic）
/// - `name` 方法返回预期的观察者名称
/// - `as_any` 方法支持向下转型
#[test]
fn observer_default_flush_and_as_any_work() {
    let observer = DummyObserver::default();

    // 调用 flush 方法，验证不会 panic
    observer.flush();

    // 验证 name 方法返回正确的名称
    assert_eq!(observer.name(), "dummy-observer");

    // 验证 as_any 方法支持向下转型到具体类型
    assert!(observer.as_any().downcast_ref::<DummyObserver>().is_some());
}

/// 测试 ObserverEvent 和 ObserverMetric 的克隆能力
///
/// 验证以下行为：
/// - `ObserverEvent` 枚举可以被克隆
/// - `ObserverMetric` 枚举可以被克隆
/// - 克隆后的值与原始值具有相同的模式和内容
#[test]
fn observer_event_and_metric_are_cloneable() {
    // 创建一个包含多个字段的 ToolCall 事件
    let event = ObserverEvent::ToolCall {
        tool: "shell".into(),
        duration: Duration::from_millis(10),
        success: true,
    };

    // 创建一个请求延迟指标
    let metric = ObserverMetric::RequestLatency(Duration::from_millis(8));

    // 克隆事件和指标
    let cloned_event = event.clone();
    let cloned_metric = metric.clone();

    // 验证克隆的事件仍然匹配 ToolCall 模式
    assert!(matches!(cloned_event, ObserverEvent::ToolCall { .. }));
    // 验证克隆的指标仍然匹配 RequestLatency 模式
    assert!(matches!(cloned_metric, ObserverMetric::RequestLatency(_)));
}

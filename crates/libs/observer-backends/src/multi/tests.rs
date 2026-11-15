//! MultiObserver 单元测试模块
//!
//! 本模块提供 `MultiObserver` 的完整测试套件，验证其多观察者分发能力。
//!
//! # 测试范围
//!
//! - 基础属性测试：名称标识、空观察者容错
//! - 事件分发测试：验证事件能正确广播至所有注册观察者
//! - 指标分发测试：验证指标能正确广播至所有注册观察者
//! - 刷新分发测试：验证 flush 操作能正确传播至所有观察者
//!
//! # 测试辅助
//!
//! 使用 `CountingObserver` 作为测试替身，通过原子计数器追踪调用次数。

use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// 计数观察者测试替身
///
/// 实现简单的事件/指标/刷新计数逻辑，用于验证 MultiObserver 的分发行为。
/// 所有计数器均为原子类型，确保线程安全的计数操作。
///
/// # 字段说明
///
/// - `event_count` - 记录 `record_event` 调用次数的原子计数器
/// - `metric_count` - 记录 `record_metric` 调用次数的原子计数器
/// - `flush_count` - 记录 `flush` 调用次数的原子计数器
struct CountingObserver {
    event_count: Arc<AtomicUsize>,
    metric_count: Arc<AtomicUsize>,
    flush_count: Arc<AtomicUsize>,
}

impl CountingObserver {
    /// 创建新的计数观察者实例
    ///
    /// # 参数
    ///
    /// - `event_count` - 事件计数器的共享引用
    /// - `metric_count` - 指标计数器的共享引用
    /// - `flush_count` - 刷新计数器的共享引用
    ///
    /// # 返回值
    ///
    /// 返回配置好计数器的 `CountingObserver` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let ec = Arc::new(AtomicUsize::new(0));
    /// let mc = Arc::new(AtomicUsize::new(0));
    /// let fc = Arc::new(AtomicUsize::new(0));
    /// let observer = CountingObserver::new(ec.clone(), mc.clone(), fc.clone());
    /// ```
    fn new(
        event_count: Arc<AtomicUsize>,
        metric_count: Arc<AtomicUsize>,
        flush_count: Arc<AtomicUsize>,
    ) -> Self {
        Self { event_count, metric_count, flush_count }
    }
}

impl Observer for CountingObserver {
    /// 记录事件并递增事件计数器
    ///
    /// 使用顺序一致性内存序确保计数操作的可见性和原子性。
    fn record_event(&self, _event: &ObserverEvent) {
        self.event_count.fetch_add(1, Ordering::SeqCst);
    }

    /// 记录指标并递增指标计数器
    ///
    /// 使用顺序一致性内存序确保计数操作的可见性和原子性。
    fn record_metric(&self, _metric: &ObserverMetric) {
        self.metric_count.fetch_add(1, Ordering::SeqCst);
    }

    /// 执行刷新并递增刷新计数器
    ///
    /// 使用顺序一致性内存序确保计数操作的可见性和原子性。
    fn flush(&self) {
        self.flush_count.fetch_add(1, Ordering::SeqCst);
    }

    /// 返回观察者的标识名称
    fn name(&self) -> &str {
        "counting"
    }

    /// 返回 `self` 作为 `Any` trait 对象的引用
    ///
    /// 用于支持运行时类型检查和向下转型。
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// 测试 MultiObserver 的名称属性
///
/// 验证 MultiObserver 在没有子观察者时，仍能返回正确的标识名称 "multi"。
#[test]
fn multi_name() {
    let m = MultiObserver::new(vec![]);
    assert_eq!(m.name(), "multi");
}

/// 测试空 MultiObserver 的容错性
///
/// 验证当 MultiObserver 不包含任何子观察者时，调用 `record_event`、
/// `record_metric` 和 `flush` 不会引发 panic，确保系统健壮性。
///
/// # 测试场景
///
/// - 空观察者列表
/// - 事件记录操作
/// - 指标记录操作
/// - 刷新操作
#[test]
fn multi_empty_no_panic() {
    let m = MultiObserver::new(vec![]);
    m.record_event(&ObserverEvent::HeartbeatTick);
    m.record_metric(&ObserverMetric::TokensUsed(10));
    m.flush();
}

/// 测试事件的多播分发
///
/// 验证 MultiObserver 能将事件正确分发至所有注册的子观察者。
/// 创建两个 CountingObserver，各记录 3 次心跳事件，验证两者计数一致。
///
/// # 测试流程
///
/// 1. 创建两组独立的原子计数器
/// 2. 构造包含两个 CountingObserver 的 MultiObserver
/// 3. 调用 3 次 `record_event` 记录心跳事件
/// 4. 验证两个观察者的事件计数器均为 3
#[test]
fn multi_fans_out_events() {
    // 第一组计数器：用于第一个观察者
    let ec1 = Arc::new(AtomicUsize::new(0));
    let mc1 = Arc::new(AtomicUsize::new(0));
    let fc1 = Arc::new(AtomicUsize::new(0));

    // 第二组计数器：用于第二个观察者
    let ec2 = Arc::new(AtomicUsize::new(0));
    let mc2 = Arc::new(AtomicUsize::new(0));
    let fc2 = Arc::new(AtomicUsize::new(0));

    // 构造包含两个观察者的 MultiObserver
    let m = MultiObserver::new(vec![
        Box::new(CountingObserver::new(ec1.clone(), mc1.clone(), fc1.clone())),
        Box::new(CountingObserver::new(ec2.clone(), mc2.clone(), fc2.clone())),
    ]);

    // 记录 3 次心跳事件
    m.record_event(&ObserverEvent::HeartbeatTick);
    m.record_event(&ObserverEvent::HeartbeatTick);
    m.record_event(&ObserverEvent::HeartbeatTick);

    // 验证两个观察者都收到了 3 次事件
    assert_eq!(ec1.load(Ordering::SeqCst), 3);
    assert_eq!(ec2.load(Ordering::SeqCst), 3);
}

/// 测试指标的多播分发
///
/// 验证 MultiObserver 能将指标正确分发至所有注册的子观察者。
/// 创建两个 CountingObserver，各记录 2 种不同指标，验证两者计数一致。
///
/// # 测试流程
///
/// 1. 创建两组独立的原子计数器
/// 2. 构造包含两个 CountingObserver 的 MultiObserver
/// 3. 调用 `record_metric` 记录令牌使用指标
/// 4. 调用 `record_metric` 记录请求延迟指标
/// 5. 验证两个观察者的指标计数器均为 2
#[test]
fn multi_fans_out_metrics() {
    // 第一组计数器：用于第一个观察者
    let ec1 = Arc::new(AtomicUsize::new(0));
    let mc1 = Arc::new(AtomicUsize::new(0));
    let fc1 = Arc::new(AtomicUsize::new(0));

    // 第二组计数器：用于第二个观察者
    let ec2 = Arc::new(AtomicUsize::new(0));
    let mc2 = Arc::new(AtomicUsize::new(0));
    let fc2 = Arc::new(AtomicUsize::new(0));

    // 构造包含两个观察者的 MultiObserver
    let m = MultiObserver::new(vec![
        Box::new(CountingObserver::new(ec1.clone(), mc1.clone(), fc1.clone())),
        Box::new(CountingObserver::new(ec2.clone(), mc2.clone(), fc2.clone())),
    ]);

    // 记录两种不同的指标
    m.record_metric(&ObserverMetric::TokensUsed(100));
    m.record_metric(&ObserverMetric::RequestLatency(Duration::from_millis(5)));

    // 验证两个观察者都收到了 2 次指标
    assert_eq!(mc1.load(Ordering::SeqCst), 2);
    assert_eq!(mc2.load(Ordering::SeqCst), 2);
}

/// 测试刷新操作的多播分发
///
/// 验证 MultiObserver 能将 flush 操作正确分发至所有注册的子观察者。
/// 创建两个 CountingObserver，共享事件和指标计数器但各自维护刷新计数器。
///
/// # 测试流程
///
/// 1. 创建共享的事件和指标计数器
/// 2. 创建两个独立的刷新计数器
/// 3. 构造包含两个 CountingObserver 的 MultiObserver
/// 4. 调用 `flush` 触发刷新操作
/// 5. 验证两个观察者的刷新计数器均为 1
#[test]
fn multi_fans_out_flush() {
    // 共享的事件和指标计数器（此测试中不使用，但需要传入构造函数）
    let ec = Arc::new(AtomicUsize::new(0));
    let mc = Arc::new(AtomicUsize::new(0));

    // 两个独立的刷新计数器
    let fc1 = Arc::new(AtomicUsize::new(0));
    let fc2 = Arc::new(AtomicUsize::new(0));

    // 构造包含两个观察者的 MultiObserver
    let m = MultiObserver::new(vec![
        Box::new(CountingObserver::new(ec.clone(), mc.clone(), fc1.clone())),
        Box::new(CountingObserver::new(ec.clone(), mc.clone(), fc2.clone())),
    ]);

    // 执行刷新操作
    m.flush();

    // 验证两个观察者都收到了 1 次刷新调用
    assert_eq!(fc1.load(Ordering::SeqCst), 1);
    assert_eq!(fc2.load(Ordering::SeqCst), 1);
}

/// 测试 `as_any` 方法的下转型能力
///
/// 验证 MultiObserver 通过 Observer trait 的 `as_any` 方法能正确下转型回具体类型。
/// 这在运行时需要操作具体观察者类型时有用（如批量子类型专属操作）。
#[test]
fn multi_as_any_downcast() {
    let m = MultiObserver::new(vec![]);
    let obs: &dyn Observer = &m;
    let downcasted = obs.as_any().downcast_ref::<MultiObserver>();
    assert!(downcasted.is_some());
    assert_eq!(downcasted.unwrap().name(), "multi");
}

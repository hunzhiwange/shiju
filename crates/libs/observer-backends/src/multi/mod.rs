//! 多观察者组合器模块
//!
//! 本模块提供 [`MultiObserver`] 实现，用于将单个观察者请求广播到多个底层观察者实例。
//! 这是组合模式的典型应用，允许系统同时使用多个观测后端（例如日志、指标、追踪等）。
//!
//! # 主要功能
//!
//! - **多路广播**：将事件和指标同时记录到所有注册的观察者
//! - **统一接口**：实现 `Observer` trait，可透明替换单个观察者
//! - **批量刷新**：一次性刷新所有底层观察者的缓冲区
//!
//! # 使用示例
//!
//! ```ignore
//! use observer_backends::{MultiObserver, LogObserver, NoopObserver};
//! use observer_types::{Observer, ObserverEvent};
//!
//! // 创建多个观察者实例
//! let log_obs = Box::new(LogObserver::new());
//! let noop_obs = Box::new(NoopObserver);
//!
//! // 组合为多观察者
//! let multi = MultiObserver::new(vec![log_obs, noop_obs]);
//!
//! // 广播事件到所有观察者
//! multi.record_event(&ObserverEvent::HeartbeatTick);
//! ```

use observer_types::{Observer, ObserverEvent, ObserverMetric};
use std::any::Any;

/// 多观察者组合器
///
/// [`MultiObserver`] 是一个组合模式的观察者实现，内部持有一组 `Observer` trait 对象，
/// 将所有观测操作（事件记录、指标记录、刷新）转发到每个底层观察者。
///
/// # 特性
///
/// - **无优先级**：观察者按注册顺序依次调用，无优先级或并行执行
/// - **容错性**：底层观察者的错误不会传播，调用方无需处理单个观察者的失败
/// - **零开销抽象**：仅在遍历时产生少量间接调用开销
///
/// # 线程安全
///
/// 所有底层观察者的方法调用必须保证线程安全（`Observer` trait 要求 `Sync`）。
pub struct MultiObserver {
    /// 注册的观察者列表
    ///
    /// 每个观察者都是 `Box<dyn Observer>` 形式，支持动态分发。
    /// 事件和指标会按顺序广播到此列表中的所有观察者。
    observers: Vec<Box<dyn Observer>>,
}

impl MultiObserver {
    /// 创建新的多观察者实例
    ///
    /// # 参数
    ///
    /// - `observers`：要注册的观察者列表，按传入顺序依次调用
    ///
    /// # 返回值
    ///
    /// 返回配置好的 [`MultiObserver`] 实例，已包含所有传入的观察者
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let observers: Vec<Box<dyn Observer>> = vec![
    ///     Box::new(LogObserver::new()),
    ///     Box::new(NoopObserver),
    /// ];
    /// let multi = MultiObserver::new(observers);
    /// ```
    pub fn new(observers: Vec<Box<dyn Observer>>) -> Self {
        Self { observers }
    }
}

/// 实现 `Observer` trait 以支持多观察者广播
///
/// 此实现将所有观测操作转发到内部注册的每个观察者，实现透明的多路广播。
impl Observer for MultiObserver {
    /// 记录观测事件
    ///
    /// 将事件广播到所有注册的观察者。每个观察者独立处理事件，
    /// 观察者之间的失败互不影响。
    ///
    /// # 参数
    ///
    /// - `event`：要记录的观测事件，包含事件类型、时间戳和相关数据
    ///
    /// # 广播行为
    ///
    /// - 按注册顺序依次调用每个观察者的 `record_event`
    /// - 调用是阻塞的，上一个观察者完成后才调用下一个
    fn record_event(&self, event: &ObserverEvent) {
        // 遍历所有观察者并记录事件
        for obs in &self.observers {
            obs.record_event(event);
        }
    }

    /// 记录观测指标
    ///
    /// 将指标广播到所有注册的观察者。适用于计数器、计量器、直方图等指标类型。
    ///
    /// # 参数
    ///
    /// - `metric`：要记录的观测指标，包含指标名称、值和标签
    ///
    /// # 广播行为
    ///
    /// - 按注册顺序依次调用每个观察者的 `record_metric`
    /// - 适用于需要同时收集到多个后端的场景（如本地日志 + 远程监控系统）
    fn record_metric(&self, metric: &ObserverMetric) {
        // 遍历所有观察者并记录指标
        for obs in &self.observers {
            obs.record_metric(metric);
        }
    }

    /// 刷新所有观察者的缓冲区
    ///
    /// 强制将所有观察者的内部缓冲区内容写入目标后端。
    /// 这通常在优雅关闭或检查点时调用，以确保所有数据已持久化。
    ///
    /// # 刷新行为
    ///
    /// - 按注册顺序依次调用每个观察者的 `flush`
    /// - 即使某个观察者刷新失败，仍会继续刷新其他观察者
    /// - 调用方应确保在程序退出前调用此方法
    fn flush(&self) {
        // 遍历所有观察者并刷新缓冲区
        for obs in &self.observers {
            obs.flush();
        }
    }

    /// 返回观察者名称
    ///
    /// # 返回值
    ///
    /// 总是返回 `"multi"`，标识这是一个多观察者组合器
    fn name(&self) -> &str {
        "multi"
    }

    /// 返回类型擦除的自引用
    ///
    /// 用于运行时类型检查和向下转型。允许调用方在需要时访问具体类型。
    ///
    /// # 返回值
    ///
    /// 返回 `&dyn Any`，可通过 `downcast_ref` 转换回具体类型
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// 单元测试模块
#[cfg(test)]
mod tests;

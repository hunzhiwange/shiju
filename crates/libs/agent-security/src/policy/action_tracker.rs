//! 动作追踪器模块
//!
//! 提供基于时间窗口的动作频率追踪功能，用于监控和限制特定操作的执行频率。
//! 主要应用于安全策略中的速率限制（Rate Limiting）场景，例如：
//! - API 请求频率限制
//! - 工具调用频率限制
//! - 用户行为频率监控
//!
//! # 核心特性
//!
//! - **线程安全**：使用 `Mutex` 保证并发访问的安全性
//! - **滑动窗口**：基于时间的滑动窗口算法，自动清理过期记录
//! - **低开销**：仅在记录和计数时进行必要的锁操作
//!
//! # 示例
//!
//! ```rust
//! use vibe_agent::security::policy::ActionTracker;
//!
//! let tracker = ActionTracker::new();
//! let count = tracker.record(); // 记录一次动作并返回当前窗口内的总数
//! println!("当前窗口内动作数: {}", count);
//! ```

use parking_lot::Mutex;
use std::time::Instant;

/// 动作追踪器
///
/// 追踪指定时间窗口内发生的动作次数。使用滑动窗口算法，
/// 自动清理超过时间窗口的旧记录，保持数据的新鲜度。
///
/// # 内部结构
///
/// - `actions`：使用 `Mutex` 保护的 `Vec<Instant>` 集合
///   - 存储每个动作发生的精确时间点
///   - 每次访问时自动清理过期记录（默认窗口为 1 小时）
///
/// # 线程安全性
///
/// 所有公开方法都是线程安全的，可以在多线程环境中安全调用。
/// 使用 `parking_lot::Mutex` 提供高性能的互斥锁实现。
///
/// # 时间窗口
///
/// 当前实现使用固定的时间窗口（3600 秒 = 1 小时），
/// 在 `record()` 和 `count()` 方法中自动清理超出窗口的记录。
#[derive(Debug)]
pub struct ActionTracker {
    /// 动作时间戳集合
    ///
    /// 存储每个动作发生的精确时间点（`Instant`）。
    /// 使用 `Mutex` 包装以支持并发访问。
    actions: Mutex<Vec<Instant>>,
}

impl ActionTracker {
    /// 创建一个新的动作追踪器实例
    ///
    /// 初始化一个空的追踪器，不包含任何历史动作记录。
    ///
    /// # 返回值
    ///
    /// 返回一个空的 `ActionTracker` 实例。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use vibe_agent::security::policy::ActionTracker;
    ///
    /// let tracker = ActionTracker::new();
    /// assert_eq!(tracker.count(), 0);
    /// ```
    pub fn new() -> Self {
        Self { actions: Mutex::new(Vec::new()) }
    }

    /// 记录一次新动作并返回当前时间窗口内的动作总数
    ///
    /// 执行以下操作：
    /// 1. 清理超出时间窗口的旧记录（滑动窗口清理）
    /// 2. 记录当前动作的时间戳
    /// 3. 返回清理后的动作总数
    ///
    /// # 返回值
    ///
    /// 返回 `usize`，表示当前时间窗口内（最近 1 小时）的动作总数。
    /// 该数值包含刚刚记录的本次动作。
    ///
    /// # 时间复杂度
    ///
    /// - 平均情况：O(n)，其中 n 为当前存储的时间戳数量
    /// - 清理过期记录需要遍历整个向量
    ///
    /// # 线程安全性
    ///
    /// 该方法在执行期间持有互斥锁，确保并发调用的安全性。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use vibe_agent::security::policy::ActionTracker;
    ///
    /// let tracker = ActionTracker::new();
    /// assert_eq!(tracker.record(), 1); // 第一次记录
    /// assert_eq!(tracker.record(), 2); // 第二次记录
    /// ```
    pub fn record(&self) -> usize {
        let mut actions = self.actions.lock();

        // 计算时间窗口的截止点（当前时间 - 3600 秒）
        // 使用 checked_sub 避免时间下溢，如果发生下溢则使用当前时间作为截止点
        let cutoff = Instant::now()
            .checked_sub(std::time::Duration::from_secs(3600))
            .unwrap_or_else(Instant::now);

        // 清理超出时间窗口的旧记录（滑动窗口核心逻辑）
        actions.retain(|t| *t > cutoff);

        // 记录当前动作的时间戳
        actions.push(Instant::now());

        // 返回当前窗口内的动作总数
        actions.len()
    }

    /// 获取当前时间窗口内的动作总数（不记录新动作）
    ///
    /// 执行以下操作：
    /// 1. 清理超出时间窗口的旧记录（滑动窗口清理）
    /// 2. 返回清理后的动作总数
    ///
    /// # 返回值
    ///
    /// 返回 `usize`，表示当前时间窗口内（最近 1 小时）的动作总数。
    /// 该数值不包含本次调用（因为只是计数，不记录）。
    ///
    /// # 时间复杂度
    ///
    /// - 平均情况：O(n)，其中 n 为当前存储的时间戳数量
    /// - 清理过期记录需要遍历整个向量
    ///
    /// # 线程安全性
    ///
    /// 该方法在执行期间持有互斥锁，确保并发调用的安全性。
    /// 即使是只读操作也需要锁，因为需要执行清理逻辑。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use vibe_agent::security::policy::ActionTracker;
    ///
    /// let tracker = ActionTracker::new();
    /// assert_eq!(tracker.count(), 0); // 尚未记录任何动作
    /// tracker.record();
    /// assert_eq!(tracker.count(), 1); // 已记录 1 次动作
    /// ```
    pub fn count(&self) -> usize {
        let mut actions = self.actions.lock();

        // 计算时间窗口的截止点（当前时间 - 3600 秒）
        // 使用 checked_sub 避免时间下溢，如果发生下溢则使用当前时间作为截止点
        let cutoff = Instant::now()
            .checked_sub(std::time::Duration::from_secs(3600))
            .unwrap_or_else(Instant::now);

        // 清理超出时间窗口的旧记录（滑动窗口核心逻辑）
        actions.retain(|t| *t > cutoff);

        // 返回当前窗口内的动作总数
        actions.len()
    }
}

/// `ActionTracker` 的克隆实现
///
/// 创建一个新的 `ActionTracker` 实例，包含当前追踪器中所有时间戳的副本。
/// 这是一个深拷贝操作，两个追踪器互不影响。
impl Clone for ActionTracker {
    /// 克隆当前追踪器
    ///
    /// # 返回值
    ///
    /// 返回一个新的 `ActionTracker` 实例，包含当前所有时间戳的副本。
    ///
    /// # 线程安全性
    ///
    /// 克隆过程中会锁定源追踪器的互斥锁，确保数据一致性。
    fn clone(&self) -> Self {
        let actions = self.actions.lock();
        Self { actions: Mutex::new(actions.clone()) }
    }
}

/// `ActionTracker` 的默认值实现
///
/// 提供与 `new()` 相同的初始化行为，支持使用 `Default` trait。
/// 这使得 `ActionTracker` 可以用于需要 `Default` 约束的泛型上下文。
impl Default for ActionTracker {
    /// 返回默认的 `ActionTracker` 实例
    ///
    /// 等价于调用 `ActionTracker::new()`。
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "action_tracker_tests.rs"]
mod action_tracker_tests;

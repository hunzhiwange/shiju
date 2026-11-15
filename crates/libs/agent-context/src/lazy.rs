//! 惰性求值工具模块
//!
//! 本模块提供线程安全的惰性求值实现，允许延迟值的初始化直到第一次访问时才进行计算。
//! 主要用于优化性能，避免不必要的计算开销，特别适用于初始化成本高昂但可能不会使用的场景。
//!
//! # 核心特性
//!
//! - **延迟初始化**：值在第一次调用 `get()` 时才被计算
//! - **线程安全**：使用 `Mutex` 保护内部状态，支持多线程并发访问
//! - **可重置**：支持通过 `reset()` 方法清除缓存值，允许重新计算
//! - **共享所有权**：返回 `Arc<T>`，允许多个所有者共享计算结果

use std::sync::{Arc, Mutex};

/// 内部状态结构体，用于跟踪惰性值的加载状态
///
/// # 字段说明
///
/// - `loaded`: 标记值是否已经被初始化
/// - `value`: 存储已初始化的值，使用 `Arc` 包装以支持共享所有权
struct State<T> {
    loaded: bool,
    value: Option<Arc<T>>,
}

/// 线程安全的惰性求值包装器
///
/// `Lazy<T>` 包装一个初始化函数，延迟值的计算直到第一次需要时才执行。
/// 一旦初始化完成，后续的 `get()` 调用将直接返回缓存的值，避免重复计算。
///
/// # 线程安全性
///
/// 内部使用 `Mutex` 保护状态，确保在多线程环境下的安全访问。
/// 如果某个线程在持有锁时 panic，后续访问将 panic（称为"锁中毒"）。
///
/// # 示例
///
/// ```rust,ignore
/// use agent_context::lazy::Lazy;
///
/// // 创建一个惰性求值的昂贵计算
/// let lazy_value = Lazy::new(|| {
///     println!("执行昂贵计算...");
///     42
/// });
///
/// // 第一次调用会执行计算
/// let v1 = lazy_value.get();
/// println!("值: {}", *v1); // 输出: 执行昂贵计算... \n 值: 42
///
/// // 后续调用直接返回缓存值
/// let v2 = lazy_value.get();
/// println!("值: {}", *v2); // 输出: 值: 42（不会再次执行计算）
/// ```
///
/// # 类型参数
///
/// - `T`: 惰性求值结果的类型
pub struct Lazy<T> {
    /// 初始化函数，在被调用时计算值
    f: Box<dyn Fn() -> T + Send + Sync>,
    /// 受互斥锁保护的内部状态
    state: Mutex<State<T>>,
}

impl<T> Lazy<T> {
    /// 创建一个新的惰性求值包装器
    ///
    /// 接受一个闭包作为初始化函数，该函数将在第一次调用 `get()` 时执行。
    /// 函数必须满足 `Send + Sync` trait 以支持跨线程传递。
    ///
    /// # 参数
    ///
    /// - `f`: 初始化函数，返回类型为 `T` 的值
    ///
    /// # 返回值
    ///
    /// 返回一个 `Lazy<T>` 实例，其内部状态初始化为未加载状态
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let lazy_config = Lazy::new(|| {
    ///     // 模拟从文件加载配置
    ///     "配置内容".to_string()
    /// });
    /// ```
    pub fn new(f: impl Fn() -> T + Send + Sync + 'static) -> Self {
        Self { f: Box::new(f), state: Mutex::new(State { loaded: false, value: None }) }
    }

    /// 获取惰性求值的值
    ///
    /// 如果值尚未初始化，此方法将调用初始化函数进行计算并缓存结果。
    /// 如果值已经初始化，直接返回缓存的值而不重新计算。
    ///
    /// # 返回值
    ///
    /// 返回包装在 `Arc` 中的值，允许多个所有者共享同一个实例。
    ///
    /// # Panic
    ///
    /// 如果另一个线程在持有内部锁时 panic（锁中毒），此方法将 panic。
    /// 如果内部状态不一致（已标记为加载但值不存在），此方法也会 panic。
    ///
    /// # 线程安全性
    ///
    /// 此方法使用互斥锁确保在多线程环境下的安全访问。
    /// 同一时刻只有一个线程能执行初始化逻辑。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let lazy_data = Lazy::new(|| vec![1, 2, 3]);
    ///
    /// // 首次访问，执行初始化
    /// let data = lazy_data.get();
    /// assert_eq!(*data, vec![1, 2, 3]);
    ///
    /// // 再次访问，返回缓存的值
    /// let data2 = lazy_data.get();
    /// assert!(Arc::ptr_eq(&data, &data2)); // 指向同一实例
    /// ```
    pub fn get(&self) -> Arc<T> {
        // 获取互斥锁以保护状态访问
        let mut state = self.state.lock().expect("lazy mutex poisoned");

        // 如果值已经加载，直接返回缓存的值
        if state.loaded {
            // 值应该存在，因为 loaded 为 true
            return state.value.as_ref().expect("lazy loaded without value").clone();
        }

        // 值尚未加载，执行初始化
        // 标记为已加载
        state.loaded = true;

        // 调用初始化函数计算值
        let v = Arc::new((self.f)());

        // 缓存计算结果
        state.value = Some(v.clone());

        v
    }

    /// 重置惰性值，清除缓存的值
    ///
    /// 调用此方法后，下一次 `get()` 调用将重新执行初始化函数。
    /// 这对于需要重新加载或刷新数据的场景非常有用。
    ///
    /// # Panic
    ///
    /// 如果另一个线程在持有内部锁时 panic（锁中毒），此方法将 panic。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let mut counter = 0;
    /// let lazy_value = Lazy::new(|| {
    ///     counter += 1;
    ///     counter
    /// });
    ///
    /// let v1 = lazy_value.get(); // counter = 1
    /// assert_eq!(*v1, 1);
    ///
    /// lazy_value.reset(); // 清除缓存
    ///
    /// let v2 = lazy_value.get(); // 重新计算，counter = 2
    /// assert_eq!(*v2, 2);
    /// ```
    pub fn reset(&self) {
        // 获取互斥锁以保护状态修改
        let mut state = self.state.lock().expect("lazy mutex poisoned");

        // 重置状态，允许下次 get() 重新初始化
        state.loaded = false;
        state.value = None;
    }
}

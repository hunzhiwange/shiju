//! 延迟执行工具模块
//!
//! 本模块提供类似 Go 语言 `defer` 或 C++ RAII 模式的延迟执行机制。
//! 允许在作用域结束时自动执行清理代码，确保资源正确释放。
//!
//! # 核心概念
//!
//! - **Defer**：包装一个闭包，在离开作用域时（Drop 时）自动执行
//! - **disarm**：取消延迟执行，阻止闭包被调用
//!
//! # 使用场景
//!
//! - 资源清理（文件句柄、锁释放、连接关闭）
//! - 状态恢复（临时修改后自动还原）
//! - 错误处理后的收尾工作
//! - 日志/指标记录
//!
//! # 示例
//!
//! ```rust
//! use agent_context::defer::defer;
//!
//! fn process_file() -> Result<(), std::io::Error> {
//!     let file = std::fs::File::create("temp.txt")?;
//!     // 离开作用域时自动清理
//!     let _guard = defer(|| {
//!         let _ = std::fs::remove_file("temp.txt");
//!     });
//!
//!     // 处理文件...
//!     Ok(())
//! } // _guard 离开作用域，自动删除临时文件
//! ```

/// 延迟执行包装器
///
/// 在离开作用域时自动执行被包装的闭包。
/// 基于 Rust 的 Drop trait 实现，确保即使发生 panic 也会执行清理代码。
///
/// # 类型参数
///
/// * `F` - 实现 `FnOnce()` 的闭包类型，无参数无返回值
///
/// # 示例
///
/// ## 基本用法
///
/// ```rust
/// use agent_context::defer::Defer;
///
/// {
///     let _guard = Defer::new(|| {
///         println!("作用域结束，执行清理");
///     });
///     println!("正在处理...");
/// } // 此处打印 "作用域结束，执行清理"
/// ```
///
/// ## 配合锁使用
///
/// ```rust
/// use std::sync::Mutex;
/// use agent_context::defer::Defer;
///
/// let lock = Mutex::new(0);
/// let guard = lock.lock().unwrap();
/// let _defer = Defer::new(|| {
///     // 确保在离开作用域时打印日志
///     println!("锁已释放");
/// });
/// // guard 在此处释放
/// ```
///
/// ## 条件取消执行
///
/// ```rust
/// use agent_context::defer::Defer;
///
/// fn process(success: bool) {
///     let guard = Defer::new(|| {
///         println!("执行回滚");
///     });
///
///     if success {
///         // 成功时取消回滚
///         guard.disarm();
///     }
///     // 失败时 guard 离开作用域，自动执行回滚
/// }
/// ```
pub struct Defer<F: FnOnce()> {
    /// 待执行的闭包
    ///
    /// 使用 `Option` 包装以支持 `disarm` 操作：
    /// - `Some(f)` - 闭包待执行
    /// - `None` - 已被取消（disarmed）
    f: Option<F>,
}

impl<F: FnOnce()> Defer<F> {
    /// 创建新的延迟执行包装器
    ///
    /// 传入的闭包将在 `Defer` 实例被 drop 时自动执行。
    ///
    /// # 参数
    ///
    /// * `f` - 要延迟执行的闭包，必须实现 `FnOnce()`
    ///
    /// # 返回值
    ///
    /// 返回包装了该闭包的 `Defer` 实例
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agent_context::defer::Defer;
    ///
    /// let _cleanup = Defer::new(|| {
    ///     // 清理代码
    /// });
    /// ```
    pub fn new(f: F) -> Self {
        Self { f: Some(f) }
    }

    /// 取消延迟执行
    ///
    /// 调用此方法后，闭包将不会在 drop 时执行。
    /// 适用于操作成功后不需要执行清理/回滚代码的场景。
    ///
    /// # 消费语义
    ///
    /// 此方法消费 `self`，调用后 `Defer` 实例不能再使用。
    /// 这是故意的设计，防止在 disarm 后仍然期望清理执行。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agent_context::defer::Defer;
    ///
    /// fn transfer_money() -> Result<(), ()> {
    ///     // 设置回滚
    ///     let rollback = Defer::new(|| {
    ///         println!("转账失败，执行回滚");
    ///     });
    ///
    ///     // 执行转账...
    ///     let success = true; // 假设成功
    ///
    ///     if success {
    ///         rollback.disarm(); // 成功则取消回滚
    ///         Ok(())
    ///     } else {
    ///         Err(()) // 失败则自动执行回滚
    ///     }
    /// }
    /// ```
    pub fn disarm(mut self) {
        // 取出并丢弃闭包，阻止其在 drop 时执行
        self.f.take();
    }
}

/// 实现 Drop trait 以支持自动清理
///
/// 当 `Defer` 实例离开作用域时，如果闭包尚未被 disarm，
/// 则自动执行该闭包。
impl<F: FnOnce()> Drop for Defer<F> {
    /// 执行析构逻辑
    ///
    /// 如果闭包存在（未被 disarm），则取出并执行它。
    /// 使用 `take()` 确保闭包只执行一次，避免 panic 时的重复执行。
    fn drop(&mut self) {
        // 检查闭包是否存在并取出（避免执行后再次 drop 时重复调用）
        if let Some(f) = self.f.take() {
            // 执行延迟的闭包
            f();
        }
        // 如果 f 为 None（已被 disarm），则什么都不做
    }
}

/// 创建延迟执行包装器的便捷函数
///
/// 这是 `Defer::new()` 的简写形式，提供更简洁的语法。
///
/// # 参数
///
/// * `f` - 要延迟执行的闭包，必须实现 `FnOnce()`
///
/// # 返回值
///
/// 返回包装了该闭包的 `Defer` 实例
///
/// # 示例
///
/// ```rust
/// use agent_context::defer::defer;
///
/// fn example() {
///     let _guard = defer(|| {
///         println!("函数退出时执行");
///     });
///
///     // 函数逻辑...
///
/// } // _guard 离开作用域，自动执行闭包
/// ```
///
/// ## 与显式锁释放对比
///
/// ```rust
/// use agent_context::defer::defer;
/// use std::sync::Mutex;
///
/// fn with_lock() {
///     let mutex = Mutex::new(vec![]);
///     let mut data = mutex.lock().unwrap();
///
///     // 确保在修改后记录日志
///     let _log = defer(|| {
///         println!("数据已修改");
///     });
///
///     data.push(42);
/// } // 先记录日志，再释放锁
/// ```
pub fn defer<F: FnOnce()>(f: F) -> Defer<F> {
    Defer::new(f)
}

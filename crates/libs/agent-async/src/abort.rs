//! 中止控制工具模块
//!
//! 本模块提供了类似于 JavaScript 的 AbortController/AbortSignal 模式的中止控制机制。
//! 它允许在异步环境中优雅地取消正在进行的操作。
//!
//! # 核心组件
//!
//! - [`AbortController`]：中止控制器，用于触发中止信号
//! - [`AbortSignal`]：中止信号，用于监听和传播中止状态
//! - [`abort_after`]：延迟中止函数，在指定时间后自动触发中止
//! - [`abort_after_any`]：组合中止函数，在超时或任一信号触发时中止
//!
//! # 使用示例
//!
//! ```rust,ignore
//! // 创建中止控制器和信号
//! let (controller, signal) = AbortController::new();
//!
//! // 在另一个任务中监听中止信号
//! tokio::spawn(async move {
//!     signal.clone().cancelled().await;
//!     println!("操作已取消");
//! });
//!
//! // 触发中止
//! controller.abort();
//! ```

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

/// 中止控制器
///
/// 用于主动触发中止信号。控制器可以克隆，克隆后的实例共享同一个中止状态。
/// 当调用 [`abort`](AbortController::abort) 方法时，所有关联的中止信号都会被激活。
///
/// # 示例
///
/// ```rust,ignore
/// let (controller, signal) = AbortController::new();
///
/// // 触发中止
/// controller.abort();
///
/// // 检查信号是否已被中止
/// assert!(signal.aborted());
/// ```
#[derive(Clone)]
pub struct AbortController {
    /// 用于发送中止状态的通道发送端
    /// 值为 `true` 表示已中止，`false` 表示未中止
    tx: watch::Sender<bool>,
}

/// 中止信号
///
/// 用于监听中止状态的变化。信号可以被克隆以在多个地方同时监听。
/// 信号提供了同步检查（[`aborted`](AbortSignal::aborted)）和异步等待（[`cancelled`](AbortSignal::cancelled)）两种方式。
///
/// # 示例
///
/// ```rust,ignore
/// let (controller, mut signal) = AbortController::new();
///
/// // 异步等待中止信号
/// tokio::spawn(async move {
///     signal.cancelled().await;
///     println!("收到中止信号");
/// });
/// ```
#[derive(Clone)]
pub struct AbortSignal {
    /// 用于接收中止状态的通道接收端
    rx: watch::Receiver<bool>,
}

impl AbortController {
    /// 创建新的中止控制器和关联的中止信号
    ///
    /// 返回一个元组，包含控制器和信号。控制器用于触发中止，
    /// 信号用于监听中止状态。
    ///
    /// # 返回值
    ///
    /// 返回 `(AbortController, AbortSignal)` 元组：
    /// - 第一个元素是中止控制器
    /// - 第二个元素是关联的中止信号
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let (controller, signal) = AbortController::new();
    /// assert!(!signal.aborted());
    /// ```
    pub fn new() -> (Self, AbortSignal) {
        let (tx, rx) = watch::channel(false);
        (Self { tx }, AbortSignal { rx })
    }

    /// 触发中止信号
    ///
    /// 将中止状态设置为 `true`，通知所有监听该信号的接收者。
    /// 此方法是幂等的，多次调用不会有副作用。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let (controller, signal) = AbortController::new();
    /// controller.abort();
    /// assert!(signal.aborted());
    /// ```
    pub fn abort(&self) {
        let _ = self.tx.send(true);
    }

    /// 创建新的中止信号订阅
    ///
    /// 返回一个新的中止信号实例，该实例订阅了当前控制器的中止状态。
    /// 这允许多个独立的监听者同时监听同一个中止源。
    ///
    /// # 返回值
    ///
    /// 返回一个新的 `AbortSignal` 实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let (controller, signal1) = AbortController::new();
    /// let signal2 = controller.signal();
    ///
    /// controller.abort();
    /// assert!(signal1.aborted());
    /// assert!(signal2.aborted());
    /// ```
    pub fn signal(&self) -> AbortSignal {
        AbortSignal { rx: self.tx.subscribe() }
    }
}

impl AbortSignal {
    /// 同步检查是否已被中止
    ///
    /// 立即返回当前的中止状态，不会阻塞。
    ///
    /// # 返回值
    ///
    /// - `true`：已中止
    /// - `false`：未中止
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let (controller, signal) = AbortController::new();
    /// assert!(!signal.aborted());
    ///
    /// controller.abort();
    /// assert!(signal.aborted());
    /// ```
    pub fn aborted(&self) -> bool {
        *self.rx.borrow()
    }

    /// 异步等待中止信号
    ///
    /// 如果已经中止，立即返回；否则等待直到收到中止信号。
    /// 此方法用于在异步代码中等待取消通知。
    ///
    /// # 取消安全性
    ///
    /// 当信号被中止时，此方法会立即返回。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let (controller, mut signal) = AbortController::new();
    ///
    /// tokio::spawn(async move {
    ///     signal.cancelled().await;
    ///     println!("操作已被取消");
    /// });
    ///
    /// // 稍后触发中止
    /// controller.abort();
    /// ```
    pub async fn cancelled(&mut self) {
        // 如果已经中止，立即返回
        if *self.rx.borrow() {
            return;
        }
        // 等待状态变化，直到收到中止信号
        while self.rx.changed().await.is_ok() {
            if *self.rx.borrow() {
                break;
            }
        }
    }

    /// 组合多个中止信号为单个信号
    ///
    /// 创建一个新的中止信号，当输入信号中的任何一个被中止时，
    /// 新信号也会被中止。这类似于 JavaScript 的 `AbortSignal.any()`。
    ///
    /// # 参数
    ///
    /// - `signals`：要组合的中止信号迭代器
    ///
    /// # 返回值
    ///
    /// 返回一个新的 `AbortSignal`，当任一输入信号被中止时触发
    ///
    /// # 实现细节
    ///
    /// 为每个输入信号创建一个异步任务，监听其状态变化。
    /// 当任一信号被中止时，新信号也会被设置中止状态。
    ///
    /// # 平台兼容性
    ///
    /// - 非 WASM 平台：使用 `tokio::spawn` 创建任务
    /// - WASM 平台：使用 `wasm_bindgen_futures::spawn_local` 创建任务
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let (ctrl1, signal1) = AbortController::new();
    /// let (ctrl2, signal2) = AbortController::new();
    ///
    /// let combined = AbortSignal::any([signal1, signal2]);
    ///
    /// // 任一控制器中止，组合信号也会中止
    /// ctrl1.abort();
    /// assert!(combined.aborted());
    /// ```
    pub fn any(signals: impl IntoIterator<Item = AbortSignal>) -> AbortSignal {
        let (tx, rx) = watch::channel(false);
        let tx = Arc::new(tx);

        // 为每个输入信号创建监听任务
        for mut signal in signals {
            let tx = tx.clone();
            let future = async move {
                // 等待该信号被中止
                signal.cancelled().await;
                // 当信号被中止时，触发组合信号的中止
                let _ = tx.send(true);
            };

            // 根据目标平台选择不同的任务调度方式
            #[cfg(not(target_arch = "wasm32"))]
            tokio::spawn(future);

            #[cfg(target_arch = "wasm32")]
            wasm_bindgen_futures::spawn_local(future);
        }
        AbortSignal { rx }
    }
}

/// 延迟中止句柄
///
/// 由 [`abort_after`] 函数返回，包含中止控制器、信号和定时任务句柄。
/// 可以通过 [`clear_timeout`](AbortAfter::clear_timeout) 方法取消定时中止。
///
/// # 字段
///
/// - `controller`：中止控制器，可用于手动触发中止
/// - `signal`：中止信号，用于监听中止状态
/// - `handle`（非 WASM）：定时任务句柄，用于取消任务
/// - `cancelled`（WASM）：取消标志，用于阻止中止触发
pub struct AbortAfter {
    /// 中止控制器，可用于手动触发提前中止
    pub controller: AbortController,
    /// 中止信号，用于监听中止状态
    pub signal: AbortSignal,
    /// 定时任务句柄（非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    handle: Option<tokio::task::JoinHandle<()>>,
    /// 取消标志（WASM 平台）
    /// 当设置为 true 时，阻止超时后触发中止
    #[cfg(target_arch = "wasm32")]
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}

impl AbortAfter {
    /// 取消定时中止
    ///
    /// 取消正在运行的定时任务，阻止超时后的自动中止。
    /// 调用此方法后，只有通过 `controller.abort()` 手动触发才会中止。
    ///
    /// # 平台差异
    ///
    /// - 非 WASM 平台：中止定时任务
    /// - WASM 平台：设置取消标志，阻止中止触发
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let mut abort_after = abort_after(5000);
    ///
    /// // 取消定时中止
    /// abort_after.clear_timeout();
    ///
    /// // 现在不会在 5 秒后自动中止
    /// // 但仍可以手动触发
    /// abort_after.controller.abort();
    /// ```
    pub fn clear_timeout(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }

        #[cfg(target_arch = "wasm32")]
        self.cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// 创建延迟中止
///
/// 创建一个在指定毫秒数后自动触发的中止机制。
/// 返回的 `AbortAfter` 结构体包含控制器、信号和可取消的定时任务。
///
/// # 参数
///
/// - `ms`：延迟时间，单位为毫秒
///
/// # 返回值
///
/// 返回 `AbortAfter` 结构体，包含：
/// - `controller`：用于手动触发中止
/// - `signal`：用于监听中止状态
/// - 定时任务句柄（可通过 `clear_timeout` 取消）
///
/// # 示例
///
/// ```rust,ignore
/// // 创建 5 秒后自动中止的控制器
/// let abort_after = abort_after(5000);
///
/// // 在异步任务中使用
/// tokio::spawn(async move {
///     let mut signal = abort_after.signal;
///     signal.cancelled().await;
///     println!("操作已超时取消");
/// });
///
/// // 或者提前取消定时器
/// let mut abort_after = abort_after(5000);
/// abort_after.clear_timeout(); // 不会自动中止了
/// abort_after.controller.abort(); // 手动中止
/// ```
pub fn abort_after(ms: u64) -> AbortAfter {
    let (controller, signal) = AbortController::new();
    let controller_for_task = controller.clone();

    #[cfg(not(target_arch = "wasm32"))]
    {
        // 在非 WASM 平台，使用 tokio 任务
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(ms)).await;
            controller_for_task.abort();
        });
        AbortAfter { controller, signal, handle: Some(handle) }
    }

    #[cfg(target_arch = "wasm32")]
    {
        // 在 WASM 平台，使用原子标志来允许取消
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancelled_clone = cancelled.clone();
        wasm_bindgen_futures::spawn_local(async move {
            tokio::time::sleep(Duration::from_millis(ms)).await;
            // 只有在未被取消时才触发中止
            if !cancelled_clone.load(std::sync::atomic::Ordering::Relaxed) {
                controller_for_task.abort();
            }
        });
        AbortAfter { controller, signal, cancelled }
    }
}

/// 组合超时和信号的中止句柄
///
/// 由 [`abort_after_any`] 函数返回，结合了超时中止和其他中止信号。
/// 当超时或任一输入信号触发时，组合信号会被中止。
///
/// # 字段
///
/// - `signal`：组合后的中止信号
/// - `timeout`：内部超时中止句柄
pub struct AbortAfterAny {
    /// 组合后的中止信号，在超时或任一信号触发时中止
    pub signal: AbortSignal,
    /// 内部超时中止句柄
    timeout: AbortAfter,
}

impl AbortAfterAny {
    /// 取消超时定时器
    ///
    /// 取消内部的超时定时任务。调用后，只有通过输入信号才能触发中止。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let (ctrl, signal) = AbortController::new();
    /// let mut abort_any = abort_after_any(5000, [signal]);
    ///
    /// // 取消超时，现在只有通过 ctrl.abort() 才能触发中止
    /// abort_any.clear_timeout();
    /// ```
    pub fn clear_timeout(&mut self) {
        self.timeout.clear_timeout();
    }
}

/// 创建超时和信号的组合中止
///
/// 创建一个组合中止机制，在以下任一条件满足时触发中止：
/// 1. 经过指定的毫秒数（超时）
/// 2. 任一输入的中止信号被触发
///
/// 这对于实现"超时或取消，以先发生者为准"的模式非常有用。
///
/// # 参数
///
/// - `ms`：超时时间，单位为毫秒
/// - `signals`：其他中止信号的迭代器
///
/// # 返回值
///
/// 返回 `AbortAfterAny` 结构体，包含：
/// - `signal`：组合后的中止信号
/// - 可通过 `clear_timeout` 取消超时
///
/// # 示例
///
/// ```rust,ignore
/// let (user_cancel, cancel_signal) = AbortController::new();
///
/// // 创建组合中止：10秒超时或用户取消
/// let abort_any = abort_after_any(10000, [cancel_signal]);
///
/// tokio::spawn(async move {
///     let mut signal = abort_any.signal;
///     signal.cancelled().await;
///     println!("操作完成或被取消");
/// });
///
/// // 用户可以手动取消
/// user_cancel.abort();
/// ```
pub fn abort_after_any(ms: u64, signals: impl IntoIterator<Item = AbortSignal>) -> AbortAfterAny {
    // 创建超时中止
    let timeout = abort_after(ms);
    // 将超时信号与其他信号组合
    let signal = AbortSignal::any(std::iter::once(timeout.signal.clone()).chain(signals));
    AbortAfterAny { signal, timeout }
}

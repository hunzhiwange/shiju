//! 任务集合工具模块
//!
//! 本模块提供了跨平台的异步任务集合抽象，用于管理和执行多个并发任务。
//! 根据目标平台自动选择最合适的实现：
//! - 在非 WebAssembly 平台上使用 `tokio::task::JoinSet` 获得最佳性能
//! - 在 WebAssembly 平台上使用 `futures_util::stream::FuturesUnordered` 确保兼容性
//!
//! # 主要功能
//!
//! - 创建任务集合实例
//! - 向集合中添加新的异步任务
//! - 等待并获取已完成的任务结果
//!
//! # 平台差异
//!
//! 非 wasm32 平台利用 Tokio 的原生 JoinSet，提供更好的性能和更多控制能力。
//! wasm32 平台使用 FuturesUnordered 作为后备方案，确保在浏览器环境中正常工作。

use std::future::Future;
use tokio::task::JoinError;

/// 非 WebAssembly 平台的任务集合实现
///
/// 使用 Tokio 的 `JoinSet` 作为底层存储，提供高效的任务管理和调度能力。
/// 适用于标准的服务器端或桌面应用程序环境。
///
/// # 类型参数
///
/// - `T`: 任务完成时返回的结果类型，必须满足 `Send + 'static` 约束
#[cfg(not(target_arch = "wasm32"))]
pub struct TaskSet<T> {
    /// 内部的 Tokio JoinSet 实例，负责实际的任务存储和调度
    inner: tokio::task::JoinSet<T>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + 'static> TaskSet<T> {
    /// 创建一个新的空任务集合
    ///
    /// # 返回值
    ///
    /// 返回一个不包含任何任务的空 `TaskSet` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let task_set = TaskSet::<i32>::new();
    /// ```
    pub fn new() -> Self {
        Self { inner: tokio::task::JoinSet::new() }
    }

    /// 向集合中添加一个新的异步任务
    ///
    /// 任务将被立即调度执行。返回的 `AbortHandle` 可用于在必要时中止任务。
    ///
    /// # 类型参数
    ///
    /// - `F`: 要执行的 Future 类型，必须满足 `Future + Send + 'static`
    ///
    /// # 参数
    ///
    /// - `task`: 要添加的异步任务
    ///
    /// # 返回值
    ///
    /// 返回一个 `AbortHandle`，可用于取消该任务的执行
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let handle = task_set.spawn(async move {
    ///     // 执行一些异步操作
    ///     42
    /// });
    /// // 如果需要，可以调用 handle.abort() 来取消任务
    /// ```
    pub fn spawn<F>(&mut self, task: F) -> tokio::task::AbortHandle
    where
        F: Future<Output = T> + Send + 'static,
    {
        self.inner.spawn(task)
    }

    /// 异步等待并获取下一个已完成任务的结果
    ///
    /// 如果集合中有任务正在运行，此方法会等待其中一个完成并返回其结果。
    /// 如果集合为空或所有任务已完成，立即返回 `None`。
    ///
    /// # 返回值
    ///
    /// - `Some(Ok(result))`: 任务成功完成，返回结果
    /// - `Some(Err(join_error))`: 任务失败或被中止
    /// - `None`: 集合中没有正在运行的任务
    ///
    /// # 示例
    ///
    /// ```ignore
    /// while let Some(result) = task_set.join_next().await {
    ///     match result {
    ///         Ok(value) => println!("任务完成: {}", value),
    ///         Err(e) => eprintln!("任务失败: {}", e),
    ///     }
    /// }
    /// ```
    pub async fn join_next(&mut self) -> Option<Result<T, JoinError>> {
        self.inner.join_next().await
    }

    /// 非阻塞地尝试获取下一个已完成任务的结果
    ///
    /// 如果有任务已完成，立即返回其结果；否则返回 `None`。
    /// 此方法不会阻塞当前线程，适合在需要轮询的场景中使用。
    ///
    /// # 返回值
    ///
    /// - `Some(Ok(result))`: 有任务已完成且成功
    /// - `Some(Err(join_error))`: 有任务已完成但失败
    /// - `None`: 没有已完成的任务或集合为空
    ///
    /// # 示例
    ///
    /// ```ignore
    /// loop {
    ///     if let Some(result) = task_set.try_join_next() {
    ///         // 处理完成的任务
    ///     } else {
    ///         // 没有完成的任务，可以做其他事情
    ///         tokio::time::sleep(Duration::from_millis(10)).await;
    ///     }
    /// }
    /// ```
    pub fn try_join_next(&mut self) -> Option<Result<T, JoinError>> {
        self.inner.try_join_next()
    }
}

/// WebAssembly 平台的任务集合实现
///
/// 使用 `futures_util::stream::FuturesUnordered` 作为底层存储，
/// 以确保在 WebAssembly（如浏览器）环境中的兼容性。
/// 由于 wasm32 环境的特殊性，任务不需要满足 `Send` 约束。
///
/// # 类型参数
///
/// - `T`: 任务完成时返回的结果类型，必须满足 `'static` 约束
#[cfg(target_arch = "wasm32")]
pub struct TaskSet<T> {
    /// 内部的 FuturesUnordered 集合，存储所有待执行的任务句柄
    inner: futures_util::stream::FuturesUnordered<tokio::task::JoinHandle<T>>,
}

#[cfg(target_arch = "wasm32")]
impl<T: 'static> TaskSet<T> {
    /// 创建一个新的空任务集合
    ///
    /// # 返回值
    ///
    /// 返回一个不包含任何任务的空 `TaskSet` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let task_set = TaskSet::<i32>::new();
    /// ```
    pub fn new() -> Self {
        Self { inner: futures_util::stream::FuturesUnordered::new() }
    }

    /// 向集合中添加一个新的异步任务
    ///
    /// 在 wasm32 平台上，任务使用 `spawn_local` 创建，不需要满足 `Send` 约束。
    /// 这使得在单线程的 WebAssembly 环境中也能正常执行异步任务。
    ///
    /// # 类型参数
    ///
    /// - `F`: 要执行的 Future 类型，必须满足 `Future + 'static`
    ///
    /// # 参数
    ///
    /// - `task`: 要添加的异步任务
    ///
    /// # 注意
    ///
    /// 与非 wasm32 版本不同，此方法不返回 `AbortHandle`，
    /// 因为在 wasm32 环境中取消任务的机制有所不同
    pub fn spawn<F>(&mut self, task: F)
    where
        F: Future<Output = T> + 'static,
    {
        self.inner.push(tokio::task::spawn_local(task));
    }

    /// 异步等待并获取下一个已完成任务的结果
    ///
    /// 如果集合中有任务正在运行，此方法会等待其中一个完成并返回其结果。
    /// 如果集合为空，立即返回 `None`。
    ///
    /// # 返回值
    ///
    /// - `Some(Ok(result))`: 任务成功完成，返回结果
    /// - `Some(Err(join_error))`: 任务失败或被中止
    /// - `None`: 集合为空，没有正在运行的任务
    ///
    /// # 示例
    ///
    /// ```ignore
    /// while let Some(result) = task_set.join_next().await {
    ///     match result {
    ///         Ok(value) => println!("任务完成: {}", value),
    ///         Err(e) => eprintln!("任务失败: {}", e),
    ///     }
    /// }
    /// ```
    pub async fn join_next(&mut self) -> Option<Result<T, JoinError>> {
        use futures_util::StreamExt;

        // 如果集合为空，直接返回 None，避免无限等待
        if self.inner.is_empty() {
            return None;
        }

        // 等待下一个任务完成并返回其结果
        self.inner.next().await
    }

    /// 非阻塞地尝试获取下一个已完成任务的结果
    ///
    /// 使用 `now_or_never()` 方法尝试立即获取已完成任务的结果，
    /// 不会阻塞当前执行流。如果没有任务已完成，立即返回 `None`。
    ///
    /// # 返回值
    ///
    /// - `Some(Ok(result))`: 有任务已完成且成功
    /// - `Some(Err(join_error))`: 有任务已完成但失败
    /// - `None`: 没有已完成的任务或集合为空
    ///
    /// # 实现说明
    ///
    /// 此方法使用 `now_or_never()` 对流进行一次轮询。
    /// 对于 `FuturesUnordered`，这会检查是否有任何任务已经完成。
    /// 注意：此方法依赖于 `FuturesUnordered` 的轮询行为，
    /// 轮询操作本身是只读的，不会产生副作用。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// loop {
    ///     if let Some(result) = task_set.try_join_next() {
    ///         // 处理完成的任务
    ///     } else {
    ///         // 没有完成的任务，可以做其他事情
    ///         // 在 wasm32 环境中可能需要 yield 给事件循环
    ///     }
    /// }
    /// ```
    pub fn try_join_next(&mut self) -> Option<Result<T, JoinError>> {
        use futures_util::FutureExt;
        use futures_util::StreamExt;

        // 如果集合为空，直接返回 None
        if self.inner.is_empty() {
            return None;
        }

        // 使用 now_or_never() 尝试立即获取结果，不阻塞
        // 对于 FuturesUnordered，这会轮询一次流以检查是否有完成的任务
        // 如果有任务已完成，返回其结果；否则返回 None
        self.inner.next().now_or_never().flatten()
    }
}

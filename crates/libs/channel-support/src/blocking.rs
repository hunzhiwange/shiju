/// 在同步上下文中阻塞执行异步 Future。
///
/// 此函数处理两种情况：
/// 1. 如果已经在 Tokio 运行时中，使用 `block_in_place` 在当前上下文中执行
/// 2. 如果不在运行时中，创建新的单线程运行时来执行
///
/// # 平台支持
/// 仅在非 WASM 目标平台上可用，因为 WASM 不支持阻塞操作。
///
/// # 参数
/// - `fut`: 要执行的异步 Future
///
/// # 返回值
/// Future 的输出结果
///
/// # Panic
/// 如果无法创建新的 Tokio 运行时会 panic（极少发生）
#[cfg(not(target_arch = "wasm32"))]
pub fn block_on_future<F: std::future::Future>(fut: F) -> F::Output {
    // 尝试获取当前运行时句柄
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        // 在现有运行时中使用 block_in_place 避免阻塞整个运行时
        return tokio::task::block_in_place(|| handle.block_on(fut));
    }
    // 不在运行时中，创建新的单线程运行时
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
        .block_on(fut)
}

//! 超时控制工具模块
//!
//! 本模块提供异步操作的超时控制功能，用于防止长时间运行的异步任务阻塞系统。
//!
//! # 核心功能
//!
//! - 为异步操作添加超时限制
//! - 提供简洁的超时包装接口
//!
//! # 使用场景
//!
//! - 网络请求超时控制
//! - 长时间任务超时保护
//! - 外部服务调用超时管理

use std::future::Future;
use std::time::Duration;

/// 为异步操作添加超时限制
///
/// 该函数包装一个 Future，为其添加超时限制。如果 Future 在指定时间内未完成，
/// 则返回超时错误。
///
/// # 参数
///
/// - `fut`: 需要添加超时限制的异步操作
/// - `ms`: 超时时间（毫秒）
///
/// # 返回值
///
/// - `Ok(T)`: 异步操作在超时前成功完成，返回操作结果
/// - `Err(String)`: 操作超时，返回包含超时时间的错误信息
///
/// # 示例
///
/// ```rust
/// use std::time::Duration;
/// use agent_async::timeout::with_timeout;
///
/// async fn example() {
///     let result = with_timeout(
///         async { 42 },
///         1000  // 1秒超时
///     ).await;
///
///     match result {
///         Ok(value) => println!("操作成功: {}", value),
///         Err(e) => println!("操作超时: {}", e),
///     }
/// }
/// ```
///
/// # 注意事项
///
/// - 超时后，原始 Future 仍会继续运行直到完成或被丢弃
/// - 建议在调用方处理超时后的清理逻辑
pub async fn with_timeout<T>(fut: impl Future<Output = T>, ms: u64) -> Result<T, String> {
    tokio::time::timeout(Duration::from_millis(ms), fut)
        .await
        .map_err(|_| format!("Operation timed out after {}ms", ms))
}

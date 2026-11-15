//! # Agent 工具模块
//!
//! 本模块提供 Agent 运行时所需的各种通用工具函数、类型和辅助设施。
//!
//! ## 模块结构
//!
//! - `abort` - 任务中止相关工具
//! - `archive` - 归档与压缩工具
//! - `color` - 终端颜色输出工具
//! - `context` - 上下文管理工具
//! - `defer` - 延迟执行工具
//! - `eventloop` - 事件循环抽象
//! - `filesystem` - 文件系统操作工具
//! - `fn` - 函数式编程工具
//! - `format` - 数据格式化工具
//! - `iife` - 立即执行函数表达式工具
//! - `keybind` - 键盘快捷键绑定
//! - `lazy` - 延迟初始化工具
//! - `locale` - 本地化与国际化工具
//! - `lock` - 锁与同步原语
//! - `log` - 日志记录工具
//! - `proxied` - 代理相关工具
//! - `queue` - 队列数据结构
//! - `rpc` - 远程过程调用工具
//! - `scrap` - 数据抓取工具
//! - `signal` - 信号处理工具
//! - `task_set` - 任务集合管理
//! - `timeout` - 超时处理工具
//! - `token` - 令牌与计数工具
//! - `wildcard` - 通配符匹配工具

pub use agent_context::{context, defer, r#fn, iife, lazy, proxied, scrap};
pub use agent_text::{color, keybind, token, wildcard};

pub mod abort;
pub mod archive;
pub mod eventloop;
pub mod filesystem;
pub mod format;
pub mod locale;
pub mod lock;
pub mod log;
pub mod queue;
pub mod rpc;
pub mod signal;
pub mod task_set;
pub mod timeout;

#[cfg(test)]
#[path = "abort_tests.rs"]
mod abort_tests;
#[cfg(test)]
#[path = "archive_tests.rs"]
mod archive_tests;
#[cfg(test)]
#[path = "eventloop_tests.rs"]
mod eventloop_tests;
#[cfg(test)]
#[path = "filesystem_tests.rs"]
mod filesystem_tests;
#[cfg(test)]
#[path = "locale_tests.rs"]
mod locale_tests;
#[cfg(test)]
#[path = "lock_tests.rs"]
mod lock_tests;
#[cfg(test)]
#[path = "queue_tests.rs"]
mod queue_tests;
#[cfg(test)]
#[path = "rpc_tests.rs"]
mod rpc_tests;
#[cfg(test)]
#[path = "signal_tests.rs"]
mod signal_tests;
#[cfg(test)]
#[path = "task_set_tests.rs"]
mod task_set_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "timeout_tests.rs"]
mod timeout_tests;

use std::future::Future;

/// 在 tokio 运行时中派生一个异步任务（非 WASM 目标平台）。
///
/// 此函数是对 `tokio::spawn` 的简单封装，确保 Future 满足 `Send` 约束。
/// 适用于所有非 WebAssembly 目标平台。
///
/// # 类型参数
///
/// - `F` - 要执行的 Future 类型，必须满足 `Future + Send + 'static`
///
/// # 返回值
///
/// 返回 `tokio::task::JoinHandle<F::Output>`，可用于等待任务完成或中止任务。
///
/// # 示例
///
/// ```rust,ignore
/// use vibe_agent::util::spawn;
///
/// let handle = spawn(async {
///     // 执行异步操作
///     42
/// });
///
/// let result = handle.await.expect("任务执行失败");
/// assert_eq!(result, 42);
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future)
}

/// 在 tokio 运行时中派生一个异步任务（WASM 目标平台）。
///
/// 此函数使用 `tokio::task::spawn_local` 在当前线程派生任务，
/// 适用于 WebAssembly 目标平台，其中 Future 不需要满足 `Send` 约束。
///
/// # 类型参数
///
/// - `F` - 要执行的 Future 类型，必须满足 `Future + 'static`
///
/// # 返回值
///
/// 返回 `tokio::task::JoinHandle<F::Output>`，可用于等待任务完成或中止任务。
///
/// # 平台说明
///
/// 此函数仅在编译为 `wasm32` 目标时可用。
#[cfg(target_arch = "wasm32")]
pub fn spawn<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
    tokio::task::spawn_local(future)
}

/// 将字符串截断为最多 `max_chars` 个字符，如果发生截断则追加 "..."。
///
/// 此函数按字符（而非字节）计数，正确处理多字节 UTF-8 字符。
/// 如果字符串被截断，会在末尾追加省略号 "..."，并去除截断位置的尾部空白。
///
/// # 参数
///
/// - `s` - 要截断的字符串
/// - `max_chars` - 最大字符数（不包含省略号）
///
/// # 返回值
///
/// 返回截断后的字符串。如果原字符串长度不超过 `max_chars`，返回原字符串的克隆。
///
/// # 示例
///
/// ```
/// use vibe_agent::util::truncate_with_ellipsis;
///
/// // 截断超过限制的字符串
/// let result = truncate_with_ellipsis("Hello, World!", 5);
/// assert_eq!(result, "Hello...");
///
/// // 不截断未超过限制的字符串
/// let result = truncate_with_ellipsis("Hi", 5);
/// assert_eq!(result, "Hi");
///
/// // 正确处理多字节字符
/// let result = truncate_with_ellipsis("你好世界", 2);
/// assert_eq!(result, "你好...");
/// ```
pub fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => {
            let truncated = &s[..idx];
            format!("{}...", truncated.trim_end())
        }
        None => s.to_string(),
    }
}

/// 返回小于或等于 `index` 的最大有效 UTF-8 字符边界索引。
///
/// 在处理字符串切片时，直接使用字节索引可能导致切在多字节字符中间，
/// 从而引发 panic。此函数确保返回的索引总是一个有效的字符边界。
///
/// # 参数
///
/// - `s` - 目标字符串
/// - `index` - 期望的字节索引
///
/// # 返回值
///
/// 返回小于或等于 `index` 的最大有效 UTF-8 字符边界索引。
/// 如果 `index` 超出字符串长度，返回字符串长度。
///
/// # 示例
///
/// ```
/// use vibe_agent::util::floor_utf8_char_boundary;
///
/// // ASCII 字符串，所有字节边界都是字符边界
/// let s = "Hello";
/// assert_eq!(floor_utf8_char_boundary(s, 3), 3);
///
/// // 多字节字符："你" 占 3 字节，"好" 占 3 字节
/// let s = "你好";
/// // 索引 1 在 "你" 的内部，向下取整到 0
/// assert_eq!(floor_utf8_char_boundary(s, 1), 0);
/// // 索引 4 在 "好" 的内部，向下取整到 3
/// assert_eq!(floor_utf8_char_boundary(s, 4), 3);
///
/// // 超出边界返回字符串长度
/// assert_eq!(floor_utf8_char_boundary(s, 100), s.len());
/// ```
pub fn floor_utf8_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }

    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// 用于处理可选值的工具枚举。
///
/// 与标准库的 `Option<T>` 不同，`MaybeSet` 区分三种状态：
///
/// - `Set(T)` - 值已明确设置
/// - `Unset` - 值未设置（使用默认值）
/// - `Null` - 值被显式设为 null（不使用默认值）
///
/// 这种三态设计在处理配置合并、JSON 反序列化等场景中特别有用，
/// 能够区分"用户未提供该字段"和"用户显式设置为 null"。
///
/// # 类型参数
///
/// - `T` - 包装的值类型
///
/// # 示例
///
/// ```
/// use vibe_agent::util::MaybeSet;
///
/// let set_value = MaybeSet::Set(42);
/// let unset_value = MaybeSet::Unset;
/// let null_value = MaybeSet::Null;
///
/// // 可以使用 match 处理三种情况
/// match set_value {
///     MaybeSet::Set(v) => println!("值已设置: {}", v),
///     MaybeSet::Unset => println!("值未设置"),
///     MaybeSet::Null => println!("值为 null"),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaybeSet<T> {
    /// 值已明确设置
    Set(T),
    /// 值未设置
    Unset,
    /// 值被显式设为 null
    Null,
}

//! 标准输入输出重定向模块
//!
//! 本模块提供 CLI 模式下标准输出（stdout）和标准错误（stderr）的重定向功能。
//! 主要用于将代理运行时的输出重定向到日志文件，同时保持能够在作用域结束时
//! 自动恢复原始的标准输出/错误流。
//!
//! # 平台兼容性
//!
//! - **Unix 系统**：完整支持，使用 `libc` 的文件描述符操作（`dup`/`dup2`）实现
//! - **非 Unix 系统**：提供空实现，重定向操作为无操作（no-op）
//!
//! # 使用示例
//!
//! ```ignore
//! use std::path::Path;
//! use cli_stdio::StdIoRedirectGuard;
//!
//! {
//!     // 在此作用域内，stdout 和 stderr 将被重定向到日志文件
//!     let _guard = StdIoRedirectGuard::redirect_to_file(Path::new("logs/agent.log"))?;
//!
//!     // 这些输出将写入日志文件
//!     println!("这会被写入日志文件");
//!     eprintln!("这也会被写入日志文件");
//!
//!     // guard 在此处被 drop，自动恢复原始 stdout/stderr
//! }
//!
//! // 现在输出恢复到终端
//! println!("这会输出到终端");
//! ```
//!
//! # 实现原理
//!
//! 在 Unix 系统上，该模块使用 RAII（资源获取即初始化）模式：
//! 1. 创建 guard 时，保存原始的 stdout/stderr 文件描述符
//! 2. 将 stdout/stderr 重定向到指定的日志文件
//! 3. guard 被 drop 时，自动恢复原始的文件描述符
//!
//! # 安全性
//!
//! 本模块包含 `unsafe` 代码块，用于调用 Unix 系统调用（`libc`）。
//! 这些操作涉及原始文件描述符操作，需要谨慎使用。

use anyhow::Result;
#[cfg(unix)]
use std::fs::{OpenOptions, create_dir_all};
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::path::Path;

/// 标准输入输出重定向守卫
///
/// 该结构体用于管理 stdout 和 stderr 的重定向生命周期。
/// 采用 RAII 模式，确保在离开作用域时自动恢复原始的标准输出/错误流。
///
/// # 平台差异
///
/// - **Unix 系统**：包含保存的文件描述符，用于在 drop 时恢复
/// - **非 Unix 系统**：零大小类型，所有操作为无操作
///
/// # 线程安全性
///
/// 该结构体的操作影响进程级别的文件描述符，因此不适用于多线程并发场景。
/// 建议在单线程环境中使用，或确保在使用期间没有其他线程依赖 stdout/stderr。
#[cfg(unix)]
pub struct StdIoRedirectGuard {
    /// 保存的原始 stdout 文件描述符
    ///
    /// 在创建 guard 时通过 `libc::dup` 复制原始的 STDOUT_FILENO，
    /// 用于在 drop 时恢复 stdout。
    saved_stdout_fd: i32,

    /// 保存的原始 stderr 文件描述符
    ///
    /// 在创建 guard 时通过 `libc::dup` 复制原始的 STDERR_FILENO，
    /// 用于在 drop 时恢复 stderr。
    saved_stderr_fd: i32,
}

/// 标准输入输出重定向守卫（非 Unix 平台的空实现）
///
/// 在非 Unix 平台上，该结构体为空，所有重定向操作为无操作。
/// 这确保了代码可以在所有平台上编译，但只在 Unix 系统上实际执行重定向。
#[cfg(not(unix))]
pub struct StdIoRedirectGuard;

impl StdIoRedirectGuard {
    /// 将 stdout 和 stderr 重定向到指定文件
    ///
    /// 在 Unix 系统上，此方法会：
    /// 1. 确保日志文件的父目录存在
    /// 2. 以追加模式打开（或创建）日志文件
    /// 3. 保存原始的 stdout 和 stderr 文件描述符
    /// 4. 将 stdout 和 stderr 重定向到日志文件
    ///
    /// 在非 Unix 系统上，此方法为无操作，直接返回空的 guard。
    ///
    /// # 参数
    ///
    /// * `log_path` - 日志文件的路径。如果父目录不存在，会自动创建。
    ///   文件以追加模式打开，不会覆盖已有内容。
    ///
    /// # 返回值
    ///
    /// 成功时返回 `StdIoRedirectGuard` 实例。当该实例被 drop 时，
    /// 会自动恢复原始的 stdout 和 stderr。
    ///
    /// 失败时返回错误，可能的原因包括：
    /// - 无法创建父目录
    /// - 无法打开日志文件
    /// - 系统 `dup`/`dup2` 调用失败
    ///
    /// # 错误处理
    ///
    /// 该方法在重定向过程中的任何步骤失败时，都会：
    /// 1. 清理已分配的资源（关闭已打开的文件描述符）
    /// 2. 尽可能恢复到调用前的状态
    /// 3. 返回描述性的错误信息
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use std::path::Path;
    ///
    /// let guard = StdIoRedirectGuard::redirect_to_file(Path::new("output.log"))?;
    /// // 现在所有 stdout/stderr 输出都会写入 output.log
    /// ```
    ///
    /// # 注意事项
    ///
    /// - 该方法会影响整个进程的 stdout/stderr，包括其他线程的输出
    /// - 建议在程序初始化阶段调用，避免在运行时频繁切换
    /// - 日志文件以追加模式打开，多次运行会累积内容
    #[cfg(unix)]
    pub fn redirect_to_file(log_path: &Path) -> Result<Self> {
        // 确保日志文件的父目录存在
        // 如果路径没有父目录（如当前目录下的文件），则跳过创建
        if let Some(parent) = log_path.parent() {
            create_dir_all(parent)?;
        }

        // 以追加模式打开（或创建）日志文件
        // 使用追加模式确保多次运行不会覆盖之前的日志
        let log_file = OpenOptions::new().create(true).append(true).open(log_path)?;
        let log_fd = log_file.as_raw_fd();

        // 保存原始的 stdout 文件描述符
        // dup 系统调用创建文件描述符的副本，用于后续恢复
        let saved_stdout_fd = unsafe { libc::dup(libc::STDOUT_FILENO) };
        if saved_stdout_fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }

        // 保存原始的 stderr 文件描述符
        let saved_stderr_fd = unsafe { libc::dup(libc::STDERR_FILENO) };
        if saved_stderr_fd < 0 {
            // 如果保存 stderr 失败，需要清理已保存的 stdout
            unsafe {
                libc::close(saved_stdout_fd);
            }
            return Err(std::io::Error::last_os_error().into());
        }

        // 将 stdout 重定向到日志文件
        // dup2 会原子地关闭目标文件描述符（如果已打开）并重新指向源
        if unsafe { libc::dup2(log_fd, libc::STDOUT_FILENO) } < 0 {
            // 重定向失败时，清理已分配的资源
            unsafe {
                libc::close(saved_stdout_fd);
                libc::close(saved_stderr_fd);
            }
            return Err(std::io::Error::last_os_error().into());
        }

        // 将 stderr 重定向到日志文件
        if unsafe { libc::dup2(log_fd, libc::STDERR_FILENO) } < 0 {
            // stderr 重定向失败时，需要先恢复 stdout，然后清理资源
            unsafe {
                libc::dup2(saved_stdout_fd, libc::STDOUT_FILENO);
                libc::close(saved_stdout_fd);
                libc::close(saved_stderr_fd);
            }
            return Err(std::io::Error::last_os_error().into());
        }

        // 返回 guard，持有保存的文件描述符
        // 当 guard 被 drop 时，会自动恢复原始的 stdout/stderr
        Ok(Self { saved_stdout_fd, saved_stderr_fd })
    }

    /// 将 stdout 和 stderr 重定向到指定文件（非 Unix 平台实现）
    ///
    /// 在非 Unix 平台上，此方法为无操作，直接返回空的 guard。
    /// 这确保了代码可以在 Windows 等平台上编译和运行，
    /// 尽管不实际执行重定向操作。
    ///
    /// # 参数
    ///
    /// * `_log_path` - 日志文件路径（被忽略，因为不执行实际操作）
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(StdIoRedirectGuard)`，其中 guard 为空结构体。
    #[cfg(not(unix))]
    pub fn redirect_to_file(_log_path: &Path) -> Result<Self> {
        Ok(Self)
    }
}

/// 为 Unix 系统实现 Drop trait，确保自动恢复原始的 stdout/stderr
///
/// 当 `StdIoRedirectGuard` 离开作用域时，该实现会：
/// 1. 将 stdout 恢复为保存的文件描述符
/// 2. 将 stderr 恢复为保存的文件描述符
/// 3. 关闭保存的文件描述符以释放资源
///
/// # 错误处理
///
/// Drop 实现中的错误被静默忽略（使用 `let _ = ...`），
/// 因为：
/// - Drop 不应 panic
/// - 在程序退出时，文件描述符会被操作系统自动清理
/// - 恢复失败通常意味着更严重的系统问题
///
/// # 安全性
///
/// 使用 `unsafe` 块调用 `libc` 系统函数。
/// 假设保存的文件描述符在 drop 时仍然有效。
#[cfg(unix)]
impl Drop for StdIoRedirectGuard {
    fn drop(&mut self) {
        unsafe {
            // 恢复原始的 stdout
            // 忽略错误，因为 drop 不应 panic
            let _ = libc::dup2(self.saved_stdout_fd, libc::STDOUT_FILENO);

            // 恢复原始的 stderr
            let _ = libc::dup2(self.saved_stderr_fd, libc::STDERR_FILENO);

            // 关闭保存的文件描述符，释放资源
            // 这些是 dup 创建的副本，需要手动关闭
            let _ = libc::close(self.saved_stdout_fd);
            let _ = libc::close(self.saved_stderr_fd);
        }
    }
}

#[cfg(test)]
#[path = "stdio_tests.rs"]
mod stdio_tests;

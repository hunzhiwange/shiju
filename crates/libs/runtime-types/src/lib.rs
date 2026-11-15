//! 运行时适配器 trait 定义模块
//!
//! 本模块定义了 [`RuntimeAdapter`] 与 [`RuntimeBounds`]，用于抽象不同平台间的差异，
//! 让代理可以在 native、Docker、WASM 等多种执行环境中运行。
//!
//! # 设计目标
//!
//! - **能力声明**：各运行时通过 `has_shell_access` / `has_filesystem_access` / `supports_long_running`
//!   等方法声明自身的平台能力。
//! - **行为适配**：编排循环据此调整策略（例如在无 shell 访问的边缘环境中禁用 shell 工具）。
//! - **安全边界**：通过 `build_shell_command` 提供统一的命令构造点，便于插入沙箱与环境控制。
//!
//! # 扩展方式
//!
//! 如需支持新的运行时环境，请：
//! 1. 为该环境实现 [`RuntimeAdapter`]。
//! 2. 在对应的运行时模块（如 `native.rs` / `docker.rs` / `wasm.rs`）中提供具体实现。
//! 3. 通过工厂注册该运行时，使应用层可按配置选择。
//!
//! # 示例
//!
//! ```rust,ignore
//! use std::path::PathBuf;
//!
//! struct MyRuntime;
//!
//! impl RuntimeAdapter for MyRuntime {
//!     fn as_any(&self) -> &dyn std::any::Any { self }
//!     fn name(&self) -> &str { "my-runtime" }
//!     fn has_shell_access(&self) -> bool { true }
//!     fn has_filesystem_access(&self) -> bool { true }
//!     fn storage_path(&self) -> PathBuf { PathBuf::from("/var/lib/my-runtime") }
//!     fn supports_long_running(&self) -> bool { true }
//!
//!     #[cfg(not(target_arch = "wasm32"))]
//!     fn build_shell_command(
//!         &self,
//!         command: &str,
//!         workspace_dir: &std::path::Path,
//!     ) -> anyhow::Result<tokio::process::Command> {
//!         let mut cmd = tokio::process::Command::new("sh");
//!         cmd.arg("-c").arg(command).current_dir(workspace_dir);
//!         Ok(cmd)
//!     }
//! }
//! ```

use std::any::Any;
use std::path::{Path, PathBuf};

/// 非 WASM 目标平台的 trait 边界集合。
///
/// 在原生或 Docker 等标准宿主环境中，要求运行时适配器满足线程安全
/// (`Send + Sync`)，以便在多线程上下文中共享使用。
#[cfg(not(target_arch = "wasm32"))]
pub trait RuntimeBounds: Send + Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync> RuntimeBounds for T {}

/// WASM 目标平台的 trait 边界集合。
///
/// 在 WASM 环境中，`Send + Sync` 可能不可用，因此提供无额外边界的版本。
#[cfg(target_arch = "wasm32")]
pub trait RuntimeBounds {}
#[cfg(target_arch = "wasm32")]
impl<T> RuntimeBounds for T {}

/// 运行时适配器 trait，抽象平台差异。
///
/// 各运行时通过实现此 trait 来声明自身的能力，并提供平台相关的实现。
/// 编排循环会根据这些能力动态调整行为，例如：
/// - 无 shell 访问时禁用 shell 工具
/// - 无文件系统访问时切换到内存存储
/// - 不支持长驻进程时不启动网关与心跳循环
pub trait RuntimeAdapter: RuntimeBounds {
    /// 提供向下转型支持，允许获取运行时特有能力。
    ///
    /// # 返回值
    ///
    /// 返回 `&dyn Any`，调用者可使用 `downcast_ref` 按需获取具体类型。
    fn as_any(&self) -> &dyn Any;

    /// 返回此运行时环境的人类可读名称。
    ///
    /// 该名称用于日志与诊断输出，例如 `"native"`、`"docker"`、`"cloudflare-workers"`。
    ///
    /// # 返回值
    ///
    /// 返回运行时名称的字符串切片。
    fn name(&self) -> &str;

    /// 报告此运行时是否支持 shell 命令执行。
    ///
    /// 当返回 `false` 时，代理将禁用基于 shell 的工具。
    /// 无服务器和边缘运行时通常返回 `false`。
    ///
    /// # 返回值
    ///
    /// - `true`：支持执行 shell 命令
    /// - `false`：不支持 shell 执行
    fn has_shell_access(&self) -> bool;

    /// 报告此运行时是否支持文件系统读写。
    ///
    /// 当返回 `false` 时，代理将禁用基于文件的工具，并回退到内存存储。
    ///
    /// # 返回值
    ///
    /// - `true`：支持文件系统访问
    /// - `false`：不支持文件系统访问
    fn has_filesystem_access(&self) -> bool;

    /// 返回此运行时的持久化存储基目录。
    ///
    /// 内存后端、日志和其他数据将存储在此路径下。
    /// 实现应返回一个平台适当的可写目录。
    ///
    /// # 返回值
    ///
    /// 返回存储路径的 `PathBuf`。
    fn storage_path(&self) -> PathBuf;

    /// 报告此运行时是否支持长期运行的后台进程。
    ///
    /// 当返回 `true` 时，代理可以启动网关服务器、心跳循环等持久任务。
    /// 具有短执行时间限制的无服务器运行时应返回 `false`。
    ///
    /// # 返回值
    ///
    /// - `true`：支持长期运行进程
    /// - `false`：不支持长期运行进程
    fn supports_long_running(&self) -> bool;

    /// 返回此运行时的最大内存预算（字节）。
    ///
    /// 默认值为 `0`，表示无限制。
    /// 受限环境（嵌入式、无服务器）应返回实际内存上限，
    /// 以便代理调整缓冲区大小和缓存策略。
    ///
    /// # 返回值
    ///
    /// 返回最大内存字节数，`0` 表示无限制。
    fn memory_budget(&self) -> u64 {
        0
    }

    /// 构建一个配置好的 shell 命令进程。
    ///
    /// 创建一个 [`tokio::process::Command`]，将在 `workspace_dir` 目录下执行 `command`。
    /// 实现可以根据平台需要：
    /// - 添加沙箱包装器
    /// - 设置环境变量
    /// - 重定向 I/O
    ///
    /// # 参数
    ///
    /// - `command`：要执行的 shell 命令字符串
    /// - `workspace_dir`：命令的工作目录
    ///
    /// # 返回值
    ///
    /// 成功时返回配置好的 `tokio::process::Command`。
    ///
    /// # 错误
    ///
    /// - 运行时不支持 shell 访问时返回错误
    /// - 无法构造命令时返回错误（例如缺少 shell 二进制文件）
    #[cfg(not(target_arch = "wasm32"))]
    fn build_shell_command(
        &self,
        command: &str,
        workspace_dir: &Path,
    ) -> anyhow::Result<tokio::process::Command>;
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

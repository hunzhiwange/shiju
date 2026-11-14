//! 沙箱 trait，用于实现可插拔的操作系统级隔离。
//!
//! 本模块定义了 [`Sandbox`] trait，它抽象了操作系统级进程隔离后端。
//! 实现类通过平台特定的沙箱技术（如 seccomp、AppArmor、namespaces）
//! 包装 shell 命令，以限制工具执行的爆炸半径。代理运行时在执行任何
//! shell 命令之前会选择并应用一个沙箱后端。

use async_trait::async_trait;
use std::process::Command;

/// 沙箱后端的操作系统级进程隔离 trait。
///
/// 实现此 trait 以添加新的沙箱策略。运行时在启动时查询
/// [`is_available`](Sandbox::is_available) 来为当前平台选择最佳后端，
/// 然后在每次 shell 执行前调用 [`wrap_command`](Sandbox::wrap_command)。
///
/// 实现必须是 `Send + Sync`，因为沙箱可能在 Tokio 运行时的并发工具执行中共享。
#[cfg(not(target_arch = "wasm32"))]
pub trait SandboxBounds: Send + Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync> SandboxBounds for T {}

/// WASM32 平台的 SandboxBounds trait。
///
/// 在 WASM32 目标平台上，不要求 Send + Sync 约束。
#[cfg(target_arch = "wasm32")]
pub trait SandboxBounds {}
#[cfg(target_arch = "wasm32")]
impl<T> SandboxBounds for T {}

/// 沙箱 trait，定义操作系统级进程隔离的接口。
///
/// 此 trait 是沙箱后端的核心抽象，所有沙箱实现都必须实现此 trait。
/// 通过条件编译支持 WASM32 和非 WASM32 平台。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Sandbox: SandboxBounds {
    /// 用沙箱保护包装命令。
    ///
    /// 原地修改 `cmd` 以应用隔离约束（例如，添加包装二进制文件、
    /// 设置环境变量、添加 seccomp 过滤器）。
    ///
    /// # 参数
    ///
    /// * `cmd` - 要包装的命令，会被原地修改
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回 `std::io::Error`
    ///
    /// # 错误
    ///
    /// 当沙箱配置无法应用时返回 `std::io::Error`
    /// （例如，缺少包装二进制文件、无效的策略文件）。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// use std::process::Command;
    /// let sandbox = NoopSandbox::default();
    /// let mut cmd = Command::new("ls");
    /// sandbox.wrap_command(&mut cmd)?;
    /// ```
    fn wrap_command(&self, cmd: &mut Command) -> std::io::Result<()>;

    /// 检查此沙箱后端在当前平台上是否可用。
    ///
    /// 当所有必需的内核特性、二进制文件和权限都存在时返回 `true`。
    /// 运行时在启动时调用此方法来选择最强大的可用后端。
    ///
    /// # 返回值
    ///
    /// * `true` - 沙箱后端可用
    /// * `false` - 沙箱后端不可用
    fn is_available(&self) -> bool;

    /// 返回此沙箱后端的人类可读名称。
    ///
    /// 用于日志和诊断中标识当前激活的隔离策略
    /// （例如，`"firejail"`、`"bubblewrap"`、`"none"`）。
    ///
    /// # 返回值
    ///
    /// 沙箱后端的名称字符串切片
    fn name(&self) -> &str;

    /// 返回此沙箱提供的隔离保证的简要描述。
    ///
    /// 显示在状态输出和健康检查中，以便操作员验证活动的安全态势。
    ///
    /// # 返回值
    ///
    /// 沙箱隔离保证的描述字符串切片
    fn description(&self) -> &str;
}

/// 空操作沙箱，不提供额外的操作系统级隔离。
///
/// 总是报告自己为可用。当没有检测到平台特定的沙箱后端时，
/// 或者在不需要隔离的开发环境中，将其作为回退选项。
/// 在此模式下，安全性完全依赖于应用层控制。
///
/// # 示例
///
/// ```rust,ignore
/// use std::process::Command;
/// let sandbox = NoopSandbox::default();
/// let mut cmd = Command::new("echo");
/// sandbox.wrap_command(&mut cmd)?;
/// assert!(sandbox.is_available());
/// assert_eq!(sandbox.name(), "none");
/// ```
#[derive(Debug, Clone, Default)]
pub struct NoopSandbox;

impl Sandbox for NoopSandbox {
    /// 包装命令（空实现，直接通过）。
    ///
    /// 不对命令做任何修改，直接返回成功。
    fn wrap_command(&self, _cmd: &mut Command) -> std::io::Result<()> {
        Ok(())
    }

    /// 检查可用性（总是返回 true）。
    ///
    /// 空操作沙箱总是可用的，不需要任何系统依赖。
    fn is_available(&self) -> bool {
        true
    }

    /// 返回沙箱名称。
    ///
    /// 空操作沙箱的名称为 `"none"`。
    fn name(&self) -> &str {
        "none"
    }

    /// 返回沙箱描述。
    ///
    /// 说明在当前平台上没有可用的 Linux 内核 LSM 沙箱。
    fn description(&self) -> &str {
        "Linux kernel LSM sandboxing (not available on this platform)"
    }
}
#[cfg(test)]
#[path = "traits_tests.rs"]
mod traits_tests;

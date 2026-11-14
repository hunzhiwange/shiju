//! Landlock 沙箱模块（Linux 内核 5.13+ LSM）
//!
//! 本模块通过 Linux 内核提供非特权沙箱功能。Landlock 是一个 Linux 安全模块（LSM），
//! 允许非特权进程创建自己的安全沙箱，限制文件系统访问权限。
//!
//! # 功能特性
//!
//! - **非特权沙箱**：不需要 root 权限即可创建安全隔离环境
//! - **文件系统访问控制**：精细控制读、写、执行等文件操作权限
//! - **继承性限制**：限制会影响当前进程及其所有子进程
//! - **纯 Rust 实现**：使用 `landlock` crate，无 C 依赖
//!
//! # 系统要求
//!
//! - Linux 内核 5.13 或更高版本
//! - 启用 `sandbox-landlock` feature 标志
//! - 内核编译时启用了 Landlock LSM 支持
//!
//! # 使用场景
//!
//! - 限制代理运行时的文件系统访问范围
//! - 防止未授权的文件读写操作
//! - 为不受信任的代码执行提供安全边界
//!
//! # 示例
//!
//! ```no_run
//! use process_sandbox::landlock::LandlockSandbox;
//! use std::path::PathBuf;
//!
//! // 创建基本的 Landlock 沙箱
//! let sandbox = LandlockSandbox::new()?;
//!
//! // 或创建带工作目录的沙箱
//! let workspace = Some(PathBuf::from("/safe/workspace"));
//! let sandbox = LandlockSandbox::with_workspace(workspace)?;
//!
//! // 检查 Landlock 是否可用
//! if sandbox.is_available() {
//!     println!("Landlock 沙箱已就绪");
//! }
//! ```

#[cfg(all(feature = "sandbox-landlock", target_os = "linux"))]
use ::landlock::{AccessFs, PathBeneath, PathFd, Ruleset, RulesetAttr, RulesetCreatedAttr};

use super::traits::Sandbox;
#[cfg(all(feature = "sandbox-landlock", target_os = "linux"))]
use std::path::Path;

/// Linux Landlock 沙箱后端
///
/// `LandlockSandbox` 是一个基于 Linux Landlock LSM 的沙箱实现，
/// 用于控制进程对文件系统的访问权限。它通过定义规则集（ruleset）
/// 来限制进程可以访问的文件路径和操作类型。
///
/// # 架构说明
///
/// Landlock 使用以下核心概念：
///
/// - **规则集（Ruleset）**：定义允许的文件系统操作类型集合
/// - **路径规则（Path Rule）**：将特定路径添加到允许列表
/// - **限制应用**：通过 `restrict_self()` 将规则集应用到当前进程
///
/// # 安全模型
///
/// - 默认拒绝：未明确允许的访问将被拒绝
/// - 白名单模式：只有显式添加的路径和操作被允许
/// - 继承性：限制会传递给所有子进程
///
/// # 平台兼容性
///
/// - 在 Linux 上且启用 `sandbox-landlock` feature 时提供完整实现
/// - 在其他平台或未启用 feature 时提供存根实现（总是返回错误）
///
/// # 示例
///
/// ```no_run
/// use process_sandbox::landlock::LandlockSandbox;
///
/// // 探测 Landlock 可用性
/// match LandlockSandbox::probe() {
///     Ok(sandbox) => println!("Landlock 可用: {:?}", sandbox),
///     Err(e) => println!("Landlock 不可用: {}", e),
/// }
/// ```
#[cfg(all(feature = "sandbox-landlock", target_os = "linux"))]
#[derive(Debug)]
pub struct LandlockSandbox {
    /// 工作空间目录路径（可选）
    ///
    /// 如果设置，该目录将被允许读写访问，用于代理执行文件操作。
    /// 这提供了一个安全的、受控的工作区域。
    workspace_dir: Option<std::path::PathBuf>,
}

#[cfg(all(feature = "sandbox-landlock", target_os = "linux"))]
impl LandlockSandbox {
    /// 创建一个新的 Landlock 沙箱实例（无工作目录）
    ///
    /// 这是 `with_workspace(None)` 的便捷方法，用于创建一个
    /// 没有特定工作目录的基本沙箱配置。
    ///
    /// # 返回值
    ///
    /// - `Ok(LandlockSandbox)` - 成功创建沙箱实例
    /// - `Err(io::Error)` - Landlock 不可用（内核版本过低或未启用 LSM）
    ///
    /// # 错误
    ///
    /// 如果系统不支持 Landlock（内核版本 < 5.13 或未启用 Landlock LSM），
    /// 将返回 `io::ErrorKind::Unsupported` 错误。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use process_sandbox::landlock::LandlockSandbox;
    ///
    /// let sandbox = LandlockSandbox::new()?;
    /// println!("沙箱名称: {}", sandbox.name());
    /// ```
    pub fn new() -> std::io::Result<Self> {
        Self::with_workspace(None)
    }

    /// 创建带有指定工作目录的 Landlock 沙箱
    ///
    /// 创建一个沙箱实例，并设置允许读写访问的工作目录。
    /// 在应用限制时，该目录将被添加到允许列表中。
    ///
    /// # 参数
    ///
    /// - `workspace_dir` - 工作目录路径（可选）
    ///   - `Some(path)` - 设置工作目录，该目录将被允许读写访问
    ///   - `None` - 不设置特定工作目录
    ///
    /// # 返回值
    ///
    /// - `Ok(LandlockSandbox)` - 成功创建沙箱实例
    /// - `Err(io::Error)` - Landlock 不可用
    ///
    /// # 实现细节
    ///
    /// 此方法通过尝试创建最小规则集来测试 Landlock 是否可用：
    /// 1. 创建一个只包含基本文件读写权限的规则集
    /// 2. 尝试创建规则集实例
    /// 3. 如果成功，返回沙箱实例；如果失败，返回错误
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use process_sandbox::landlock::LandlockSandbox;
    /// use std::path::PathBuf;
    ///
    /// // 创建带工作目录的沙箱
    /// let workspace = Some(PathBuf::from("/tmp/agent_workspace"));
    /// let sandbox = LandlockSandbox::with_workspace(workspace)?;
    ///
    /// // 创建不带工作目录的沙箱
    /// let sandbox = LandlockSandbox::with_workspace(None)?;
    /// ```
    pub fn with_workspace(workspace_dir: Option<std::path::PathBuf>) -> std::io::Result<Self> {
        // 通过尝试创建最小规则集来测试 Landlock 是否可用
        // 这会在内核不支持 Landlock 或配置不正确时提前失败
        let test_ruleset = Ruleset::default()
            .handle_access(AccessFs::ReadFile | AccessFs::WriteFile)
            .and_then(|ruleset| ruleset.create());

        match test_ruleset {
            Ok(_) => Ok(Self { workspace_dir }),
            Err(e) => {
                tracing::debug!("Landlock 不可用: {}", e);
                Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Landlock 不可用"))
            }
        }
    }

    /// 探测 Landlock 是否可用（用于自动检测）
    ///
    /// 这是在选择沙箱后端时用于自动检测的首选方法。
    /// 它会尝试创建一个 Landlock 实例，成功则表示可用。
    ///
    /// # 返回值
    ///
    /// - `Ok(LandlockSandbox)` - Landlock 可用，返回可用的沙箱实例
    /// - `Err(io::Error)` - Landlock 不可用
    ///
    /// # 使用场景
    ///
    /// 通常在选择沙箱后端时使用，与其它后端（如 Firejail、Bubblewrap）
    /// 一起尝试，选择第一个可用的后端。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use process_sandbox::landlock::LandlockSandbox;
    ///
    /// // 在选择后端时自动检测
    /// if let Ok(sandbox) = LandlockSandbox::probe() {
    ///     println!("使用 Landlock 后端");
    /// } else {
    ///     println!("Landlock 不可用，尝试其他后端");
    /// }
    /// ```
    pub fn probe() -> std::io::Result<Self> {
        Self::new()
    }

    /// 将 Landlock 限制应用到当前进程
    ///
    /// 这是一个内部方法，用于配置并应用文件系统访问限制规则。
    /// 它会创建规则集，添加允许的路径，然后应用到当前进程。
    ///
    /// # 返回值
    ///
    /// - `Ok(())` - 限制应用成功
    /// - `Err(io::Error)` - 应用限制失败
    ///
    /// # 限制规则
    ///
    /// 此方法会设置以下限制：
    ///
    /// 1. **工作目录**：如果设置了 `workspace_dir`，允许读写和列出目录
    /// 2. **/tmp 目录**：允许读写临时文件
    /// 3. **/usr 目录**：只读访问，用于读取库和程序
    /// 4. **/bin 目录**：只读访问，用于执行基本命令
    ///
    /// # 安全注意事项
    ///
    /// - 限制是永久性的，会影响当前进程及其所有子进程
    /// - 一旦应用，无法撤销或放宽限制
    /// - 后续调用只能添加更多限制，不能移除已有限制
    ///
    /// # 错误处理
    ///
    /// 如果任何路径不存在或无法访问，会返回错误。
    /// 应用限制失败时会记录警告日志。
    fn apply_restrictions(&self) -> std::io::Result<()> {
        // 创建规则集并定义要处理的文件系统操作类型
        // 这些是我们想要限制的权限集合
        let mut ruleset = Ruleset::default()
            .handle_access(
                AccessFs::ReadFile      // 读取文件
                    | AccessFs::WriteFile   // 写入文件
                    | AccessFs::ReadDir     // 读取目录
                    | AccessFs::RemoveDir   // 删除目录
                    | AccessFs::RemoveFile  // 删除文件
                    | AccessFs::MakeChar    // 创建字符设备
                    | AccessFs::MakeSock    // 创建套接字
                    | AccessFs::MakeFifo    // 创建命名管道
                    | AccessFs::MakeBlock   // 创建块设备
                    | AccessFs::MakeReg     // 创建普通文件
                    | AccessFs::MakeSym, // 创建符号链接
            )
            .and_then(|ruleset| ruleset.create())
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        // 添加工作目录到允许列表（如果已设置）
        // 工作目录将被授予读、写和列出目录的权限
        if let Some(ref workspace) = self.workspace_dir {
            if workspace.exists() {
                // 获取工作目录的文件描述符
                let workspace_fd =
                    PathFd::new(workspace).map_err(|e| std::io::Error::other(e.to_string()))?;

                // 将工作目录添加到规则集，授予读写和列出权限
                ruleset = ruleset
                    .add_rule(PathBeneath::new(
                        workspace_fd,
                        AccessFs::ReadFile | AccessFs::WriteFile | AccessFs::ReadDir,
                    ))
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
            }
        }

        // 允许 /tmp 目录的读写访问，用于临时文件操作
        // 这对于许多程序创建临时文件是必需的
        let tmp_fd =
            PathFd::new(Path::new("/tmp")).map_err(|e| std::io::Error::other(e.to_string()))?;
        ruleset = ruleset
            .add_rule(PathBeneath::new(tmp_fd, AccessFs::ReadFile | AccessFs::WriteFile))
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        // 允许 /usr 目录的只读访问，用于执行命令和读取库文件
        // 大多数系统程序和库都位于 /usr 下
        let usr_fd =
            PathFd::new(Path::new("/usr")).map_err(|e| std::io::Error::other(e.to_string()))?;
        ruleset = ruleset
            .add_rule(PathBeneath::new(usr_fd, AccessFs::ReadFile | AccessFs::ReadDir))
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        // 允许 /bin 目录的只读访问，用于执行基本系统命令
        // 包含基本的 shell 命令和系统工具
        let bin_fd =
            PathFd::new(Path::new("/bin")).map_err(|e| std::io::Error::other(e.to_string()))?;
        ruleset = ruleset
            .add_rule(PathBeneath::new(bin_fd, AccessFs::ReadFile | AccessFs::ReadDir))
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        // 应用规则集到当前进程
        // 这会立即限制当前进程及其所有未来子进程的文件系统访问
        match ruleset.restrict_self() {
            Ok(_) => {
                tracing::debug!("Landlock 限制已成功应用");
                Ok(())
            }
            Err(e) => {
                tracing::warn!("应用 Landlock 限制失败: {}", e);
                Err(std::io::Error::other(e.to_string()))
            }
        }
    }
}

/// 为 LandlockSandbox 实现 Sandbox trait
///
/// 此实现将 Landlock 集成到统一的沙箱接口中，
/// 允许与其他沙箱后端（如 Firejail、Bubblewrap）互换使用。
#[cfg(all(feature = "sandbox-landlock", target_os = "linux"))]
impl Sandbox for LandlockSandbox {
    /// 包装命令以在沙箱中执行
    ///
    /// **当前状态：未实现（返回错误）**
    ///
    /// # 技术限制
    ///
    /// Landlock 的 `restrict_self()` 方法会影响当前进程及其所有后代进程。
    /// 如果在这里应用限制，会导致：
    ///
    /// 1. **永久限制父进程**：每次调用都会收紧长期运行的代理运行时的权限
    /// 2. **累积效应**：多次调用会导致权限逐渐退化
    /// 3. **无法回滚**：Landlock 限制一旦应用就无法撤销
    ///
    /// # 解决方案
    ///
    /// 要安全地实现此功能，需要：
    /// - 在子进程的 pre-exec 路径中应用限制（fork 后、exec 前）
    /// - 或者使用其他后端如 Firejail、Bubblewrap 或 Docker
    ///
    /// # 参数
    ///
    /// - `_cmd` - 要包装的命令（当前未使用）
    ///
    /// # 返回值
    ///
    /// 总是返回 `Unsupported` 错误，建议使用其他沙箱后端
    ///
    /// # 错误
    ///
    /// ```text
    /// "Landlock 逐命令包装尚不支持；请使用 firejail、bubblewrap 或 docker 后端"
    /// ```
    fn wrap_command(&self, _cmd: &mut std::process::Command) -> std::io::Result<()> {
        // `restrict_self()` 会影响当前进程和所有后代进程。
        // 在这里应用会永久收紧父代理运行时的权限，
        // 每次命令调用都会导致执行能力退化。
        //
        // 直到我们可以在子进程 pre-exec 路径中应用限制之前，
        // 选择安全失败而不是改变长期运行的父进程。
        let _ = &self.workspace_dir;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Landlock 逐命令包装尚不支持；请使用 firejail、bubblewrap 或 docker 后端",
        ))
    }

    /// 检查 Landlock 沙箱是否可用
    ///
    /// 通过尝试创建最小规则集来验证 Landlock 是否可以在当前系统上工作。
    /// 这比简单的特性检查更可靠，因为它会实际测试内核支持。
    ///
    /// # 返回值
    ///
    /// - `true` - Landlock 可用且可以创建规则集
    /// - `false` - Landlock 不可用（内核版本过低或 LSM 未启用）
    ///
    /// # 实现细节
    ///
    /// 此方法会：
    /// 1. 创建一个只包含读取文件权限的最小规则集
    /// 2. 尝试实例化规则集
    /// 3. 根据结果返回布尔值
    fn is_available(&self) -> bool {
        // 通过尝试创建最小规则集来验证可用性
        // 这比简单检查特性标志更可靠
        Ruleset::default()
            .handle_access(AccessFs::ReadFile)
            .and_then(|ruleset| ruleset.create())
            .is_ok()
    }

    /// 获取沙箱后端名称
    ///
    /// # 返回值
    ///
    /// 返回固定字符串 `"landlock"`
    fn name(&self) -> &str {
        "landlock"
    }

    /// 获取沙箱后端描述
    ///
    /// # 返回值
    ///
    /// 返回描述性字符串，说明这是 Linux 内核 LSM 沙箱，
    /// 专注于文件系统访问控制
    fn description(&self) -> &str {
        "Linux 内核 LSM 沙箱（文件系统访问控制）"
    }
}

/// 非 Linux 平台或未启用 feature 时的存根实现
///
/// 这是一个空的结构体，用于在不支持 Landlock 的平台上提供类型兼容性。
/// 所有方法都会返回 `Unsupported` 错误。
#[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
#[derive(Debug)]
pub struct LandlockSandbox;

/// 非 Linux 平台的 LandlockSandbox 实现
///
/// 所有方法都返回错误，表示 Landlock 在当前平台上不可用。
/// 这确保了代码可以在所有平台上编译，但只在支持的平台上有功能实现。
#[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
impl LandlockSandbox {
    /// 创建新的 Landlock 沙箱（存根实现）
    ///
    /// # 返回值
    ///
    /// 总是返回 `Unsupported` 错误，说明 Landlock 仅在启用相关 feature 的 Linux 上受支持
    pub fn new() -> std::io::Result<Self> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Landlock 仅在启用 sandbox-landlock feature 的 Linux 上受支持",
        ))
    }

    /// 创建带工作目录的 Landlock 沙箱（存根实现）
    ///
    /// # 参数
    ///
    /// - `_workspace_dir` - 工作目录（被忽略，因为平台不支持）
    ///
    /// # 返回值
    ///
    /// 总是返回 `Unsupported` 错误
    pub fn with_workspace(_workspace_dir: Option<std::path::PathBuf>) -> std::io::Result<Self> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Landlock 仅在 Linux 上受支持"))
    }

    /// 探测 Landlock 可用性（存根实现）
    ///
    /// # 返回值
    ///
    /// 总是返回 `Unsupported` 错误
    pub fn probe() -> std::io::Result<Self> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Landlock 仅在 Linux 上受支持"))
    }
}

/// 非 Linux 平台的 Sandbox trait 实现（存根）
///
/// 所有方法返回错误或 false，确保在不支持的平台上不会误用 Landlock。
#[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
impl Sandbox for LandlockSandbox {
    /// 包装命令（存根实现）
    ///
    /// # 返回值
    ///
    /// 总是返回 `Unsupported` 错误
    fn wrap_command(&self, _cmd: &mut std::process::Command) -> std::io::Result<()> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Landlock 仅在 Linux 上受支持"))
    }

    /// 检查可用性（存根实现）
    ///
    /// # 返回值
    ///
    /// 总是返回 `false`
    fn is_available(&self) -> bool {
        false
    }

    /// 获取名称
    ///
    /// 即使在不可用的平台上也返回 "landlock"，用于标识
    fn name(&self) -> &str {
        "landlock"
    }

    /// 获取描述
    ///
    /// 明确说明在当前平台上不可用
    fn description(&self) -> &str {
        "Linux 内核 LSM 沙箱（在当前平台上不可用）"
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

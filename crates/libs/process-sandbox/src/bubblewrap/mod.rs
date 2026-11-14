//! Bubblewrap 沙箱模块（基于用户命名空间的 Linux/macOS 沙箱）
//!
//! 本模块实现了基于 Bubblewrap (bwrap) 的沙箱隔离机制，通过 Linux 用户命名空间
//! 提供进程级别的安全隔离。Bubblewrap 是一个非特权沙箱工具，允许普通用户创建
//! 隔离的执行环境。
//!
//! # 核心功能
//!
//! - **文件系统隔离**：通过绑定挂载控制文件系统访问权限
//! - **命名空间隔离**：使用 `--unshare-all` 隔离所有命名空间
//! - **进程生命周期管理**：父进程终止时自动终止沙箱进程
//! - **设备访问控制**：限制对设备文件的访问
//!
//! # 安全特性
//!
//! - 只读挂载系统目录（如 `/usr`）防止系统文件被篡改
//! - 独立的 `/tmp` 目录避免临时文件冲突
//! - 独立的设备和进程命名空间隔离资源访问
//!
//! # 使用场景
//!
//! - 执行不可信代码时的安全隔离
//! - 限制工具执行的文件系统访问范围
//! - 防止恶意代码对宿主系统造成破坏
//!
//! # 前置要求
//!
//! 系统需安装 `bwrap` (bubblewrap) 工具。在大多数 Linux 发行版中可通过包管理器安装：
//! - Debian/Ubuntu: `apt install bubblewrap`
//! - Fedora: `dnf install bubblewrap`
//! - Arch Linux: `pacman -S bubblewrap`

use super::traits::Sandbox;
use std::process::Command;

/// Bubblewrap 沙箱后端实现
///
/// 该结构体实现了 `Sandbox` trait，使用 Bubblewrap 工具为命令执行提供沙箱隔离。
/// Bubblewrap 通过 Linux 用户命名空间创建隔离环境，无需 root 权限即可运行。
///
/// # 隔离策略
///
/// 该实现采用以下隔离策略：
/// - `/usr` 目录只读挂载，保护系统核心文件
/// - `/dev` 设备目录独立，限制硬件访问
/// - `/proc` 进程信息目录独立，隔离进程信息
/// - `/tmp` 临时目录可读写，允许临时文件操作
/// - 所有命名空间独立（网络、IPC、PID 等）
/// - 沙箱进程随父进程终止而终止
///
/// # 示例
///
/// ```rust,no_run
/// use std::process::Command;
/// use process_sandbox::bubblewrap::BubblewrapSandbox;
/// use process_sandbox::traits::Sandbox;
///
/// // 创建沙箱实例
/// let sandbox = BubblewrapSandbox::new()?;
///
/// // 检查沙箱是否可用
/// if sandbox.is_available() {
///     // 包装命令以在沙箱中执行
///     let mut cmd = Command::new("ls");
///     cmd.arg("-la");
///     sandbox.wrap_command(&mut cmd)?;
///
///     // 执行沙箱化的命令
///     let output = cmd.output()?;
/// }
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct BubblewrapSandbox;

impl BubblewrapSandbox {
    /// 创建新的 Bubblewrap 沙箱实例
    ///
    /// 检查系统中是否已安装 `bwrap` 工具，若已安装则返回沙箱实例，
    /// 否则返回 `NotFound` 错误。
    ///
    /// # 返回值
    ///
    /// - `Ok(BubblewrapSandbox)` - 沙箱创建成功
    /// - `Err(io::Error)` - `bwrap` 未安装，错误类型为 `ErrorKind::NotFound`
    ///
    /// # 错误
    ///
    /// 当 `bwrap` 命令不在系统 PATH 中或执行失败时返回错误。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use process_sandbox::bubblewrap::BubblewrapSandbox;
    ///
    /// match BubblewrapSandbox::new() {
    ///     Ok(sandbox) => println!("沙箱已就绪"),
    ///     Err(e) => eprintln!("无法创建沙箱: {}", e),
    /// }
    /// ```
    pub fn new() -> std::io::Result<Self> {
        if Self::is_installed() {
            Ok(Self)
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Bubblewrap not found"))
        }
    }

    /// 探测并创建 Bubblewrap 沙箱实例
    ///
    /// 该方法是 `new()` 的别名，用于探测系统环境并尝试创建沙箱实例。
    /// 主要用于 trait 对象的动态创建场景。
    ///
    /// # 返回值
    ///
    /// - `Ok(BubblewrapSandbox)` - 沙箱探测成功
    /// - `Err(io::Error)` - `bwrap` 未安装或不可用
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use process_sandbox::bubblewrap::BubblewrapSandbox;
    ///
    /// let sandbox = BubblewrapSandbox::probe()?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn probe() -> std::io::Result<Self> {
        Self::new()
    }

    /// 检查 Bubblewrap 是否已安装
    ///
    /// 通过执行 `bwrap --version` 命令检测系统是否安装了 Bubblewrap 工具。
    /// 该方法不会向标准输出打印任何内容，仅检查命令是否成功执行。
    ///
    /// # 返回值
    ///
    /// - `true` - `bwrap` 已安装且可执行
    /// - `false` - `bwrap` 未安装或不可执行
    ///
    /// # 实现细节
    ///
    /// 执行 `bwrap --version` 命令并检查退出状态码。若命令执行成功
    /// （退出码为 0），则认为已安装；否则认为未安装。
    fn is_installed() -> bool {
        Command::new("bwrap").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
    }
}

impl Sandbox for BubblewrapSandbox {
    /// 将命令包装为在 Bubblewrap 沙箱中执行
    ///
    /// 该方法接收一个 `Command` 实例，将其转换为通过 `bwrap` 执行的沙箱化命令。
    /// 原始命令的程序路径和参数会被保留，但在沙箱环境中执行。
    ///
    /// # 参数
    ///
    /// - `cmd` - 需要沙箱化的命令引用（可变）
    ///
    /// # 返回值
    ///
    /// - `Ok(())` - 命令包装成功
    /// - `Err(io::Error)` - 命令包装失败（通常不会发生）
    ///
    /// # 沙箱配置
    ///
    /// 应用的隔离策略如下：
    /// - `--ro-bind /usr /usr`: 只读挂载 `/usr` 目录（系统库和程序）
    /// - `--dev /dev`: 创建独立的设备文件系统
    /// - `--proc /proc`: 创建独立的进程信息文件系统
    /// - `--bind /tmp /tmp`: 可读写挂载 `/tmp` 目录（临时文件）
    /// - `--unshare-all`: 隔离所有命名空间（网络、IPC、PID、UTS、用户等）
    /// - `--die-with-parent`: 父进程终止时自动终止沙箱进程
    ///
    /// # 安全性
    ///
    /// 通过这些隔离措施，沙箱中的进程：
    /// - 无法修改系统文件（`/usr` 只读）
    /// - 拥有独立的设备和进程视图
    /// - 无法访问网络（命名空间隔离）
    /// - 无法访问宿主系统的其他进程
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use std::process::Command;
    /// use process_sandbox::bubblewrap::BubblewrapSandbox;
    /// use process_sandbox::traits::Sandbox;
    ///
    /// let sandbox = BubblewrapSandbox::new()?;
    /// let mut cmd = Command::new("python3");
    /// cmd.args(["-c", "print('Hello from sandbox')"]);
    ///
    /// // 包装命令后，实际执行的命令类似于：
    /// // bwrap --ro-bind /usr /usr --dev /dev --proc /proc \
    /// //       --bind /tmp /tmp --unshare-all --die-with-parent \
    /// //       python3 -c "print('Hello from sandbox')"
    /// sandbox.wrap_command(&mut cmd)?;
    ///
    /// let output = cmd.output()?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    fn wrap_command(&self, cmd: &mut Command) -> std::io::Result<()> {
        let program = cmd.get_program().to_string_lossy().to_string();
        let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

        let mut bwrap_cmd = Command::new("bwrap");
        bwrap_cmd.args([
            "--ro-bind",
            "/usr",
            "/usr",
            "--dev",
            "/dev",
            "--proc",
            "/proc",
            "--bind",
            "/tmp",
            "/tmp",
            "--unshare-all",
            "--die-with-parent",
        ]);
        bwrap_cmd.arg(&program);
        bwrap_cmd.args(&args);

        *cmd = bwrap_cmd;
        Ok(())
    }

    /// 检查沙箱是否可用
    ///
    /// 返回 Bubblewrap 工具是否已在系统中安装并可正常执行。
    /// 这是 `Sandbox` trait 的实现，用于动态选择可用的沙箱后端。
    ///
    /// # 返回值
    ///
    /// - `true` - 沙箱可用（`bwrap` 已安装）
    /// - `false` - 沙箱不可用
    fn is_available(&self) -> bool {
        Self::is_installed()
    }

    /// 获取沙箱名称
    ///
    /// 返回该沙箱后端的标识符名称，用于日志记录和调试。
    ///
    /// # 返回值
    ///
    /// 返回字符串 `"bubblewrap"`
    fn name(&self) -> &str {
        "bubblewrap"
    }

    /// 获取沙箱描述
    ///
    /// 返回该沙箱后端的人类可读描述信息，说明其技术特性和要求。
    ///
    /// # 返回值
    ///
    /// 返回描述字符串，包含沙箱类型和依赖要求
    fn description(&self) -> &str {
        "User namespace sandbox (requires bwrap)"
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

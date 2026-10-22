//! Native 运行时模块
//!
//! 本模块提供了原生运行时环境的实现，提供对宿主系统的完全访问能力。
//! 支持在 Mac、Linux、Docker 和 Raspberry Pi 等多种环境中运行。
//!
//! # 主要功能
//!
//! - **Shell 访问**：自动检测并配置适合当前系统的 shell 程序
//! - **文件系统访问**：提供完整的文件系统读写能力
//! - **命令执行**：支持在指定工作目录中执行 shell 命令
//! - **跨平台支持**：适配 Windows、Linux、macOS 等多种操作系统
//!
//! # Shell 检测策略
//!
//! - **Windows**：优先检测 bash、sh、pwsh、powershell，最后使用 cmd
//! - **Unix/Linux/macOS**：优先使用 `$SHELL` 环境变量指定的 shell，其次检测 zsh、bash、sh
//!
//! # 示例
//!
//! ```rust
//! use vibe_agent::runtime::native::NativeRuntime;
//! use vibe_agent::runtime::RuntimeAdapter;
//!
//! let runtime = NativeRuntime::new();
//! if runtime.has_shell_access() {
//!     println!("Shell 访问已启用: {:?}", runtime.selected_shell_kind());
//! }
//! ```

use super::traits::RuntimeAdapter;
use std::path::{Path, PathBuf};

/// Native 运行时 —— 提供对宿主系统的完全访问能力
///
/// 该运行时支持在 Mac、Linux、Docker 和 Raspberry Pi 等多种环境中运行，
/// 提供完整的 shell 访问、文件系统访问和命令执行能力。
///
/// # 特性
///
/// - 自动检测并配置适合当前系统的 shell 程序
/// - 支持完整的文件系统读写操作
/// - 支持长时间运行的进程
/// - 跨平台兼容（Windows、Linux、macOS）
///
/// # 示例
///
/// ```rust
/// let runtime = NativeRuntime::new();
/// assert!(runtime.has_filesystem_access());
/// ```
pub struct NativeRuntime {
    /// 检测到的 shell 程序配置
    /// 如果为 None，表示当前环境未找到可用的 shell
    shell: Option<ShellProgram>,
}

/// Shell 程序配置
///
/// 包含 shell 程序的类型和可执行文件路径信息。
#[derive(Debug, Clone, PartialEq, Eq)]
struct ShellProgram {
    /// Shell 类型（zsh、bash、sh、powershell 等）
    kind: ShellKind,
    /// Shell 可执行文件的完整路径
    program: PathBuf,
}

/// Shell 类型枚举
///
/// 定义支持的 shell 程序类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellKind {
    /// Zsh shell
    Zsh,
    /// 标准 POSIX shell
    Sh,
    /// Bash shell
    Bash,
    /// PowerShell Core（跨平台）
    Pwsh,
    /// Windows PowerShell
    PowerShell,
    /// Windows 命令提示符
    Cmd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellLaunchStyle {
    ZshLoginInteractive,
    Posix,
    BashLogin,
    PowerShell,
    Cmd,
}

impl ShellKind {
    /// 获取 shell 类型的字符串表示
    ///
    /// # 返回值
    ///
    /// 返回 shell 类型的字符串名称，用于日志和调试输出。
    ///
    /// # 示例
    ///
    /// ```rust
    /// let kind = ShellKind::Bash;
    /// assert_eq!(kind.as_str(), "bash");
    /// ```
    fn as_str(self) -> &'static str {
        match self {
            ShellKind::Zsh => "zsh",
            ShellKind::Sh => "sh",
            ShellKind::Bash => "bash",
            ShellKind::Pwsh => "pwsh",
            ShellKind::PowerShell => "powershell",
            ShellKind::Cmd => "cmd",
        }
    }

    fn launch_style(self) -> ShellLaunchStyle {
        match self {
            ShellKind::Zsh => ShellLaunchStyle::ZshLoginInteractive,
            ShellKind::Sh => ShellLaunchStyle::Posix,
            ShellKind::Bash => ShellLaunchStyle::BashLogin,
            ShellKind::Pwsh | ShellKind::PowerShell => ShellLaunchStyle::PowerShell,
            ShellKind::Cmd => ShellLaunchStyle::Cmd,
        }
    }
}

impl ShellProgram {
    /// 为命令进程添加 shell 特定的参数
    ///
    /// 根据 shell 类型，为 tokio::process::Command 添加适当的启动参数。
    /// 不同 shell 的参数格式各不相同，此方法确保命令能够正确执行。
    ///
    /// # 参数
    ///
    /// - `process`：要配置的命令进程对象
    /// - `command`：要执行的 shell 命令字符串
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let shell = ShellProgram { kind: ShellKind::Bash, program: PathBuf::from("/bin/bash") };
    /// let mut process = tokio::process::Command::new(&shell.program);
    /// shell.add_shell_args(&mut process, "echo hello");
    /// // 对于 bash，会添加 "-l" "-c" "echo hello" 参数
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    fn add_shell_args(&self, process: &mut tokio::process::Command, command: &str) {
        match self.kind.launch_style() {
            // zsh: 使用登录 shell、交互模式，执行命令
            ShellLaunchStyle::ZshLoginInteractive => {
                process.arg("-l").arg("-i").arg("-c").arg(command);
            }
            // sh: 标准 POSIX shell，直接执行命令
            ShellLaunchStyle::Posix => {
                process.arg("-c").arg(command);
            }
            // bash: 使用登录 shell 执行命令
            ShellLaunchStyle::BashLogin => {
                process.arg("-l").arg("-c").arg(command);
            }
            // PowerShell: 禁用启动Logo、配置文件，使用非交互模式执行命令
            ShellLaunchStyle::PowerShell => {
                process
                    .arg("-NoLogo")
                    .arg("-NoProfile")
                    .arg("-NonInteractive")
                    .arg("-Command")
                    .arg(command);
            }
            // cmd: 使用 /C 参数执行命令后退出
            ShellLaunchStyle::Cmd => {
                process.arg("/C").arg(command);
            }
        }
    }
}

/// 检测当前系统的原生 shell
///
/// 根据操作系统类型自动检测最适合的 shell 程序。
///
/// # 平台特定行为
///
/// - **wasm32 架构**：返回 None（不支持 shell 访问）
/// - **Windows**：检测 COMSPEC 环境变量，优先使用可用的 shell
/// - **Unix/Linux/macOS**：检测 SHELL 环境变量，优先使用用户配置的 shell
///
/// # 返回值
///
/// 返回检测到的 shell 程序配置，如果未找到可用 shell 则返回 None。
fn detect_native_shell() -> Option<ShellProgram> {
    // WASM 架构不支持 shell 访问
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
    // Windows 平台检测逻辑
    #[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
    {
        let comspec = std::env::var_os("COMSPEC").map(PathBuf::from);
        detect_native_shell_with(true, resolve_shell_on_path, comspec, None)
    }
    // Unix/Linux/macOS 平台检测逻辑
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "windows")))]
    {
        let user_shell = std::env::var_os("SHELL").map(PathBuf::from);
        detect_native_shell_with(false, resolve_shell_on_path, None, user_shell)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_shell_on_path(name: &str) -> Option<PathBuf> {
    which::which(name).ok()
}

fn windows_shell_candidates() -> [(&'static str, ShellKind); 6] {
    [
        ("bash", ShellKind::Bash),
        ("sh", ShellKind::Sh),
        ("pwsh", ShellKind::Pwsh),
        ("powershell", ShellKind::PowerShell),
        ("cmd", ShellKind::Cmd),
        ("cmd.exe", ShellKind::Cmd),
    ]
}

fn unix_shell_candidates() -> [(&'static str, ShellKind); 3] {
    [("zsh", ShellKind::Zsh), ("bash", ShellKind::Bash), ("sh", ShellKind::Sh)]
}

/// 使用自定义解析器检测原生 shell
///
/// 提供可配置的 shell 检测机制，支持自定义路径解析逻辑。
///
/// # 参数
///
/// - `is_windows`：是否为 Windows 系统
/// - `resolve`：用于解析命令名称到可执行文件路径的函数
/// - `comspec`：Windows COMSPEC 环境变量指定的命令处理器路径
/// - `user_shell`：用户配置的 shell 路径（Unix/Linux/macOS）
///
/// # 返回值
///
/// 返回检测到的 shell 程序配置，如果未找到可用 shell 则返回 None。
///
/// # 检测优先级
///
/// ## Windows
/// 1. bash（排除 WSL launcher）
/// 2. sh
/// 3. pwsh
/// 4. powershell
/// 5. cmd / cmd.exe
/// 6. COMSPEC 环境变量
///
/// ## Unix/Linux/macOS
/// 1. 用户 SHELL 环境变量
/// 2. zsh
/// 3. bash
/// 4. sh
fn detect_native_shell_with<F>(
    is_windows: bool,
    mut resolve: F,
    comspec: Option<PathBuf>,
    user_shell: Option<PathBuf>,
) -> Option<ShellProgram>
where
    F: FnMut(&str) -> Option<PathBuf>,
{
    if is_windows {
        // Windows 平台按优先级检测 shell
        for (name, kind) in windows_shell_candidates() {
            if let Some(program) = resolve(name) {
                // Windows 可能暴露 C:\Windows\System32\bash.exe，这是一个
                // 传统的 WSL launcher，会在 Linux 用户空间执行命令。
                // 这会破坏原生 Windows 命令（如 ipconfig）的执行。
                if name == "bash" && is_windows_wsl_bash_launcher(&program) {
                    continue;
                }
                return Some(ShellProgram { kind, program });
            }
        }
        // 如果未找到其他 shell，回退到 COMSPEC
        if let Some(program) = comspec {
            return Some(ShellProgram { kind: ShellKind::Cmd, program });
        }
        return None;
    }

    // Unix/Linux/macOS 平台优先使用用户配置的 shell
    if let Some(program) = user_shell {
        if let Some(kind) = classify_unix_shell_program(&program) {
            // 验证 shell 程序文件是否存在
            if program.exists() {
                return Some(ShellProgram { kind, program });
            }
        }
    }

    // 如果用户 shell 不可用，按优先级检测系统 shell
    for (name, kind) in unix_shell_candidates() {
        if let Some(program) = resolve(name) {
            return Some(ShellProgram { kind, program });
        }
    }
    None
}

/// 分类 Unix shell 程序类型
///
/// 根据可执行文件路径判断 shell 类型。
///
/// # 参数
///
/// - `program`：shell 可执行文件路径
///
/// # 返回值
///
/// 返回识别到的 shell 类型，如果无法识别则返回 None。
///
/// # 识别逻辑
///
/// - 文件名包含 "zsh" → ShellKind::Zsh
/// - 文件名包含 "bash" → ShellKind::Bash
/// - 文件名恰好为 "sh" → ShellKind::Sh
fn classify_unix_shell_program(program: &Path) -> Option<ShellKind> {
    // 提取文件名并转换为小写
    let shell_name = program.file_name()?.to_str()?.to_ascii_lowercase();
    if shell_name.contains("zsh") {
        return Some(ShellKind::Zsh);
    }
    if shell_name.contains("bash") {
        return Some(ShellKind::Bash);
    }
    if shell_name == "sh" {
        return Some(ShellKind::Sh);
    }
    None
}

/// 检查给定的 bash 路径是否为 Windows WSL launcher
///
/// Windows 系统中的 System32\bash.exe 是 WSL 的启动器，
/// 它会在 Linux 子系统中执行命令，而非原生 Windows 环境。
/// 需要排除此路径，以确保使用原生 Windows shell。
///
/// # 参数
///
/// - `program`：要检查的 bash 可执行文件路径
///
/// # 返回值
///
/// 如果是 WSL launcher 路径返回 true，否则返回 false。
fn is_windows_wsl_bash_launcher(program: &Path) -> bool {
    // 标准化路径格式，统一使用反斜杠并转换为小写
    let normalized = program.to_string_lossy().replace('/', "\\").to_ascii_lowercase();
    // 检查是否为已知的 WSL launcher 路径
    normalized.ends_with("\\windows\\system32\\bash.exe")
        || normalized.ends_with("\\windows\\sysnative\\bash.exe")
}

/// 生成缺失 shell 的错误消息
///
/// 根据操作系统类型返回相应的错误提示信息，
/// 指导用户如何安装和配置 shell 环境。
///
/// # 返回值
///
/// 返回静态字符串错误消息。
fn missing_shell_error() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "Native runtime could not find a usable shell (tried: bash, sh, pwsh, powershell, cmd). \
         Install Git Bash or PowerShell and ensure it is available on PATH."
    }
    #[cfg(not(target_os = "windows"))]
    {
        "Native runtime could not find a usable shell (tried: zsh, bash, sh). \
         Install a POSIX shell and ensure it is available on PATH."
    }
}

fn storage_path_from_home_dir(home_dir: Option<&Path>) -> PathBuf {
    match home_dir {
        Some(home_dir) => vw_config_types::paths::home_config_dir(home_dir),
        None => PathBuf::from(vw_config_types::paths::HOME_CONFIG_DIR_NAME),
    }
}

impl NativeRuntime {
    /// 创建新的 Native 运行时实例
    ///
    /// 自动检测当前系统的 shell 配置，初始化运行时环境。
    ///
    /// # 返回值
    ///
    /// 返回初始化后的 NativeRuntime 实例。
    ///
    /// # 示例
    ///
    /// ```rust
    /// let runtime = NativeRuntime::new();
    /// println!("Shell 访问: {}", runtime.has_shell_access());
    /// ```
    pub fn new() -> Self {
        Self { shell: detect_native_shell() }
    }

    /// 获取已选择的 shell 类型名称
    ///
    /// 返回当前配置的 shell 类型字符串，用于日志和调试。
    ///
    /// # 返回值
    ///
    /// - `Some(&str)`：shell 类型名称（如 "bash"、"zsh"、"cmd" 等）
    /// - `None`：未检测到可用的 shell
    ///
    /// # 示例
    ///
    /// ```rust
    /// let runtime = NativeRuntime::new();
    /// if let Some(kind) = runtime.selected_shell_kind() {
    ///     println!("当前 shell: {}", kind);
    /// }
    /// ```
    pub(crate) fn selected_shell_kind(&self) -> Option<&'static str> {
        self.shell.as_ref().map(|shell| shell.kind.as_str())
    }

    /// 获取已选择的 shell 可执行文件路径
    ///
    /// 返回当前配置的 shell 程序的完整路径。
    ///
    /// # 返回值
    ///
    /// - `Some(&Path)`：shell 可执行文件路径
    /// - `None`：未检测到可用的 shell
    ///
    /// # 示例
    ///
    /// ```rust
    /// let runtime = NativeRuntime::new();
    /// if let Some(path) = runtime.selected_shell_program() {
    ///     println!("Shell 路径: {:?}", path);
    /// }
    /// ```
    pub(crate) fn selected_shell_program(&self) -> Option<&Path> {
        self.shell.as_ref().map(|shell| shell.program.as_path())
    }

    /// 为测试创建 NativeRuntime 实例
    ///
    /// 允许在测试中指定自定义的 shell 配置，而不是自动检测。
    ///
    /// # 参数
    ///
    /// - `shell`：可选的 shell 程序配置
    ///
    /// # 返回值
    ///
    /// 返回配置了指定 shell 的 NativeRuntime 实例。
    #[cfg(test)]
    fn new_for_test(shell: Option<ShellProgram>) -> Self {
        Self { shell }
    }
}

impl RuntimeAdapter for NativeRuntime {
    /// 将运行时转换为 Any 类型引用
    ///
    /// 用于运行时类型检查和向下转型。
    ///
    /// # 返回值
    ///
    /// 返回运行时实例的 Any 类型引用。
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// 获取运行时名称
    ///
    /// 返回运行时的标识名称。
    ///
    /// # 返回值
    ///
    /// 返回 "native" 字符串。
    fn name(&self) -> &str {
        "native"
    }

    /// 检查是否具有 shell 访问权限
    ///
    /// 判断当前运行时是否配置了可用的 shell。
    ///
    /// # 返回值
    ///
    /// - `true`：已检测到可用的 shell
    /// - `false`：未找到可用的 shell
    fn has_shell_access(&self) -> bool {
        self.shell.is_some()
    }

    /// 检查是否具有文件系统访问权限
    ///
    /// Native 运行时始终具有完整的文件系统访问能力。
    ///
    /// # 返回值
    ///
    /// 始终返回 `true`。
    fn has_filesystem_access(&self) -> bool {
        true
    }

    /// 获取存储路径
    ///
    /// 返回 VibeWindow 数据存储目录的路径。
    /// 优先使用用户主目录下的 `.shiju-agent` 目录，
    /// 如果无法确定用户主目录，则使用当前目录下的 `.shiju-agent`。
    ///
    /// # 返回值
    ///
    /// 返回存储目录的路径。
    ///
    /// # 示例
    ///
    /// ```rust
    /// let runtime = NativeRuntime::new();
    /// let storage = runtime.storage_path();
    /// println!("存储路径: {:?}", storage);
    /// ```
    fn storage_path(&self) -> PathBuf {
        let home_dir =
            directories::UserDirs::new().map(|user_dirs| user_dirs.home_dir().to_owned());
        storage_path_from_home_dir(home_dir.as_deref())
    }

    /// 检查是否支持长时间运行的进程
    ///
    /// Native 运行时支持守护进程和长时间运行的后台任务。
    ///
    /// # 返回值
    ///
    /// 始终返回 `true`。
    fn supports_long_running(&self) -> bool {
        true
    }

    /// 构建 shell 命令进程
    ///
    /// 创建并配置用于执行 shell 命令的 tokio 进程对象。
    ///
    /// # 参数
    ///
    /// - `command`：要执行的 shell 命令字符串
    /// - `workspace_dir`：命令执行的工作目录
    ///
    /// # 返回值
    ///
    /// - `Ok(Command)`：成功配置的命令进程对象
    /// - `Err`：未检测到可用的 shell 或配置失败
    ///
    /// # 错误
    ///
    /// 如果未检测到可用的 shell，返回包含错误信息的 anyhow::Error。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let runtime = NativeRuntime::new();
    /// let cmd = runtime.build_shell_command("ls -la", Path::new("/tmp"))?;
    /// // 执行命令...
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    fn build_shell_command(
        &self,
        command: &str,
        workspace_dir: &Path,
    ) -> anyhow::Result<tokio::process::Command> {
        // 获取 shell 配置，如果未配置则返回错误
        let shell = self.shell.as_ref().ok_or_else(|| anyhow::anyhow!(missing_shell_error()))?;

        // 创建命令进程并配置 shell 参数
        let mut process = tokio::process::Command::new(&shell.program);
        shell.add_shell_args(&mut process, command);
        // 设置工作目录
        process.current_dir(workspace_dir);
        Ok(process)
    }
}

#[cfg(test)]
mod tests;

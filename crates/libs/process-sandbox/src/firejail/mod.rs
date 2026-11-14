//! Firejail 沙箱模块（Linux 用户空间沙箱）
//!
//! 本模块提供基于 Firejail 的沙箱实现，用于在 Linux 系统上隔离和限制进程权限。
//!
//! # 功能概述
//!
//! - **进程隔离**：通过 Firejail 创建隔离的执行环境
//! - **资源限制**：限制进程对系统资源的访问（音频、视频、3D 加速等）
//! - **安全加固**：提供最小化的 /dev 目录和私有主目录
//! - **自动检测**：支持探测系统中是否安装 Firejail
//!
//! # 依赖要求
//!
//! 需要在系统上安装 Firejail：
//! ```bash
//! sudo apt install firejail
//! ```
//!
//! # 使用示例
//!
//! ```no_run
//! use process_sandbox::firejail::FirejailSandbox;
//! use process_sandbox::traits::Sandbox;
//! use std::process::Command;
//!
//! // 创建 Firejail 沙箱实例
//! let sandbox = FirejailSandbox::new()?;
//!
//! // 包装命令以在沙箱中执行
//! let mut cmd = Command::new("ls");
//! sandbox.wrap_command(&mut cmd)?;
//! # Ok::<(), std::io::Error>(())
//! ```

use super::traits::Sandbox;
use std::process::Command;

/// Firejail 沙箱后端实现
///
/// 该结构体为 Linux 系统提供基于 Firejail 的沙箱功能。
/// Firejail 是一个 SUID 沙箱程序，用于隔离 Linux 应用程序的执行环境。
///
/// # 安全特性
///
/// - 私有主目录：为每个沙箱进程创建独立的主目录
/// - 最小化设备访问：仅提供必要的 /dev 设备
/// - 禁用危险硬件：阻止音频、视频、3D 加速等硬件访问
/// - 无配置文件模式：跳过系统级 Firejail 配置文件加载
///
/// # 示例
///
/// ```no_run
/// use process_sandbox::firejail::FirejailSandbox;
///
/// // 创建新的沙箱实例
/// let sandbox = FirejailSandbox::new()?;
///
/// // 检查沙箱是否可用
/// assert!(sandbox.is_available());
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct FirejailSandbox;

impl FirejailSandbox {
    /// 创建新的 Firejail 沙箱实例
    ///
    /// 该方法会检查系统中是否已安装 Firejail，如果未安装则返回错误。
    ///
    /// # 返回值
    ///
    /// - `Ok(Self)`：成功创建沙箱实例
    /// - `Err(io::Error)`：Firejail 未安装，错误中包含安装提示
    ///
    /// # 错误
    ///
    /// 如果系统中未找到 Firejail 可执行文件，将返回 `ErrorKind::NotFound` 错误。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use process_sandbox::firejail::FirejailSandbox;
    ///
    /// match FirejailSandbox::new() {
    ///     Ok(sandbox) => println!("Firejail 沙箱已就绪"),
    ///     Err(e) => eprintln!("无法创建沙箱: {}", e),
    /// }
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn new() -> std::io::Result<Self> {
        if Self::is_installed() {
            Ok(Self)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Firejail 未安装。请使用以下命令安装：sudo apt install firejail",
            ))
        }
    }

    /// 探测 Firejail 是否可用（用于自动检测）
    ///
    /// 该方法用于在运行时自动检测系统中是否安装了 Firejail。
    /// 通常用于沙箱后端的自动选择逻辑中。
    ///
    /// # 返回值
    ///
    /// - `Ok(Self)`：Firejail 可用，返回沙箱实例
    /// - `Err(io::Error)`：Firejail 不可用
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use process_sandbox::firejail::FirejailSandbox;
    ///
    /// if let Ok(sandbox) = FirejailSandbox::probe() {
    ///     println!("检测到 Firejail，已启用沙箱保护");
    /// } else {
    ///     println!("未检测到 Firejail，将使用其他沙箱方案");
    /// }
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn probe() -> std::io::Result<Self> {
        Self::new()
    }

    /// 检查 Firejail 是否已安装
    ///
    /// 通过执行 `firejail --version` 命令来验证 Firejail 是否可用。
    ///
    /// # 返回值
    ///
    /// - `true`：Firejail 已安装且可正常执行
    /// - `false`：Firejail 未安装或执行失败
    fn is_installed() -> bool {
        Command::new("firejail")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl Sandbox for FirejailSandbox {
    /// 包装命令以在沙箱中执行
    ///
    /// 该方法将原始命令包装在 Firejail 沙箱环境中执行，
    /// 应用一系列安全限制以隔离进程。
    ///
    /// # 参数
    ///
    /// - `cmd`：需要包装的命令对象，将被原地修改为在沙箱中执行的版本
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：命令包装成功
    /// - `Err(io::Error)`：命令包装失败（当前实现不会返回错误）
    ///
    /// # 安全策略
    ///
    /// 应用的安全限制包括：
    /// - `--private=home`：创建新的私有主目录
    /// - `--private-dev`：使用最小化的 /dev 设备集合
    /// - `--nosound`：禁用音频设备访问
    /// - `--no3d`：禁用 3D 硬件加速
    /// - `--novideo`：禁用视频设备访问
    /// - `--nowheel`：禁用输入设备（鼠标滚轮等）
    /// - `--notv`：禁用电视设备
    /// - `--noprofile`：跳过用户配置文件加载
    /// - `--quiet`：抑制 Firejail 警告信息
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use process_sandbox::firejail::FirejailSandbox;
    /// use process_sandbox::traits::Sandbox;
    /// use std::process::Command;
    ///
    /// let sandbox = FirejailSandbox::new()?;
    /// let mut cmd = Command::new("bash");
    /// cmd.arg("-c").arg("echo 'Hello from sandbox'");
    ///
    /// sandbox.wrap_command(&mut cmd)?;
    /// // 现在 cmd 会在沙箱中执行
    /// # Ok::<(), std::io::Error>(())
    /// ```
    fn wrap_command(&self, cmd: &mut Command) -> std::io::Result<()> {
        // 提取原始命令的程序路径和参数
        let program = cmd.get_program().to_string_lossy().to_string();
        let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

        // 构建 Firejail 包装命令，应用安全标志
        let mut firejail_cmd = Command::new("firejail");
        firejail_cmd.args([
            "--private=home", // 创建新的私有主目录
            "--private-dev",  // 使用最小化的 /dev 设备集合
            "--nosound",      // 禁用音频设备访问
            "--no3d",         // 禁用 3D 硬件加速
            "--novideo",      // 禁用视频设备访问
            "--nowheel",      // 禁用输入设备
            "--notv",         // 禁用电视设备
            "--noprofile",    // 跳过配置文件加载
            "--quiet",        // 抑制警告信息
        ]);

        // 添加原始命令及其参数
        firejail_cmd.arg(&program);
        firejail_cmd.args(&args);

        // 用沙箱包装后的命令替换原命令
        *cmd = firejail_cmd;
        Ok(())
    }

    /// 检查沙箱是否可用
    ///
    /// # 返回值
    ///
    /// - `true`：Firejail 已安装且可用
    /// - `false`：Firejail 不可用
    fn is_available(&self) -> bool {
        Self::is_installed()
    }

    /// 获取沙箱名称
    ///
    /// # 返回值
    ///
    /// 返回沙箱后端的标识名称 "firejail"
    fn name(&self) -> &str {
        "firejail"
    }

    /// 获取沙箱描述
    ///
    /// # 返回值
    ///
    /// 返回沙箱后端的人类可读描述信息
    fn description(&self) -> &str {
        "Linux 用户空间沙箱（需要安装 firejail）"
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

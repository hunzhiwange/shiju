//! 平台 shell 沙箱执行器。
//!
//! 该模块负责把运行时生成的 shell 命令包装进平台沙箱：macOS 使用 seatbelt
//! `sandbox-exec`，Linux 使用 firejail。它只做后端探测和命令包装，不决定是否启用沙箱。

use std::path::Path;

use anyhow::Context;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
use anyhow::bail;
use tokio::process::Command;

use runtime_types::RuntimeAdapter;
#[cfg(target_os = "linux")]
use agent_security::firejail::FirejailSandbox;
#[cfg(target_os = "linux")]
use agent_security::traits::Sandbox;

#[cfg(target_os = "macos")]
use super::NetworkPolicy;
use super::SandboxConfig;

/// 将 shell 命令包进平台沙箱的执行器。
pub struct SandboxExecutor {
    config: SandboxConfig,
}

impl SandboxExecutor {
    /// 创建新的沙箱执行器。
    ///
    /// 参数：
    /// - `config`：文件系统、网络和开关策略。
    ///
    /// 返回值：持有该配置的执行器。
    /// 错误处理：构造本身不检查后端可用性；后端错误会在构建命令时返回。
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// 检测当前平台的沙箱后端是否可用。
    ///
    /// 返回值：macOS 上表示 `sandbox-exec` 可调用，Linux 上表示 firejail 可初始化；
    /// 其他平台返回 `false`。
    /// 错误处理：探测失败会折叠为 `false`。
    pub fn backend_available() -> bool {
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("sandbox-exec")
                .arg("-h")
                .output()
                .map(|output| output.status.success() || output.status.code() == Some(64))
                .unwrap_or(false)
        }

        #[cfg(target_os = "linux")]
        {
            FirejailSandbox::new().is_ok()
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            false
        }
    }

    /// 构建并包装一个 shell 命令。
    ///
    /// 参数：
    /// - `runtime`：负责生成基础 shell 命令的运行时适配器。
    /// - `command`：用户请求执行的 shell 字符串。
    /// - `workdir`：命令工作目录。
    ///
    /// 返回值：已经按平台沙箱包装的 Tokio 命令。
    /// 错误处理：运行时构建失败、后端初始化失败或平台不支持时返回错误。
    pub fn build_command(
        &self,
        runtime: &dyn RuntimeAdapter,
        command: &str,
        workdir: &Path,
    ) -> anyhow::Result<Command> {
        let mut cmd = runtime.build_shell_command(command, workdir).with_context(|| {
            format!("failed to build runtime command for {}", workdir.display())
        })?;

        self.wrap_command(&mut cmd, workdir)?;
        Ok(cmd)
    }

    fn wrap_command(&self, cmd: &mut Command, workdir: &Path) -> anyhow::Result<()> {
        #[cfg(target_os = "macos")]
        {
            self.wrap_with_seatbelt(cmd, workdir)
        }

        #[cfg(target_os = "linux")]
        {
            // Linux 后端交给 firejail 追加参数，避免本层和后端适配层出现两套权限解释。
            let sandbox =
                FirejailSandbox::new().context("failed to initialize firejail sandbox")?;
            sandbox
                .wrap_command(cmd.as_std_mut())
                .context("failed to wrap command with firejail")?;
            let _ = workdir;
            return Ok(());
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = (cmd, workdir);
            bail!("sandbox execution is not supported on this platform");
        }
    }

    #[cfg(target_os = "macos")]
    fn wrap_with_seatbelt(&self, cmd: &mut Command, workdir: &Path) -> anyhow::Result<()> {
        // sandbox-exec 需要作为外层进程启动，因此保留原始 program/args 后重建命令。
        let program = cmd.as_std().get_program().to_os_string();
        let args: Vec<_> = cmd.as_std().get_args().map(|arg| arg.to_os_string()).collect();
        let profile = self.generate_seatbelt_profile();

        let mut wrapped = Command::new("sandbox-exec");
        wrapped.arg("-p").arg(profile);
        wrapped.arg(&program);
        wrapped.args(&args);
        wrapped.current_dir(workdir);
        *cmd = wrapped;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn generate_seatbelt_profile(&self) -> String {
        let mut rules = vec!["(version 1)".to_string(), "(deny default)".to_string()];

        // macOS shell startup needs read access to dynamic libraries, locale data, and system
        // services. Keep writes and network constrained, while relying on pre-exec policy checks
        // for sensitive read path arguments.
        rules.push("(allow process*)".to_string());
        rules.push("(allow sysctl*)".to_string());
        rules.push("(allow mach-lookup)".to_string());
        rules.push("(allow file-read*)".to_string());

        // 先 deny default，再按策略显式放行写入路径，避免无意继承宿主进程写能力。
        for path in &self.config.filesystem.read_paths {
            rules.push(format!("(allow file-read* (subpath \"{}\"))", path.display()));
        }
        for path in &self.config.filesystem.write_paths {
            rules.push(format!("(allow file-read* file-write* (subpath \"{}\"))", path.display()));
        }
        for path in &self.config.filesystem.execute_paths {
            rules.push(format!("(allow process-exec (subpath \"{}\"))", path.display()));
        }
        rules.push("(allow file-read* file-write* (subpath \"/tmp\"))".to_string());
        rules.push("(allow file-read* file-write* (subpath \"/private/tmp\"))".to_string());

        match &self.config.network {
            NetworkPolicy::DenyAll => rules.push("(deny network*)".to_string()),
            NetworkPolicy::AllowAll | NetworkPolicy::AllowHosts(_) => {
                rules.push("(allow network*)".to_string());
            }
        }

        rules.join("\n")
    }
}

#[cfg(test)]
#[path = "executor_tests.rs"]
mod executor_tests;

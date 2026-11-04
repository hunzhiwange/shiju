//! ACP 内置代理安装执行：固定来源、用户级 uv 引导与进程树清理。
use std::process::Stdio;
use vw_config_types::acp_install::acp_install_command;

/// 按网关所在平台安装内置代理，不接收任意命令、包名或 URL。
pub async fn install_builtin_agent(agent: &str) -> Result<(), String> {
    if crate::managed_runtime::install(agent).await? {
        return Ok(());
    }
    let (program, args) = acp_install_command(agent)
        .ok_or_else(|| "该 ACP 暂无一键安装命令，请使用官方安装器".to_string())?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(300);
    if program == "uv"
        && installed_uv().is_none()
        && vw_shared::shell::resolve_executable("uv").is_none()
    {
        // uv 会自行准备所需 Python；使用官方用户级安装器，无需用户复制命令。
        let (bootstrap, args) = if cfg!(windows) {
            (
                "powershell",
                &[
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "$ErrorActionPreference='Stop'; & ([scriptblock]::Create((Invoke-RestMethod 'https://astral.sh/uv/install.ps1')))",
                ][..],
            )
        } else {
            (
                "bash",
                &[
                    "-c",
                    "set -euo pipefail; curl -fsSL --proto '=https' https://astral.sh/uv/install.sh | sh",
                ][..],
            )
        };
        run_installer(bootstrap, args, deadline).await?;
    }
    // uv 的安装目录可能刚刚创建，不能只依赖网关启动时缓存的 PATH。
    let executable = if program == "uv" {
        installed_uv().unwrap_or_else(|| program.into())
    } else {
        program.into()
    };
    run_installer(&executable, args, deadline).await?;
    Ok(())
}

pub(super) async fn ensure_uv(deadline: tokio::time::Instant) -> Result<String, String> {
    if let Some(path) = installed_uv().or_else(|| {
        vw_shared::shell::resolve_executable("uv").map(|path| path.to_string_lossy().into_owned())
    }) {
        return Ok(path);
    }
    run_installer(
        "bash",
        &[
            "-c",
            "set -euo pipefail; curl -fsSL --proto '=https' https://astral.sh/uv/install.sh | sh",
        ],
        deadline,
    )
    .await?;
    installed_uv().ok_or_else(|| "uv bootstrap did not install its executable".into())
}

pub(super) fn installed_uv() -> Option<String> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let bin = std::env::var_os("UV_INSTALL_DIR")
        .or_else(|| std::env::var_os("XDG_BIN_HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(home).join(".local/bin"));
    let path = bin.join(if cfg!(windows) { "uv.exe" } else { "uv" });
    path.is_file().then(|| path.to_string_lossy().into_owned())
}

// 请求断开或超时也必须停止 shell 启动的下载、解压等子进程。
struct InstallerTree(Option<u32>);
impl Drop for InstallerTree {
    fn drop(&mut self) {
        if let Some(pid) = self.0.take() {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
            #[cfg(windows)]
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
    }
}

pub(super) async fn run_installer(
    program: &str,
    args: &[&str],
    deadline: tokio::time::Instant,
) -> Result<(), String> {
    let mut command = crate::build_spawn_command(program, &Default::default());
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| format!("无法启动 {program}，请检查网关主机上的运行环境与 PATH"))?;
    let mut tree = InstallerTree(child.id());
    let status = match tokio::time::timeout_at(deadline, child.wait()).await {
        Ok(result) => result.map_err(|_| "无法获取安装进程状态".to_string())?,
        Err(_) => {
            drop(tree);
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err("安装超时，请检查网关主机网络与安装状态".to_string());
        }
    };
    tree.0 = None;
    if !status.success() {
        return Err(format!("安装失败（{status}），请检查网关主机网络、安装权限及包管理器配置"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "install_tests.rs"]
mod tests;

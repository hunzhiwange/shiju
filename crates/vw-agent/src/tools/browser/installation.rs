//! 代理浏览器用户级安装与运行入口；固定包来源，不修改系统 PATH。
use crate::app::agent::shell::tokio_command;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

const PACKAGE: &str = "agent-browser@0.42.0";

fn install_root() -> Result<PathBuf, String> {
    let home = directories::BaseDirs::new().ok_or("无法确定用户主目录")?;
    Ok(vw_config_types::paths::home_config_dir(home.home_dir())
        .join("browser-runtime/agent-browser-0.42.0"))
}

fn entry(root: &Path) -> PathBuf {
    root.join("node_modules/agent-browser/bin/agent-browser.js")
}

pub(super) fn agent_browser_command() -> tokio::process::Command {
    if let Ok(root) = install_root()
        && root.join("ready").is_file()
        && entry(&root).is_file()
    {
        let mut command = tokio_command("node");
        command.arg(entry(&root));
        command
    } else {
        tokio_command("agent-browser")
    }
}

/// npm 的 JS 入口绕过 Windows 的批处理 shim，所有路径均按独立参数传递。
fn npm_entry() -> Result<PathBuf, String> {
    let npm = vw_shared::shell::resolve_executable("npm")
        .or_else(|| vw_shared::shell::resolve_executable("npm.cmd"))
        .ok_or("请先在网关主机安装 Node.js 22 或更高版本（含 npm），再重试")?;
    let npm = npm.canonicalize().map_err(|_| "无法定位 npm")?;
    if npm.extension().is_some_and(|ext| ext == "js") {
        return Ok(npm);
    }
    let script = npm.parent().ok_or("无法定位 npm 目录")?.join("node_modules/npm/bin/npm-cli.js");
    script
        .is_file()
        .then_some(script)
        .ok_or_else(|| "无法定位 npm-cli.js，请修复 Node.js/npm 安装".into())
}

struct ProcessTree(Option<u32>);
impl Drop for ProcessTree {
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
                .status();
        }
    }
}

async fn run(
    mut command: tokio::process::Command,
    deadline: tokio::time::Instant,
) -> Result<(), String> {
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| "无法启动安装命令，请检查 Node.js 与 npm")?;
    let mut tree = ProcessTree(child.id());
    let status = match tokio::time::timeout_at(deadline, child.wait()).await {
        Ok(result) => result.map_err(|_| "无法读取安装进程状态")?,
        Err(_) => {
            drop(tree);
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err("浏览器安装超时，请检查网关主机网络后重试".into());
        }
    };
    if !status.success() {
        return Err(format!("浏览器安装失败（{status}），请检查网络、磁盘空间及安装帮助"));
    }
    tree.0 = None;
    Ok(())
}

/// 在网关主机安装固定版本 CLI、下载 Chrome，再实际启动无头浏览器验证依赖。
pub(crate) async fn install() -> Result<(), String> {
    if !cfg!(any(target_os = "macos", target_os = "linux", target_os = "windows")) {
        return Err("当前系统不支持一键安装代理浏览器".into());
    }
    let root = install_root()?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(600);
    // 已成功安装时只重新验证，避免覆盖正在被工具使用的文件。
    if !root.join("ready").is_file() {
        let npm = npm_entry()?;
        let mut command = tokio_command("node");
        command.arg(npm).args(["install", "--prefix"]).arg(&root).args([
            "--no-audit",
            "--no-fund",
            "--engine-strict",
            PACKAGE,
        ]);
        run(command, deadline).await.map_err(|error| {
            format!("安装 agent-browser 失败：{error}；请确认 Node.js 22 或更高版本及 npm 可用")
        })?;
    }
    let mut command = tokio_command("node");
    command.arg(entry(&root)).arg("install");
    run(command, deadline).await.map_err(|error| format!("下载 Chrome 失败：{error}"))?;
    // Linux 系统库缺失时明确失败；不自动提权安装系统包。
    let session = format!("shiju-install-{}", std::process::id());
    let mut command = tokio_command("node");
    command.arg(entry(&root)).args(["--session", &session, "open", "about:blank"]);
    let probe = run(command, deadline).await;
    let mut close = tokio_command("node");
    close.arg(entry(&root)).args(["--session", &session, "close"]);
    let closed = run(close, tokio::time::Instant::now() + Duration::from_secs(15)).await;
    probe.map_err(|error| format!("{error}；Linux 请按帮助安装浏览器系统依赖"))?;
    closed?;
    tokio::fs::write(root.join("ready"), b"0.42.0").await.map_err(|_| "无法记录浏览器安装状态")?;
    Ok(())
}

#[cfg(test)]
#[path = "installation_tests.rs"]
mod tests;

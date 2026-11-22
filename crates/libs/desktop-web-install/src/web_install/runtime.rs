//! 管理从 Web 包启动的 Node 静态服务；仅允许回环地址和自动分配端口。
use anyhow::{Context, Result, bail};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

#[derive(Debug, Default)]
pub struct Runtime {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    process: Option<RunningWeb>,
    closed: bool,
}

#[derive(Debug)]
struct RunningWeb {
    child: Child,
    url: String,
}

impl Drop for RunningWeb {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn node_command(node: &Path) -> Command {
    let command = Command::new(node);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut command = command;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        command
    }
    #[cfg(not(windows))]
    command
}

fn supported_node_version(version: &str) -> bool {
    version
        .trim()
        .trim_start_matches('v')
        .split('.')
        .next()
        .and_then(|major| major.parse::<u32>().ok())
        .is_some_and(|major| major >= 22)
}

#[cfg(unix)]
pub(super) fn require_node() -> Result<PathBuf> {
    use app_discovery::posix::{PosixEnvironment, build_tool_search_paths, login_shell_path};
    use app_discovery::version::probe_local_version;
    use std::env;

    let path = env::var_os("PATH").unwrap_or_default();
    let environment = PosixEnvironment {
        home: directories::BaseDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_default(),
        path: Default::default(),
        login_path: None,
        variables: ["PNPM_HOME", "NPM_CONFIG_PREFIX"]
            .into_iter()
            .filter_map(|key| env::var_os(key).map(|value| (key.to_owned(), value)))
            .collect(),
    };
    let mut searched = std::collections::HashSet::new();
    let mut find_node = |dirs: &mut dyn Iterator<Item = PathBuf>| {
        dirs.filter(|dir| dir.is_absolute() && searched.insert(dir.clone()))
            .map(|dir| dir.join("node"))
            .find(|candidate| {
                probe_local_version(candidate, Duration::from_secs(2))
                    .ok()
                    .and_then(|probe| probe.version)
                    .is_some_and(|version| supported_node_version(&version))
            })
    };

    if let Some(node) = find_node(&mut env::split_paths(&path))
        .or_else(|| find_node(&mut build_tool_search_paths("node", &environment).into_iter()))
    {
        return Ok(node);
    }
    if let Some(path) = env::var_os("SHELL")
        .and_then(|shell| login_shell_path(Path::new(&shell), Duration::from_secs(2)).ok())
        && let Some(node) = find_node(&mut env::split_paths(&path))
    {
        return Ok(node);
    }
    bail!("未找到可用的 Node.js 22 或以上版本，请检查 Node.js 版本及安装路径");
}

#[cfg(not(unix))]
pub(super) fn require_node() -> Result<PathBuf> {
    let node = PathBuf::from("node");
    let output = node_command(&node)
        .arg("--version")
        .output()
        .context("请先安装 Node.js 22 或以上版本，并确保 node 位于 PATH 中")?;
    if !output.status.success() || !supported_node_version(&String::from_utf8_lossy(&output.stdout)) {
        bail!("请先安装 Node.js 22 或以上版本，并确保 node 位于 PATH 中");
    }
    Ok(node)
}

impl Runtime {
    pub(super) fn url(&self) -> Result<Option<String>> {
        let mut inner = self.inner.lock().map_err(|_| anyhow::anyhow!("Web 服务状态锁异常"))?;
        if let Some(process) = inner.process.as_mut() {
            if process.child.try_wait()?.is_none() {
                return Ok(Some(process.url.clone()));
            }
            inner.process = None;
        }
        Ok(None)
    }

    pub(super) fn start(&self, path: &Path, node: Option<PathBuf>) -> Result<String> {
        {
            let mut inner = self.inner.lock().map_err(|_| anyhow::anyhow!("Web 服务状态锁异常"))?;
            if inner.closed {
                bail!("应用正在退出，无法启动 Web 服务");
            }
            if let Some(process) = inner.process.as_mut() {
                if process.child.try_wait()?.is_none() {
                    return Ok(process.url.clone());
                }
                inner.process = None;
            }
        }
        let node = match node {
            Some(node) => node,
            None => require_node()?,
        };
        super::installer::validate_package(path)?;
        let child = node_command(&node)
            .arg(path.join("serve.mjs"))
            .args(["--port", "0"])
            .current_dir(path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("启动 Web 服务失败")?;
        let mut process = RunningWeb { child, url: String::new() };
        let stdout = process.child.stdout.take().context("无法读取 Web 服务启动信息")?;
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        // 等待就绪期间不持锁，退出应用仍可立即终止子进程。
        {
            let mut inner = self.inner.lock().map_err(|_| anyhow::anyhow!("Web 服务状态锁异常"))?;
            if inner.closed {
                bail!("应用正在退出，无法启动 Web 服务");
            }
            inner.process = Some(process);
        }
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout.take(4096)).read_line(&mut line).map(|_| line);
            let _ = sender.send(result);
        });
        let ready = (|| -> Result<String> {
            let line = receiver
                .recv_timeout(Duration::from_secs(10))
                .context("等待 Web 服务启动超时，请重试")??;
            let url = line.trim().strip_prefix("ShiJu Web: ").context("Web 服务启动响应无效")?;
            let port = url
                .strip_prefix("http://127.0.0.1:")
                .and_then(|v| v.strip_suffix('/'))
                .and_then(|v| v.parse::<u16>().ok())
                .filter(|port| *port != 0)
                .context("Web 服务必须监听本机回环地址")?;
            let mut inner = self.inner.lock().map_err(|_| anyhow::anyhow!("Web 服务状态锁异常"))?;
            let process = inner.process.as_mut().context("Web 服务已停止")?;
            if process.child.try_wait()?.is_some() {
                bail!("Web 服务启动后意外退出");
            }
            process.url = format!("http://127.0.0.1:{port}/");
            Ok(process.url.clone())
        })();
        if ready.is_err() {
            self.stop().context("Web 服务启动失败，清理子进程失败")?;
        }
        ready
    }

    pub(super) fn stop(&self) -> Result<()> {
        let mut inner = self.inner.lock().map_err(|_| anyhow::anyhow!("Web 服务状态锁异常"))?;
        if let Some(process) = inner.process.as_mut() {
            if process.child.try_wait()?.is_none() {
                process.child.kill().context("停止 Web 服务失败")?;
            }
            process.child.wait()?;
        }
        inner.process = None;
        Ok(())
    }

    pub fn shutdown(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.closed = true;
            inner.process = None;
        }
    }
}

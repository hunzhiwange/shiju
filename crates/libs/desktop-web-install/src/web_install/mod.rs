//! 独立 Web 包的安装与本机服务状态，不参与桌面安装包构建。
mod installer;
mod runtime;

use anyhow::{Context, Result, bail};
use iced::Task;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use vw_shared::update::UpdateProgress;

#[derive(Debug, Default)]
pub struct State {
    pub progress: Option<UpdateProgress>,
    pub progress_rx: Option<mpsc::Receiver<Event>>,
    pub visible: bool,
    pub busy: bool,
    pub status: String,
    pub path: Option<PathBuf>,
    pub url: Option<String>,
    pub runtime: Arc<runtime::Runtime>,
}

#[derive(Debug, Clone)]
pub enum Event {
    ProgressTick,
    Progress(UpdateProgress),
    Status(&'static str),
    Open,
    Close,
    InstallAndStart,
    OpenDirectory,
    Start,
    Stop,
    Finished(Snapshot),
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    path: Option<PathBuf>,
    url: Option<String>,
    error: Option<String>,
}

/// Updates local installation state; the caller displays the returned error.
pub fn update(
    state: &mut State,
    event: Event,
    source: Option<vw_shared::update::UpdateSource>,
    shutdown_requested: bool,
) -> (Task<Event>, Option<String>) {
    match event {
        Event::ProgressTick => {
            if let Some(receiver) = &state.progress_rx {
                while let Ok(event) = receiver.try_recv() {
                    match event {
                        Event::Progress(progress) => state.progress = Some(progress),
                        Event::Status(status) => state.status = status.into(),
                        _ => {}
                    }
                }
            }
            return (Task::none(), None);
        }
        Event::Progress(_) | Event::Status(_) => return (Task::none(), None),
        Event::Close => {
            state.visible = false;
            return (Task::none(), None);
        }
        Event::Finished(snapshot) => {
            state.busy = false;
            state.progress_rx = None;
            state.progress = None;
            state.path = snapshot.path;
            state.url = snapshot.url;
            state.status = snapshot.error.clone().unwrap_or_default();
            return (Task::none(), snapshot.error);
        }
        Event::Open => {
            state.visible = true;
        }
        _ => {}
    }
    if state.busy || shutdown_requested {
        return (Task::none(), None);
    }
    state.busy = true;
    state.progress = None;
    let (progress_tx, progress_rx) = mpsc::channel();
    state.progress_rx = Some(progress_rx);
    state.status = match event {
        Event::InstallAndStart => "正在从所选来源下载并安装 Web 版本…",
        Event::OpenDirectory => "正在打开 Web 目录…",
        Event::Start => "正在启动 Web 服务…",
        Event::Stop => "正在停止 Web 服务…",
        _ => "正在读取 Web 安装状态…",
    }
    .into();
    let runtime = state.runtime.clone();
    let task = Task::perform(
        async move {
            let fallback = runtime.clone();
            match tokio::task::spawn_blocking(move || {
                let error = operate(event, &runtime, source, &progress_tx)
                    .err()
                    .map(|error| format!("{error:#}"));
                Snapshot {
                    path: installer::installed().ok().flatten(),
                    url: runtime.url().ok().flatten(),
                    error,
                }
            })
            .await
            {
                Ok(snapshot) => snapshot,
                Err(error) => Snapshot {
                    path: None,
                    url: fallback.url().ok().flatten(),
                    error: Some(format!("Web 操作任务失败: {error}")),
                },
            }
        },
        Event::Finished,
    );
    (task, None)
}

fn operate(
    event: Event,
    runtime: &runtime::Runtime,
    source: Option<vw_shared::update::UpdateSource>,
    progress_tx: &mpsc::Sender<Event>,
) -> Result<()> {
    // 安装失败后也能停止既有服务；读到损坏安装时保留明确错误。
    if matches!(event, Event::Stop) {
        runtime.stop()?;
    }
    if matches!(event, Event::InstallAndStart) {
        if runtime.url()?.is_some() {
            bail!("请先停止 Web 服务，再安装或更新");
        }
        runtime::require_node()?;
        let path = installer::install(source.context("请先选择有效的升级来源")?, progress_tx)?;
        let _ = progress_tx.send(Event::Status("正在启动 Web 服务…"));
        let url = runtime.start(&path)?;
        open::that(&url).context("Web 已启动，打开浏览器失败；请使用弹窗中的访问地址")?;
    } else if matches!(event, Event::OpenDirectory) {
        let path = installer::installed()?.context("请先下载安装 Web 版本")?;
        #[cfg(target_os = "macos")]
        {
            let status = std::process::Command::new("/usr/bin/open")
                .arg("-R")
                .arg(&path)
                .status()
                .context("在 Finder 中定位 Web 目录失败")?;
            if !status.success() {
                bail!("在 Finder 中定位 Web 目录失败");
            }
        }
        #[cfg(not(target_os = "macos"))]
        open::that(&path).context("打开 Web 目录失败")?;
    } else if matches!(event, Event::Start) {
        let path = installer::installed()?.context("请先下载安装 Web 版本")?;
        let url = runtime.start(&path)?;
        open::that(&url).context("Web 已启动，打开浏览器失败；请使用弹窗中的访问地址")?;
    }
    installer::installed()?;
    Ok(())
}

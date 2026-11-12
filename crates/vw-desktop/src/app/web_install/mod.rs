//! 独立 Web 包的安装与本机服务状态，不参与桌面安装包构建。
mod installer;
mod runtime;

use crate::app::message::ViewMessage;
use crate::app::{App, Message};
use anyhow::{Context, Result, bail};
use iced::Task;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Default)]
pub(crate) struct State {
    pub(crate) visible: bool,
    pub(crate) busy: bool,
    pub(crate) status: String,
    pub(crate) path: Option<PathBuf>,
    pub(crate) url: Option<String>,
    pub(crate) runtime: Arc<runtime::Runtime>,
}

#[derive(Debug, Clone)]
pub enum Event {
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

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let source = app.desktop_update.source;
    let state = &mut app.web_install;
    match event {
        Event::Close => {
            state.visible = false;
            return Task::none();
        }
        Event::Finished(snapshot) => {
            state.busy = false;
            state.path = snapshot.path;
            state.url = snapshot.url;
            state.status = snapshot.error.clone().unwrap_or_default();
            return match snapshot.error {
                Some(error) => app.show_error_toast(error),
                None => Task::none(),
            };
        }
        Event::Open => {
            state.visible = true;
            app.active_menu = None;
        }
        _ => {}
    }
    if state.busy || app.shutdown_requested {
        return Task::none();
    }
    state.busy = true;
    state.status = match event {
        Event::InstallAndStart => "正在从所选来源下载并安装 Web 版本…",
        Event::OpenDirectory => "正在打开 Web 目录…",
        Event::Start => "正在启动 Web 服务…",
        Event::Stop => "正在停止 Web 服务…",
        _ => "正在读取 Web 安装状态…",
    }
    .into();
    let runtime = state.runtime.clone();
    Task::perform(
        async move {
            let fallback = runtime.clone();
            match tokio::task::spawn_blocking(move || {
                let error =
                    operate(event, &runtime, source).err().map(|error| format!("{error:#}"));
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
        |result| Message::View(ViewMessage::WebInstall(Event::Finished(result))),
    )
}

fn operate(
    event: Event,
    runtime: &runtime::Runtime,
    source: Option<vw_shared::update::UpdateSource>,
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
        let path = installer::install(source.context("请先选择有效的升级来源")?)?;
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

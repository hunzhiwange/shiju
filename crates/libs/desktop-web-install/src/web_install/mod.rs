//! 独立 Web 包的安装与本机服务状态，不参与桌面安装包构建。
mod installer;
mod copy;
mod runtime;

use anyhow::Result;
use self::copy::ProductContext as _;
use iced::Task;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use update_types::UpdateProgress;

#[derive(Debug, Default)]
pub struct State {
    pub progress: Option<UpdateProgress>,
    pub progress_rx: Option<mpsc::Receiver<Event>>,
    pub visible: bool,
    pub busy: bool,
    pub status: String,
    pub operation_error: Option<Arc<anyhow::Error>>,
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
    error: Option<Arc<anyhow::Error>>,
}

/// Updates local installation state; the caller displays the returned error.
pub fn update(
    state: &mut State,
    event: Event,
    source: Option<desktop_release::UpdateSource>,
    shutdown_requested: bool,
) -> (Task<Event>, Option<desktop_i18n::format::Message>) {
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
            state.status = snapshot.error.as_ref().map(|error| format!("{error:#}")).unwrap_or_default();
            let feedback = snapshot.error.as_ref().map(|error| copy::message(error));
            state.operation_error = snapshot.error;
            return (Task::none(), feedback);
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
    state.operation_error = None;
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
                    .map(Arc::new);
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
                    error: Some(Arc::new(copy::error(desktop_i18n::format::Message::new("Web 操作任务失败: %s", vec![error.to_string().into()])))),
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
    source: Option<desktop_release::UpdateSource>,
    progress_tx: &mpsc::Sender<Event>,
) -> Result<()> {
    // 安装失败后也能停止既有服务；读到损坏安装时保留明确错误。
    if matches!(event, Event::Stop) {
        runtime.stop()?;
    }
    if matches!(event, Event::InstallAndStart) {
        if runtime.url()?.is_some() {
            return Err(copy::error(desktop_i18n::format::Message::new("请先停止 Web 服务，再安装或更新", vec![])));
        }
        let node = runtime::require_node()?;
        let path = installer::install(source.product_context("请先选择有效的升级来源")?, progress_tx)?;
        let _ = progress_tx.send(Event::Status("正在启动 Web 服务…"));
        let url = runtime.start(&path, Some(node))?;
        open::that(&url).product_context("Web 已启动，打开浏览器失败；请使用弹窗中的访问地址")?;
    } else if matches!(event, Event::OpenDirectory) {
        let path = installer::installed()?.product_context("请先下载安装 Web 版本")?;
        #[cfg(target_os = "macos")]
        {
            let status = std::process::Command::new("/usr/bin/open")
                .arg("-R")
                .arg(&path)
                .status()
                .product_context("在 Finder 中定位 Web 目录失败")?;
            if !status.success() {
                return Err(copy::error(desktop_i18n::format::Message::new("在 Finder 中定位 Web 目录失败", vec![])));
            }
        }
        #[cfg(not(target_os = "macos"))]
        open::that(&path).product_context("打开 Web 目录失败")?;
    } else if matches!(event, Event::Start) {
        let path = installer::installed()?.product_context("请先下载安装 Web 版本")?;
        let url = runtime.start(&path, None)?;
        open::that(&url).product_context("Web 已启动，打开浏览器失败；请使用弹窗中的访问地址")?;
    }
    installer::installed()?;
    Ok(())
}

impl State {
    pub fn rendered_status(&self, i18n: desktop_i18n::I18n) -> String {
        self.operation_error.as_ref()
            .map(|error| copy::message(error).render(i18n).unwrap_or_else(|_| self.status.clone()))
            .unwrap_or_else(|| i18n.__(&self.status).to_owned())
    }
}

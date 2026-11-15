//! Update modal state transitions, release checks and download lifecycle.
use iced::Task;
use crate::app::desktop_update::DesktopUpdate;
use crate::cli_install::{install_cli_tool, current_cli_tool_version_label, versions_match};
#[cfg(not(target_arch = "wasm32"))]
use desktop_release as update;
#[derive(Debug, Clone)]
pub enum PanelMessage {
    InstallCliTool,
    RunInstallCliTool,
    CheckCliToolUpdate,
    CheckCliToolUpdateFinished(u64, desktop_release::UpdateSource, Result<String, String>),
    OpenAppUpdateModal,
    AppUpdateSourceSelected(desktop_release::UpdateSource),
    AppUpdateSourceSaved(Result<(), String>),
    CheckAppUpdate,
    CheckAppUpdateFinished(
        u64,
        desktop_release::UpdateSource,
        Result<desktop_release::DesktopRelease, String>,
    ),
    RunAppUpdate,
    CancelAppUpdate,
    #[cfg(not(target_arch = "wasm32"))]
    AppUpdateDownloadPrepared(u64, Result<crate::app::installer_update::PreparedDesktopUpdate, String>),
    #[cfg(not(target_arch = "wasm32"))]
    AppUpdatePrepared(Result<crate::app::installer_update::PreparedDesktopUpdate, String>),
    OpenAppInstaller,
    OpenAppInstallerDirectory,
    AppInstallerDirectoryOpened(Result<(), String>),
    AppUpdateProgressTick,
    AppUpdateFinished(Result<String, String>),
    InstallCliToolFinished(Result<String, String>),
    CloseInstallCliModal,
}
/// Update-modal projection transferred by value by the desktop shell.
#[derive(Default)]
pub struct PanelState {
    pub show_cli_install_modal: bool,
    pub cli_install_modal_title: String,
    pub cli_install_modal_message: String,
    pub cli_install_modal_current_version: String,
    pub cli_install_modal_server_version: String,
    pub cli_install_modal_show_update_action: bool,
    pub cli_install_modal_show_install_action: bool,
    pub cli_install_modal_use_app_update_action: bool,
    pub cli_install_modal_is_checking_update: bool,
    pub app_update_progress: Option<update_types::UpdateProgress>,
    pub app_update_progress_rx: Option<std::sync::mpsc::Receiver<update_types::UpdateProgress>>,
}
pub enum Effect {
    CloseMenu,
    SaveSource(desktop_release::UpdateSource),
    Error(String),
    Shutdown,
}
fn show_error_toast(effects: &mut Vec<Effect>, error: impl Into<String>) -> Task<PanelMessage> {
    effects.push(Effect::Error(error.into()));
    Task::none()
}
pub fn update(
    panel: &mut PanelState,
    state: &mut DesktopUpdate,
    effects: &mut Vec<Effect>,
    version: &'static str,
    #[cfg(not(target_arch = "wasm32"))] locale: config_ui::DesktopLocale,
    message: PanelMessage,
) -> Task<PanelMessage> {
    match message {
        PanelMessage::InstallCliTool => {
            if state.busy {
                return Task::none();
            }
            state.request += 1;
            state.checking = false;
            effects.push(Effect::CloseMenu);
            panel.show_cli_install_modal = true;
            panel.cli_install_modal_title = "CLI 更新检测".to_string();
            panel.cli_install_modal_message =
                "打开终端，通过 shiju 调用各项功能：shiju agent 启动 TUI，shiju service 管理服务。下方命令可直接复制。"
                    .to_string();
            panel.cli_install_modal_current_version = current_cli_tool_version_label();
            panel.cli_install_modal_server_version = "未检测".to_string();
            panel.cli_install_modal_show_update_action = true;
            panel.cli_install_modal_show_install_action = true;
            panel.cli_install_modal_use_app_update_action = false;
            panel.cli_install_modal_is_checking_update = false;
            update(panel, state, effects, version, #[cfg(not(target_arch = "wasm32"))] locale, PanelMessage::CheckCliToolUpdate)
        }
        PanelMessage::RunInstallCliTool => {
            panel.cli_install_modal_is_checking_update = false;
            Task::perform(async { install_cli_tool() }, |res| {
                PanelMessage::InstallCliToolFinished(res)
            })
        }
        PanelMessage::CheckCliToolUpdate => {
            if state.busy || state.checking {
                return Task::none();
            }
            let Some(source) = state.source else {
                return show_error_toast(effects, "请先选择有效的升级来源");
            };
            state.request += 1;
            let request = state.request;
            state.checking = true;
            panel.cli_install_modal_is_checking_update = true;
            panel.cli_install_modal_message = "正在请求所选来源的最新 CLI 版本...".into();
            Task::perform(fetch_latest_release_version(source), move |res| {
                PanelMessage::CheckCliToolUpdateFinished(request, source, res)
            })
        }
        PanelMessage::CheckCliToolUpdateFinished(request, source, res) => {
            if request != state.request
                || Some(source) != state.source
                || !state.checking
            {
                return Task::none();
            }
            state.checking = false;
            panel.cli_install_modal_is_checking_update = false;
            match res {
                Ok(version) => {
                    panel.cli_install_modal_server_version = version.clone();
                    if versions_match(&panel.cli_install_modal_current_version, &version) {
                        panel.cli_install_modal_message =
                            "检测完成，本地 CLI 已是最新版本。".to_string();
                    } else {
                        panel.cli_install_modal_message =
                            "检测完成，发现新的 CLI 版本，可直接安装更新。".to_string();
                    }
                }
                Err(error) => {
                    panel.cli_install_modal_server_version = "检测失败".to_string();
                    panel.cli_install_modal_message = error;
                }
            }
            Task::none()
        }
        PanelMessage::OpenAppUpdateModal => {
            effects.push(Effect::CloseMenu);
            panel.show_cli_install_modal = true;
            if panel.cli_install_modal_use_app_update_action {
                #[cfg(not(target_arch = "wasm32"))]
                if state.ready.is_some() {
                    return Task::none();
                }
                return update(panel, state, effects, version, #[cfg(not(target_arch = "wasm32"))] locale, PanelMessage::CheckAppUpdate);
            }
            state.request += 1;
            state.checking = false;
            panel.cli_install_modal_title = "检测更新".into();
            panel.cli_install_modal_message = state
                .error
                .clone()
                .unwrap_or_else(|| "请选择来源并检测最新发布版本。".into());
            panel.cli_install_modal_current_version = current_app_version_label(version);
            panel.cli_install_modal_server_version = "未检测".into();
            panel.cli_install_modal_show_update_action = true;
            panel.cli_install_modal_show_install_action = false;
            panel.cli_install_modal_use_app_update_action = true;
            panel.cli_install_modal_is_checking_update =
                state.checking || state.busy;
            if let Some(release) = &state.release {
                panel.cli_install_modal_server_version = release.version.clone();
                panel.cli_install_modal_show_install_action = state.installable();
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(ready) = &state.ready {
                panel.cli_install_modal_title = "安装包已就绪".into();
                panel.cli_install_modal_message = ready.instructions();
                panel.cli_install_modal_show_install_action = false;
                return Task::none();
            }
            update(panel, state, effects, version, #[cfg(not(target_arch = "wasm32"))] locale, PanelMessage::CheckAppUpdate)
        }
        PanelMessage::AppUpdateSourceSelected(source) => {
            if state.busy || state.saving {
                return Task::none();
            }
            state.source = Some(source);
            state.error = None;
            state.selected_by_user = true;
            state.saving = true;
            state.request += 1;
            state.checking = false;
            state.release = None;
            #[cfg(not(target_arch = "wasm32"))]
            {
                state.ready = None;
            }
            panel.app_update_progress = None;
            panel.app_update_progress_rx = None;
            panel.cli_install_modal_server_version = "未检测".into();
            panel.cli_install_modal_show_install_action =
                !panel.cli_install_modal_use_app_update_action;
            panel.cli_install_modal_is_checking_update = false;
            panel.cli_install_modal_message = "来源已切换，请重新检测更新。".into();
            effects.push(Effect::SaveSource(source));
            Task::none()
        }
        PanelMessage::AppUpdateSourceSaved(result) => {
            state.saving = false;
            match result {
                Ok(()) => Task::none(),
                Err(error) => {
                    show_error_toast(effects, format!("升级来源未保存，本轮选择仍有效：{error}"))
                }
            }
        }
        PanelMessage::CheckAppUpdate => {
            if state.busy || state.checking {
                return Task::none();
            }
            let Some(source) = state.source else {
                return show_error_toast(effects, "请先选择有效的升级来源");
            };
            state.request += 1;
            let request = state.request;
            state.checking = true;
            state.release = None;
            #[cfg(not(target_arch = "wasm32"))]
            {
                state.ready = None;
            }
            panel.cli_install_modal_is_checking_update = true;
            panel.cli_install_modal_show_install_action = false;
            panel.cli_install_modal_message = "正在请求所选来源的最新应用版本...".into();
            panel.app_update_progress = None;
            panel.app_update_progress_rx = None;
            Task::perform(fetch_latest_app_release(source), move |res| {
                PanelMessage::CheckAppUpdateFinished(request, source, res)
            })
        }
        PanelMessage::CheckAppUpdateFinished(request, source, res) => {
            if request != state.request
                || Some(source) != state.source
                || !state.checking
            {
                return Task::none();
            }
            state.checking = false;
            panel.cli_install_modal_is_checking_update = false;
            match res {
                Ok(release) if release.source == source => {
                    panel.cli_install_modal_server_version = release.version.clone();
                    let newer = release.is_newer_than(version);
                    match newer {
                        Ok(newer) => {
                            panel.cli_install_modal_show_install_action = newer;
                            panel.cli_install_modal_message = if newer {
                                "检测完成，发现新版本，可下载安装包后通过系统安装器安装。"
                            } else {
                                "检测完成，当前应用已是最新版本或高于所选来源版本。"
                            }
                            .into();
                            state.release = Some(release);
                        }
                        Err(error) => return app_update_error(panel, state, effects, error.to_string()),
                    }
                }
                Ok(_) => return app_update_error(panel, state, effects, "发布结果与所选来源不一致".into()),
                Err(error) => return app_update_error(panel, state, effects, error),
            }
            Task::none()
        }
        PanelMessage::RunAppUpdate => {
            if state.busy
                || state.checking
                || !state.installable()
            {
                return Task::none();
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let release = state.release.clone().expect("checked snapshot");
                let (progress_tx, progress_rx) = std::sync::mpsc::channel();
                state.busy = true;
                panel.cli_install_modal_is_checking_update = true;
                panel.cli_install_modal_show_install_action = false;
                panel.cli_install_modal_message = "正在下载并校验安装包...".into();
                panel.app_update_progress = None;
                panel.app_update_progress_rx = Some(progress_rx);
                state.request += 1;
                let request = state.request;
                let (task, handle) = Task::perform(
                    async move {
                        crate::app::installer_update::prepare_desktop_update(&release, progress_tx)
                            .await
                            .map_err(|error| error.to_string())
                    },
                    move |result| PanelMessage::AppUpdateDownloadPrepared(request, result),
                ).abortable();
                state.download_abort = Some(handle);
                task
            }
            #[cfg(target_arch = "wasm32")]
            {
                app_update_error(panel, state, effects, "当前平台不支持应用更新".into())
            }
        }
        PanelMessage::CancelAppUpdate => {
            // Installation is irreversible here; only an active download is cancellable.
            let Some(handle) = state.download_abort.take() else {
                return Task::none();
            };
            handle.abort();
            state.request += 1;
            state.busy = false;
            panel.cli_install_modal_is_checking_update = false;
            panel.cli_install_modal_show_install_action = state.installable();
            panel.cli_install_modal_message = "下载已取消，可重新下载安装包。".into();
            panel.app_update_progress_rx = None;
            panel.app_update_progress = None;
            Task::none()
        }
        #[cfg(not(target_arch = "wasm32"))]
        PanelMessage::AppUpdateDownloadPrepared(request, result) => {
            if request != state.request || state.download_abort.is_none() {
                return Task::none();
            }
            update(panel, state, effects, version, #[cfg(not(target_arch = "wasm32"))] locale, PanelMessage::AppUpdatePrepared(result))
        }
        #[cfg(not(target_arch = "wasm32"))]
        PanelMessage::AppUpdatePrepared(result) => {
            if !state.busy {
                return Task::none();
            }
            state.download_abort = None;
            state.busy = false;
            panel.cli_install_modal_is_checking_update = false;
            // The completion message may arrive before the next progress subscription tick.
            // The producer has finished, so retain its final measured byte count before closing.
            if let Some(receiver) = panel.app_update_progress_rx.take() {
                if let Some(progress) = receiver.try_iter().last() {
                    panel.app_update_progress = Some(progress);
                }
            }
            match result {
                Ok(ready) => {
                    panel.cli_install_modal_title = "安装包已就绪".into();
                    panel.cli_install_modal_message = ready.instructions();
                    state.ready = Some(ready);
                    Task::none()
                }
                Err(error) => app_update_error(panel, state, effects, error),
            }
        }
        PanelMessage::OpenAppInstaller => {
            if state.busy {
                return Task::none();
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(ready) = state.ready.clone() {
                panel.cli_install_modal_title = "更新中…".into();
                panel.cli_install_modal_message =
                    "正在校验安装包并启动自动更新，请稍候；准备就绪后应用将自动退出。".into();
                state.busy = true;
                panel.cli_install_modal_is_checking_update = true;
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::app::installer_update::open_desktop_installer(&ready, locale)
                        })
                        .await
                        .map_err(|error| error.to_string())?
                        .map_err(|error| format!("{error:#}"))
                    },
                    |result| PanelMessage::AppUpdateFinished(result),
                );
            }
            Task::none()
        }
        PanelMessage::OpenAppInstallerDirectory => {
            if state.busy {
                return Task::none();
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(ready) = state.ready.clone() {
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::app::installer_update::reveal_desktop_installer(&ready)
                        })
                        .await
                        .map_err(|error| error.to_string())?
                        .map_err(|error| format!("{error:#}"))
                    },
                    |result| PanelMessage::AppInstallerDirectoryOpened(result),
                );
            }
            Task::none()
        }
        PanelMessage::AppInstallerDirectoryOpened(result) => match result {
            Ok(()) => Task::none(),
            Err(error) => show_error_toast(effects,error),
        },
        PanelMessage::AppUpdateProgressTick => {
            if let Some(receiver) = panel.app_update_progress_rx.take() {
                let mut active_receiver = Some(receiver);
                for _ in 0..48 {
                    let Some(current_receiver) = active_receiver.as_ref() else {
                        break;
                    };
                    match current_receiver.try_recv() {
                        Ok(progress) => panel.app_update_progress = Some(progress),
                        Err(std::sync::mpsc::TryRecvError::Empty) => break,
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            active_receiver = None;
                            break;
                        }
                    }
                }
                panel.app_update_progress_rx = active_receiver;
            }
            Task::none()
        }
        PanelMessage::AppUpdateFinished(res) => {
            if !state.busy {
                return Task::none();
            }
            state.busy = false;
            panel.cli_install_modal_is_checking_update = false;
            panel.app_update_progress_rx = None;
            match res {
                Ok(message) => {
                    panel.cli_install_modal_title = "安装包已就绪".into();
                    panel.cli_install_modal_message = message;
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        effects.push(Effect::Shutdown);
                        Task::none()
                    }
                    #[cfg(target_arch = "wasm32")]
                    Task::none()
                }
                Err(error) => app_update_error(panel, state, effects, error),
            }
        }
        PanelMessage::InstallCliToolFinished(res) => {
            panel.show_cli_install_modal = true;
            reset_cli_update_modal_state(panel);
            match res {
                Ok(message) => {
                    panel.cli_install_modal_title = "CLI 安装完成".to_string();
                    panel.cli_install_modal_message = message;
                }
                Err(error) => {
                    panel.cli_install_modal_title = "CLI 安装失败".to_string();
                    panel.cli_install_modal_message = error;
                }
            }
            Task::none()
        }
        PanelMessage::CloseInstallCliModal => {
            panel.show_cli_install_modal = false;
            if !panel.cli_install_modal_use_app_update_action {
                state.request += 1;
                state.checking = false;
                reset_cli_update_modal_state(panel);
            }
            Task::none()
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
async fn fetch_latest_release_version(source: desktop_release::UpdateSource) -> Result<String, String> {
    fetch_latest_app_release(source).await.map(|release| release.version)
}

#[cfg(target_arch = "wasm32")]
async fn fetch_latest_release_version(
    _source: desktop_release::UpdateSource,
) -> Result<String, String> {
    Err("当前平台不支持检测更新".to_string())
}

fn app_update_error(panel: &mut PanelState, state: &mut DesktopUpdate, effects: &mut Vec<Effect>, error: String) -> Task<PanelMessage> {
    panel.cli_install_modal_show_install_action = state.installable();
    panel.cli_install_modal_message = error.clone();
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ready) = &state.ready {
        panel.cli_install_modal_title = "自动更新启动失败".into();
        panel.cli_install_modal_show_install_action = false;
        panel.cli_install_modal_message = format!("{error}\n{}", ready.instructions());
    }
    show_error_toast(effects, error)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_latest_app_release(
    source: update::UpdateSource,
) -> Result<update::DesktopRelease, String> {
    update::fetch_desktop_release(source).await.map_err(|e| format!("请求应用版本失败: {e}"))
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_latest_app_release(
    _source: desktop_release::UpdateSource,
) -> Result<desktop_release::DesktopRelease, String> {
    Err("当前平台不支持检测应用更新".into())
}

fn reset_cli_update_modal_state(panel: &mut PanelState) {
    panel.cli_install_modal_current_version.clear();
    panel.cli_install_modal_server_version.clear();
    panel.cli_install_modal_show_update_action = false;
    panel.cli_install_modal_show_install_action = false;
    panel.cli_install_modal_use_app_update_action = false;
    panel.cli_install_modal_is_checking_update = false;
    panel.app_update_progress = None;
    panel.app_update_progress_rx = None;
}

#[cfg(not(target_arch = "wasm32"))]
fn current_app_version_label(version: &str) -> String {
    format!("v{version}")
}

#[cfg(target_arch = "wasm32")]
fn current_app_version_label(_version: &str) -> String {
    "当前平台不支持".to_string()
}


#[cfg(test)]
#[path = "panel_tests.rs"]
mod panel_tests;

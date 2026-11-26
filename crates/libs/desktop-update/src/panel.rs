//! Update modal state transitions, release checks and download lifecycle.
use crate::app::desktop_update::DesktopUpdate;
use crate::automatic::{CONFIG_KEY, DownloadPolicy};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::installer_update::PreparedDesktopUpdate;
use crate::cli_install::{current_cli_tool_version_label, install_cli_tool, versions_match};
#[cfg(not(target_arch = "wasm32"))]
use desktop_release as update;
use iced::Task;
#[derive(Debug, Clone)]
pub enum PanelMessage {
    InstallCliTool,
    RunInstallCliTool,
    CheckCliToolUpdate,
    CheckCliToolUpdateFinished(u64, desktop_release::UpdateSource, Result<String, String>),
    OpenAppUpdateModal,
    DownloadPolicyChanged(DownloadPolicy),
    DownloadPolicySaved(Result<(), String>),
    #[cfg(not(target_arch = "wasm32"))]
    AutomaticDownloadTick(u32),
    #[cfg(not(target_arch = "wasm32"))]
    AutomaticConditionsChecked(u64, DownloadPolicy, bool),
    #[cfg(not(target_arch = "wasm32"))]
    CachedInstallerChecked(u64, Option<PreparedDesktopUpdate>),
    AppUpdateSourceSelected(desktop_release::UpdateSource),
    AppUpdateSourceSaved(Result<(), String>),
    CheckAppUpdate,
    CheckAppUpdateFinished(
        u64,
        desktop_release::UpdateSource,
        Result<desktop_release::DesktopRelease, String>,
    ),
    RunAppUpdate,
    #[cfg(not(target_arch = "wasm32"))]
    SetAppDownloadPaused(bool),
    CancelAppUpdate,
    #[cfg(not(target_arch = "wasm32"))]
    AppUpdateDownloadPrepared(u64, Result<crate::app::installer_update::PreparedDesktopUpdate, String>),
    #[cfg(not(target_arch = "wasm32"))]
    AppUpdatePrepared(Result<crate::app::installer_update::PreparedDesktopUpdate, String>),
    OpenAppInstaller,
    #[cfg(not(target_arch = "wasm32"))]
    RequestAppInstallerConfirmation,
    #[cfg(not(target_arch = "wasm32"))]
    ResolveAppInstallerConfirmation { install: bool },
    OpenAppInstallerDirectory,
    AppInstallerDirectoryOpened(Result<(), String>),
    AppUpdateProgressTick,
    AppUpdateFinished(Result<String, String>),
    InstallCliToolFinished(Result<desktop_i18n::format::Message, crate::cli_install::InstallError>),
    CloseInstallCliModal,
}
/// Update-modal projection transferred by value by the desktop shell.
#[derive(Default)]
pub struct PanelState {
    #[cfg(not(target_arch = "wasm32"))]
    pub app_installer_confirmation_request: Option<u64>,
    pub show_cli_install_modal: bool,
    pub cli_install_modal_title: String,
    pub cli_install_modal_message: String,
    pub cli_install_message: Option<desktop_i18n::format::Message>,
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
    SaveDownloadPolicy(serde_json::Value),
    Error(String),
    LocalizedError(&'static str, Vec<desktop_i18n::format::Argument>),
    Shutdown,
}
fn localized_error(
    effects: &mut Vec<Effect>,
    source: &'static str,
    arguments: Vec<desktop_i18n::format::Argument>,
) -> Task<PanelMessage> {
    effects.push(Effect::LocalizedError(source, arguments));
    Task::none()
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
        PanelMessage::DownloadPolicyChanged(policy) => {
            if state.saving_download_policy || state.download_policy == policy {
                return Task::none();
            }
            state.set_download_policy(policy);
            state.saving_download_policy = true;
            effects.push(Effect::SaveDownloadPolicy(serde_json::json!({(CONFIG_KEY): policy})));
            #[cfg(not(target_arch = "wasm32"))]
            {
                state.automatic_retry_after = None;
                if state.automatic_download_active {
                    cancel_download(panel, state);
                }
            }
            Task::none()
        }
        PanelMessage::DownloadPolicySaved(result) => {
            state.saving_download_policy = false;
            match result {
                Ok(()) => Task::none(),
                Err(error) => localized_error(
                    effects,
                    "自动下载设置未保存：%s",
                    vec![error.into()],
                ),
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        PanelMessage::AutomaticDownloadTick(hour) => {
            if state.automatic_probe_pending || !state.download_policy.enabled
                || (!state.automatic_download_active && !can_auto_download(panel, state))
            {
                return Task::none();
            }
            state.automatic_probe_pending = true;
            let policy = state.download_policy;
            let request = state.request;
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || policy.allows_on_device(hour))
                        .await
                        .unwrap_or(false)
                },
                move |allowed| PanelMessage::AutomaticConditionsChecked(request, policy, allowed),
            )
        }
        #[cfg(not(target_arch = "wasm32"))]
        PanelMessage::AutomaticConditionsChecked(request, policy, allowed) => {
            if !std::mem::take(&mut state.automatic_probe_pending) {
                return Task::none();
            }
            if request != state.request || policy != state.download_policy {
                return Task::none();
            }
            if state.automatic_download_active {
                if !allowed {
                    cancel_download(panel, state);
                }
            } else if allowed && can_auto_download(panel, state) {
                state.automatic_download_active = true;
                return update(panel, state, effects, version, locale, PanelMessage::RunAppUpdate);
            }
            Task::none()
        }
        #[cfg(not(target_arch = "wasm32"))]
        PanelMessage::CachedInstallerChecked(request, ready) => {
            if request == state.request && !state.busy && state.installable()
                && let Some(ready) = ready
            {
                mark_installer_ready(panel, state, ready);
            }
            Task::none()
        }
        PanelMessage::InstallCliTool => {
            if state.busy {
                return Task::none();
            }
            state.request += 1;
            state.checking = false;
            effects.push(Effect::CloseMenu);
            panel.show_cli_install_modal = true;
            panel.cli_install_modal_title = "CLI 更新检测".to_string();
            panel.cli_install_message = None;
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
                return localized_error(effects, "请先选择有效的升级来源", vec![]);
            };
            state.request += 1;
            let request = state.request;
            state.checking = true;
            panel.cli_install_modal_is_checking_update = true;
            panel.cli_install_message = None;
            panel.cli_install_modal_message = "正在请求所选来源的最新 CLI 版本...".into();
            Task::perform(fetch_latest_release_version(source), move |res| {
                PanelMessage::CheckCliToolUpdateFinished(request, source, res)
            })
        }
        PanelMessage::CheckCliToolUpdateFinished(request, source, res) => {
            if request != state.request || Some(source) != state.source || !state.checking {
                return Task::none();
            }
            state.checking = false;
            panel.cli_install_modal_is_checking_update = false;
            match res {
                Ok(version) => {
                    panel.cli_install_modal_server_version = version.clone();
                    if versions_match(&panel.cli_install_modal_current_version, &version) {
                        panel.cli_install_message = None;
                        panel.cli_install_modal_message =
                            "检测完成，本地 CLI 已是最新版本。".to_string();
                    } else {
                        panel.cli_install_message = None;
                        panel.cli_install_modal_message =
                            "检测完成，发现新的 CLI 版本，可直接安装更新。".to_string();
                    }
                }
                Err(error) => {
                    panel.cli_install_modal_server_version = "检测失败".to_string();
                    panel.cli_install_message = None;
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
                return update(
                    panel,
                    state,
                    effects,
                    version,
                    #[cfg(not(target_arch = "wasm32"))]
                    locale,
                    PanelMessage::CheckAppUpdate,
                );
            }
            if !state.busy {
                state.request += 1;
                state.checking = false;
            }
            panel.cli_install_modal_title = "检测更新".into();
            panel.cli_install_message = None;
            panel.cli_install_modal_message =
                state.error.clone().unwrap_or_else(|| "请选择来源并检测最新发布版本。".into());
            panel.cli_install_modal_current_version = current_app_version_label(version);
            panel.cli_install_modal_server_version = "未检测".into();
            panel.cli_install_modal_show_update_action = true;
            panel.cli_install_modal_show_install_action = false;
            panel.cli_install_modal_use_app_update_action = true;
            panel.cli_install_modal_is_checking_update = state.checking || state.busy;
            if let Some(release) = &state.release {
                panel.cli_install_modal_server_version = release.version.clone();
                panel.cli_install_modal_show_install_action = state.installable();
            }
            if state.busy {
                panel.cli_install_modal_show_install_action = false;
                if state.download_abort.is_some() {
                    panel.cli_install_modal_message = if state.download_paused() {
                        "下载已暂停".into()
                    } else {
                        "正在下载并校验安装包...".into()
                    };
                } else {
                    panel.cli_install_modal_title = "更新中…".into();
                    panel.cli_install_modal_message =
                        "正在校验安装包并启动自动更新，请稍候；准备就绪后应用将自动退出。".into();
                }
                return Task::none();
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(ready) = &state.ready {
                panel.cli_install_modal_title = "安装包已就绪".into();
                panel.cli_install_message = None;
                panel.cli_install_modal_message = ready.instructions();
                panel.cli_install_modal_show_install_action = false;
                return Task::none();
            }
            update(
                panel,
                state,
                effects,
                version,
                #[cfg(not(target_arch = "wasm32"))]
                locale,
                PanelMessage::CheckAppUpdate,
            )
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
            panel.cli_install_message = None;
            panel.cli_install_modal_message = "来源已切换，请重新检测更新。".into();
            effects.push(Effect::SaveSource(source));
            Task::none()
        }
        PanelMessage::AppUpdateSourceSaved(result) => {
            state.saving = false;
            match result {
                Ok(()) => Task::none(),
                Err(error) => localized_error(
                    effects,
                    "升级来源未保存，本轮选择仍有效：%s",
                    vec![error.into()],
                ),
            }
        }
        PanelMessage::CheckAppUpdate => {
            if state.busy || state.checking {
                return Task::none();
            }
            let Some(source) = state.source else {
                return localized_error(effects, "请先选择有效的升级来源", vec![]);
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
            panel.cli_install_message = None;
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
                            panel.cli_install_message = None;
                            panel.cli_install_modal_message = if newer {
                                "检测完成，发现新版本，可下载安装包后通过系统安装器安装。"
                            } else {
                                "检测完成，当前应用已是最新版本或高于所选来源版本。"
                            }
                            .into();
                            state.release = Some(release);
                            #[cfg(not(target_arch = "wasm32"))]
                            if newer {
                                let release = state.release.clone().expect("checked snapshot");
                                return Task::perform(
                                    async move {
                                        crate::app::installer_update::restore_desktop_update(release)
                                            .await.ok().flatten()
                                    },
                                    move |ready| PanelMessage::CachedInstallerChecked(request, ready),
                                );
                            }
                        }
                        Err(error) => {
                            return app_update_error(panel, state, effects, error.to_string());
                        }
                    }
                }
                Ok(_) => {
                    return app_update_error(
                        panel,
                        state,
                        effects,
                        "发布结果与所选来源不一致".into(),
                    );
                }
                Err(error) => return app_update_error(panel, state, effects, error),
            }
            Task::none()
        }
        PanelMessage::RunAppUpdate => {
            if state.busy || state.checking || !state.installable() {
                return Task::none();
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                if state.ready.is_some() {
                    return Task::none();
                }
                let release = state.release.clone().expect("checked snapshot");
                if panel.cli_install_modal_use_app_update_action {
                    panel.cli_install_modal_server_version = release.version.clone();
                }
                let (progress_tx, progress_rx) = std::sync::mpsc::channel();
                let (pause, paused) = tokio::sync::watch::channel(false);
                state.busy = true;
                panel.cli_install_modal_is_checking_update = true;
                panel.cli_install_modal_show_install_action = false;
                panel.cli_install_message = None;
                panel.cli_install_modal_message = "正在下载并校验安装包...".into();
                panel.app_update_progress = None;
                panel.app_update_progress_rx = Some(progress_rx);
                state.request += 1;
                let request = state.request;
                let (task, handle) = Task::perform(
                    async move {
                        crate::app::installer_update::prepare_desktop_update_with_pause(
                            &release,
                            progress_tx,
                            paused,
                        )
                        .await
                        .map_err(|error| error.to_string())
                    },
                    move |result| PanelMessage::AppUpdateDownloadPrepared(request, result),
                ).abortable();
                state.download_abort = Some(handle);
                state.download_pause = Some(pause);
                task
            }
            #[cfg(target_arch = "wasm32")]
            {
                app_update_error(panel, state, effects, "当前平台不支持应用更新".into())
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        PanelMessage::SetAppDownloadPaused(paused) => {
            let verifying = panel.app_update_progress.as_ref().is_some_and(|progress| {
                progress.total_bytes.is_some_and(|total| progress.downloaded_bytes >= total)
            });
            if let Some(pause) = &state.download_pause
                && (!paused || !verifying)
                && pause.send(paused).is_ok()
            {
                // A user-controlled download no longer follows automatic download conditions.
                state.automatic_download_active = false;
                panel.cli_install_modal_message = if paused {
                    if let Some(progress) = &mut panel.app_update_progress {
                        progress.bytes_per_second = 0.0;
                    }
                    "下载已暂停".into()
                } else {
                    "正在下载并校验安装包...".into()
                };
            }
            Task::none()
        }
        PanelMessage::CancelAppUpdate => {
            #[cfg(not(target_arch = "wasm32"))]
            if state.download_abort.is_some() {
                state.automatic_retry_after = Some(std::time::Instant::now() + AUTOMATIC_RETRY_DELAY);
                cancel_download(panel, state);
            }
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
            let automatic = std::mem::take(&mut state.automatic_download_active);
            state.download_abort = None;
            state.download_pause = None;
            state.busy = false;
            panel.cli_install_modal_is_checking_update = false;
            // The completion message may arrive before the next progress subscription tick.
            // The producer has finished, so retain its final measured byte count before closing.
            if let Some(receiver) = panel.app_update_progress_rx.take()
                && let Some(progress) = receiver.try_iter().last() {
                    panel.app_update_progress = Some(progress);
                }
            match result {
                Ok(ready) => {
                    mark_installer_ready(panel, state, ready);
                    Task::none()
                }
                Err(error) => {
                    if automatic {
                        state.automatic_retry_after =
                            Some(std::time::Instant::now() + AUTOMATIC_RETRY_DELAY);
                        if !panel.show_cli_install_modal {
                            panel.cli_install_modal_show_install_action = state.installable();
                            return Task::none();
                        }
                    }
                    app_update_error(panel, state, effects, error)
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        PanelMessage::RequestAppInstallerConfirmation => {
            if !state.busy && state.ready.is_some() {
                panel.app_installer_confirmation_request = Some(state.request);
            }
            Task::none()
        }
        #[cfg(not(target_arch = "wasm32"))]
        PanelMessage::ResolveAppInstallerConfirmation { install } => {
            let request = panel.app_installer_confirmation_request.take();
            if install && request == Some(state.request) {
                return update(panel, state, effects, version, locale, PanelMessage::OpenAppInstaller);
            }
            Task::none()
        }
        PanelMessage::OpenAppInstaller => {
            if state.busy {
                return Task::none();
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(ready) = state.ready.clone() {
                panel.cli_install_modal_title = "更新中…".into();
                panel.cli_install_message = None;
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
                    PanelMessage::AppUpdateFinished,
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
                    PanelMessage::AppInstallerDirectoryOpened,
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
                        Ok(mut progress) => {
                            if state.download_paused() {
                                progress.bytes_per_second = 0.0;
                            }
                            panel.app_update_progress = Some(progress);
                        }
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
                    panel.cli_install_message = None;
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
                    panel.cli_install_message = None;
                    panel.cli_install_modal_message = message
                        .render(desktop_i18n::I18n::new(
                            config_ui::DesktopLocale::SimplifiedChinese,
                        ))
                        .expect("valid CLI source templates");
                    panel.cli_install_message = Some(message);
                }
                Err(error) => {
                    panel.cli_install_modal_title = "CLI 安装失败".to_string();
                    let message = error.message();
                    panel.cli_install_message = None;
                    panel.cli_install_modal_message = message
                        .render(desktop_i18n::I18n::new(
                            config_ui::DesktopLocale::SimplifiedChinese,
                        ))
                        .expect("valid CLI source templates");
                    panel.cli_install_message = Some(message);
                }
            }
            Task::none()
        }
        PanelMessage::CloseInstallCliModal => {
            panel.show_cli_install_modal = false;
            if !panel.cli_install_modal_use_app_update_action {
                if !state.busy {
                    state.request += 1;
                    state.checking = false;
                }
                reset_cli_update_modal_state(panel);
            }
            Task::none()
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
const AUTOMATIC_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(15 * 60);

#[cfg(not(target_arch = "wasm32"))]
fn can_auto_download(panel: &PanelState, state: &DesktopUpdate) -> bool {
    state.download_policy.enabled
        && !state.busy
        && !state.checking
        && !state.saving
        && !state.saving_skip
        && state.background_request.is_none()
        && state.ready.is_none()
        && (!panel.show_cli_install_modal || panel.cli_install_modal_use_app_update_action)
        && state.automatic_retry_after.is_none_or(|until| std::time::Instant::now() >= until)
        && state.notification_release().is_some()
}

#[cfg(not(target_arch = "wasm32"))]
fn cancel_download(panel: &mut PanelState, state: &mut DesktopUpdate) {
    let Some(handle) = state.download_abort.take() else {
        return;
    };
    handle.abort();
    state.download_pause = None;
    state.automatic_download_active = false;
    state.request += 1;
    state.busy = false;
    panel.cli_install_modal_is_checking_update = false;
    panel.cli_install_modal_show_install_action = state.installable();
    panel.cli_install_message = None;
    panel.cli_install_modal_message = "下载已取消，可重新下载安装包。".into();
    panel.app_update_progress_rx = None;
    panel.app_update_progress = None;
}

#[cfg(not(target_arch = "wasm32"))]
fn mark_installer_ready(panel: &mut PanelState, state: &mut DesktopUpdate, ready: PreparedDesktopUpdate) {
    if panel.cli_install_modal_use_app_update_action {
        panel.cli_install_modal_title = "安装包已就绪".into();
        panel.cli_install_message = None;
        panel.cli_install_modal_message = ready.instructions();
        panel.cli_install_modal_show_install_action = false;
    }
    state.ready = Some(ready);
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

fn app_update_error(
    panel: &mut PanelState,
    state: &mut DesktopUpdate,
    effects: &mut Vec<Effect>,
    error: String,
) -> Task<PanelMessage> {
    panel.cli_install_modal_show_install_action = state.installable();
    panel.cli_install_message = None;
    panel.cli_install_modal_message = error.clone();
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ready) = &state.ready {
        panel.cli_install_modal_title = "自动更新启动失败".into();
        panel.cli_install_modal_show_install_action = false;
        panel.cli_install_message = None;
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

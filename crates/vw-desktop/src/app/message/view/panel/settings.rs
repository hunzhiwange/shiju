//! 处理设置面板、项目配置与桌面窗口关闭流程的消息。

use super::ViewMessage;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::FocusArea;
use crate::app::message::settings::McpMessage;
use crate::app::message::{ProjectMessage, SettingsMessage};
use crate::app::state::WorkspaceMode;
use crate::app::{App, Message, SettingsTab, set_config_field};
use iced::Task;
#[cfg(not(target_arch = "wasm32"))]
use std::ffi::OsString;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;
#[cfg(not(target_arch = "wasm32"))]
use vw_shared::update;

#[cfg(not(target_arch = "wasm32"))]
const SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
#[cfg(all(not(target_arch = "wasm32"), not(test)))]
const SHUTDOWN_EXIT_WATCHDOG_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy)]
enum ShutdownStreamKind {
    Chat,
    TaskPlanRun,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct ShutdownStreamRequest {
    request_id: String,
    kind: ShutdownStreamKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 关闭请求中需要分别处理的主窗口与任务宠物窗口。
pub(crate) struct CloseRequestedWindows {
    /// 发起关闭的主窗口；若请求来自宠物窗口则为空。
    pub(crate) main_window_id: Option<iced::window::Id>,
    /// 发起关闭的任务宠物窗口；关闭它不会退出主应用。
    pub(crate) task_pet_window_id: Option<iced::window::Id>,
}

/// 将窗口关闭请求分类为退出应用或仅关闭任务宠物。
pub(crate) fn close_requested_windows(
    main_window_id: Option<iced::window::Id>,
    task_pet_window_id: Option<iced::window::Id>,
    requested_window_id: iced::window::Id,
) -> CloseRequestedWindows {
    if main_window_id == Some(requested_window_id) {
        return CloseRequestedWindows {
            main_window_id,
            task_pet_window_id: task_pet_window_id.filter(|id| *id != requested_window_id),
        };
    }

    if task_pet_window_id == Some(requested_window_id) {
        return CloseRequestedWindows { main_window_id: None, task_pet_window_id };
    }

    CloseRequestedWindows { main_window_id: None, task_pet_window_id: None }
}

/// 原子式标记关闭流程；已开始关闭时返回 `false` 防止重复清理。
pub(crate) fn begin_shutdown(shutdown_requested: &mut bool) -> bool {
    if *shutdown_requested {
        return false;
    }
    *shutdown_requested = true;
    true
}

/// 处理设置、更新、项目与窗口生命周期相关的面板消息。
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::ToggleSettingsPanel => {
            if app.workspace_mode == WorkspaceMode::Work {
                return Task::done(Message::View(ViewMessage::WorkSidebarToggled));
            }
            app.show_settings = !app.show_settings;
            set_config_field("show_settings", serde_json::Value::Bool(app.show_settings));
            if app.show_settings
                && let Some(path) = app.project_path.as_deref().map(str::to_owned)
                && !app.project_sessions.contains_key(&path)
                && !app.project_sessions_loading.contains(&path)
            {
                app.project_sessions_loading.insert(path.clone());
                let project_path_clone = path.clone();
                return Task::perform(
                    async move {
                        let client = crate::app::gateway_client().map_err(|err| err.to_string())?;
                        client
                            .session_list::<Vec<vw_shared::session::info::Info>>(Some(
                                &project_path_clone,
                            ))
                            .await
                    },
                    |res| Message::Project(ProjectMessage::ProjectSessionsLoaded(path, res)),
                );
            }
            Task::none()
        }
        ViewMessage::ProjectFileNewSession => {
            if let Some(path) = app.project_path.as_deref().map(str::to_owned) {
                Task::done(Message::Project(ProjectMessage::ProjectCreateSession(path)))
            } else {
                Task::none()
            }
        }
        ViewMessage::ProjectFileNewProject => {
            Task::done(Message::Project(ProjectMessage::OpenFolderPressed))
        }
        ViewMessage::ProjectFileShowSessions => {
            app.show_settings = true;
            set_config_field("show_settings", serde_json::Value::Bool(app.show_settings));
            app.settings_tab = SettingsTab::Sessions;
            Task::none()
        }
        ViewMessage::ProjectFileShowProjects => {
            app.show_settings = true;
            set_config_field("show_settings", serde_json::Value::Bool(app.show_settings));
            app.settings_tab = SettingsTab::Projects;
            Task::none()
        }
        ViewMessage::ProjectFileSaveAll => {
            app.show_preview_context_menu = false;
            let tasks = app
                .preview_tabs
                .iter()
                .filter(|tab| tab.is_dirty)
                .map(|tab| {
                    Task::done(Message::Preview(
                        crate::app::message::PreviewMessage::SaveFilePath {
                            path: tab.path.clone(),
                            notify: false,
                        },
                    ))
                })
                .collect::<Vec<_>>();
            if tasks.is_empty() { Task::none() } else { Task::batch(tasks) }
        }
        ViewMessage::ToggleSystemSettings => {
            app.show_system_settings = !app.show_system_settings;
            app.system_settings_help_tab = None;
            Task::none()
        }
        ViewMessage::OpenSystemSettingsTab(tab) => {
            app.show_system_settings = true;
            app.system_settings_help_tab = None;
            app.active_menu = None;
            app.show_model_popover = false;
            app.show_mode_popover = false;
            app.show_file_popover = false;
            app.show_usage_popover = false;
            Task::done(Message::Settings(SettingsMessage::SystemTabSelected(tab)))
        }
        ViewMessage::OpenSystemSettingsModelDetail(provider_id, model_id) => {
            app.show_system_settings = true;
            app.system_settings_help_tab = None;
            app.active_menu = None;
            app.show_model_popover = false;
            app.show_mode_popover = false;
            app.show_file_popover = false;
            app.show_usage_popover = false;
            app.system_settings_tab = crate::app::components::system_settings::SystemTab::Models;
            app.model_settings.active_tab = crate::app::state::ModelConfigTab::Models;
            Task::done(Message::Settings(SettingsMessage::ModelDetailOpen(provider_id, model_id)))
        }
        ViewMessage::ToggleAboutModal => {
            app.show_about_modal = !app.show_about_modal;
            Task::none()
        }
        ViewMessage::RestartApp => {
            app.active_menu = None;
            Task::perform(async { restart_application() }, |res| {
                Message::View(ViewMessage::RestartAppFinished(res))
            })
        }
        ViewMessage::RestartAppFinished(res) => match res {
            Ok(()) => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    std::process::exit(0);
                }
                #[cfg(target_arch = "wasm32")]
                {
                    app.error_message = Some("当前平台不支持重启应用".to_string());
                    Task::none()
                }
            }
            Err(error) => {
                app.error_message = Some(error);
                Task::none()
            }
        },
        ViewMessage::InstallCliTool => {
            if app.desktop_update.busy {
                return Task::none();
            }
            app.desktop_update.request += 1;
            app.desktop_update.checking = false;
            app.active_menu = None;
            app.show_cli_install_modal = true;
            app.cli_install_modal_title = "CLI 更新检测".to_string();
            app.cli_install_modal_message =
                "打开终端，通过 shiju 调用各项功能：shiju agent 启动 TUI，shiju service 管理服务。下方命令可直接复制。"
                    .to_string();
            app.cli_install_modal_current_version = current_cli_tool_version_label();
            app.cli_install_modal_server_version = "未检测".to_string();
            app.cli_install_modal_show_update_action = true;
            app.cli_install_modal_show_install_action = true;
            app.cli_install_modal_use_app_update_action = false;
            app.cli_install_modal_is_checking_update = false;
            Task::none()
        }
        ViewMessage::RunInstallCliTool => {
            app.cli_install_modal_is_checking_update = false;
            Task::perform(async { install_cli_tool() }, |res| {
                Message::View(ViewMessage::InstallCliToolFinished(res))
            })
        }
        ViewMessage::CliCommandsTabSelected(tab) => {
            app.cli_commands_tab = tab;
            Task::none()
        }
        ViewMessage::CheckCliToolUpdate => {
            if app.desktop_update.busy || app.desktop_update.checking {
                return Task::none();
            }
            let Some(source) = app.desktop_update.source else {
                return app.show_error_toast("请先选择有效的升级来源");
            };
            app.desktop_update.request += 1;
            let request = app.desktop_update.request;
            app.desktop_update.checking = true;
            app.cli_install_modal_is_checking_update = true;
            app.cli_install_modal_message = "正在请求所选来源的最新 CLI 版本...".into();
            Task::perform(fetch_latest_release_version(source), move |res| {
                Message::View(ViewMessage::CheckCliToolUpdateFinished(request, source, res))
            })
        }
        ViewMessage::CheckCliToolUpdateFinished(request, source, res) => {
            if request != app.desktop_update.request
                || Some(source) != app.desktop_update.source
                || !app.desktop_update.checking
            {
                return Task::none();
            }
            app.desktop_update.checking = false;
            app.cli_install_modal_is_checking_update = false;
            match res {
                Ok(version) => {
                    app.cli_install_modal_server_version = version.clone();
                    if versions_match(&app.cli_install_modal_current_version, &version) {
                        app.cli_install_modal_message =
                            "检测完成，本地 CLI 已是最新版本。".to_string();
                    } else {
                        app.cli_install_modal_message =
                            "检测完成，发现新的 CLI 版本，可直接安装更新。".to_string();
                    }
                }
                Err(error) => {
                    app.cli_install_modal_server_version = "检测失败".to_string();
                    app.cli_install_modal_message = error;
                }
            }
            Task::none()
        }
        ViewMessage::OpenAppUpdateModal => {
            app.active_menu = None;
            app.show_cli_install_modal = true;
            if app.cli_install_modal_use_app_update_action {
                return Task::none();
            }
            app.desktop_update.request += 1;
            app.desktop_update.checking = false;
            app.cli_install_modal_title = "检测更新".into();
            app.cli_install_modal_message = app
                .desktop_update
                .error
                .clone()
                .unwrap_or_else(|| "请选择来源并检测最新发布版本。".into());
            app.cli_install_modal_current_version = current_app_version_label();
            app.cli_install_modal_server_version = "未检测".into();
            app.cli_install_modal_show_update_action = true;
            app.cli_install_modal_show_install_action = false;
            app.cli_install_modal_use_app_update_action = true;
            app.cli_install_modal_is_checking_update =
                app.desktop_update.checking || app.desktop_update.busy;
            if let Some(release) = &app.desktop_update.release {
                app.cli_install_modal_server_version = release.version.clone();
                app.cli_install_modal_show_install_action = app.desktop_update.installable();
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(ready) = &app.desktop_update.ready {
                app.cli_install_modal_title = "安装包已就绪".into();
                app.cli_install_modal_message = ready.instructions();
                app.cli_install_modal_show_install_action = false;
            }
            Task::none()
        }
        ViewMessage::AppUpdateSourceSelected(source) => {
            if app.desktop_update.busy || app.desktop_update.saving {
                return Task::none();
            }
            app.desktop_update.source = Some(source);
            app.desktop_update.error = None;
            app.desktop_update.selected_by_user = true;
            app.desktop_update.saving = true;
            app.desktop_update.request += 1;
            app.desktop_update.checking = false;
            app.desktop_update.release = None;
            #[cfg(not(target_arch = "wasm32"))]
            {
                app.desktop_update.ready = None;
            }
            app.app_update_progress = None;
            app.app_update_progress_rx = None;
            app.cli_install_modal_server_version = "未检测".into();
            app.cli_install_modal_show_install_action = !app.cli_install_modal_use_app_update_action;
            app.cli_install_modal_is_checking_update = false;
            app.cli_install_modal_message = "来源已切换，请重新检测更新。".into();
            Task::perform(
                crate::app::config::save_app_config_async(
                    crate::app::desktop_update::source_patch(source),
                ),
                |result| Message::View(ViewMessage::AppUpdateSourceSaved(result)),
            )
        }
        ViewMessage::AppUpdateSourceSaved(result) => {
            app.desktop_update.saving = false;
            match result {
                Ok(()) => Task::none(),
                Err(error) => {
                    app.show_error_toast(format!("升级来源未保存，本轮选择仍有效：{error}"))
                }
            }
        }
        ViewMessage::CheckAppUpdate => {
            if app.desktop_update.busy || app.desktop_update.checking {
                return Task::none();
            }
            let Some(source) = app.desktop_update.source else {
                return app.show_error_toast("请先选择有效的升级来源");
            };
            app.desktop_update.request += 1;
            let request = app.desktop_update.request;
            app.desktop_update.checking = true;
            app.desktop_update.release = None;
            #[cfg(not(target_arch = "wasm32"))]
            {
                app.desktop_update.ready = None;
            }
            app.cli_install_modal_is_checking_update = true;
            app.cli_install_modal_show_install_action = false;
            app.cli_install_modal_message = "正在请求所选来源的最新应用版本...".into();
            app.app_update_progress = None;
            app.app_update_progress_rx = None;
            Task::perform(fetch_latest_app_release(source), move |res| {
                Message::View(ViewMessage::CheckAppUpdateFinished(request, source, res))
            })
        }
        ViewMessage::CheckAppUpdateFinished(request, source, res) => {
            if request != app.desktop_update.request
                || Some(source) != app.desktop_update.source
                || !app.desktop_update.checking
            {
                return Task::none();
            }
            app.desktop_update.checking = false;
            app.cli_install_modal_is_checking_update = false;
            match res {
                Ok(release) if release.source == source => {
                    app.cli_install_modal_server_version = release.version.clone();
                    let newer = release.is_newer_than(env!("CARGO_PKG_VERSION"));
                    match newer {
                        Ok(newer) => {
                            app.cli_install_modal_show_install_action = newer;
                            app.cli_install_modal_message = if newer {
                                "检测完成，发现新版本，可下载安装包后通过系统安装器安装。"
                            } else {
                                "检测完成，当前应用已是最新版本或高于所选来源版本。"
                            }
                            .into();
                            app.desktop_update.release = Some(release);
                        }
                        Err(error) => return app_update_error(app, error.to_string()),
                    }
                }
                Ok(_) => return app_update_error(app, "发布结果与所选来源不一致".into()),
                Err(error) => return app_update_error(app, error),
            }
            Task::none()
        }
        ViewMessage::RunAppUpdate => {
            if app.desktop_update.busy
                || app.desktop_update.checking
                || !app.desktop_update.installable()
            {
                return Task::none();
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let release = app.desktop_update.release.clone().expect("checked snapshot");
                let (progress_tx, progress_rx) = std::sync::mpsc::channel();
                app.desktop_update.busy = true;
                app.cli_install_modal_is_checking_update = true;
                app.cli_install_modal_show_install_action = false;
                app.cli_install_modal_message = "正在下载并校验安装包...".into();
                app.app_update_progress = None;
                app.app_update_progress_rx = Some(progress_rx);
                Task::perform(
                    async move {
                        crate::app::installer_update::prepare_desktop_update(&release, progress_tx)
                            .await
                            .map_err(|error| error.to_string())
                    },
                    |result| Message::View(ViewMessage::AppUpdatePrepared(result)),
                )
            }
            #[cfg(target_arch = "wasm32")]
            {
                app_update_error(app, "当前平台不支持应用更新".into())
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        ViewMessage::AppUpdatePrepared(result) => {
            if !app.desktop_update.busy {
                return Task::none();
            }
            app.desktop_update.busy = false;
            app.cli_install_modal_is_checking_update = false;
            app.app_update_progress_rx = None;
            match result {
                Ok(ready) => {
                    app.cli_install_modal_title = "安装包已就绪".into();
                    app.cli_install_modal_message = ready.instructions();
                    app.desktop_update.ready = Some(ready);
                    Task::none()
                }
                Err(error) => app_update_error(app, error),
            }
        }
        ViewMessage::OpenAppInstaller => {
            if app.desktop_update.busy {
                return Task::none();
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(ready) = app.desktop_update.ready.clone() {
                app.desktop_update.busy = true;
                app.cli_install_modal_is_checking_update = true;
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::app::installer_update::open_desktop_installer(&ready)
                        })
                        .await
                        .map_err(|error| error.to_string())?
                        .map_err(|error| format!("{error:#}"))
                    },
                    |result| Message::View(ViewMessage::AppUpdateFinished(result)),
                );
            }
            Task::none()
        }
        ViewMessage::AppUpdateProgressTick => {
            if let Some(receiver) = app.app_update_progress_rx.take() {
                let mut active_receiver = Some(receiver);
                for _ in 0..48 {
                    let Some(current_receiver) = active_receiver.as_ref() else {
                        break;
                    };
                    match current_receiver.try_recv() {
                        Ok(progress) => app.app_update_progress = Some(progress),
                        Err(std::sync::mpsc::TryRecvError::Empty) => break,
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            active_receiver = None;
                            break;
                        }
                    }
                }
                app.app_update_progress_rx = active_receiver;
            }
            Task::none()
        }
        ViewMessage::AppUpdateFinished(res) => {
            if !app.desktop_update.busy {
                return Task::none();
            }
            app.desktop_update.busy = false;
            app.cli_install_modal_is_checking_update = false;
            app.app_update_progress_rx = None;
            match res {
                Ok(message) => {
                    app.cli_install_modal_title = "安装包已就绪".into();
                    app.cli_install_modal_message = message;
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(window_id) = app.main_window_id {
                            return start_shutdown(app, window_id);
                        }
                        if begin_shutdown(&mut app.shutdown_requested) {
                            arm_shutdown_exit_watchdog();
                            return iced::exit();
                        }
                    }
                    Task::none()
                }
                Err(error) => app_update_error(app, error),
            }
        }
        ViewMessage::InstallCliToolFinished(res) => {
            app.show_cli_install_modal = true;
            reset_cli_update_modal_state(app);
            match res {
                Ok(message) => {
                    app.cli_install_modal_title = "CLI 安装完成".to_string();
                    app.cli_install_modal_message = message;
                }
                Err(error) => {
                    app.cli_install_modal_title = "CLI 安装失败".to_string();
                    app.cli_install_modal_message = error;
                }
            }
            Task::none()
        }
        ViewMessage::CloseInstallCliModal => {
            app.show_cli_install_modal = false;
            if !app.cli_install_modal_use_app_update_action {
                app.desktop_update.request += 1;
                app.desktop_update.checking = false;
                reset_cli_update_modal_state(app);
            }
            Task::none()
        }
        ViewMessage::ToggleDiffPanel => {
            if app.workspace_mode == WorkspaceMode::Work {
                return Task::none();
            }
            app.show_diff = !app.show_diff;
            Task::none()
        }
        ViewMessage::ToggleGitDiffSummary => {
            app.show_git_diff_summary = !app.show_git_diff_summary;
            set_config_field(
                "show_git_diff_summary",
                serde_json::Value::Bool(app.show_git_diff_summary),
            );
            Task::none()
        }
        ViewMessage::ToggleTerminalPanel => {
            if app.workspace_mode == WorkspaceMode::Work {
                return Task::none();
            }
            app.terminal.is_visible = !app.terminal.is_visible;
            if app.terminal.is_visible && app.terminal.height < 160.0 {
                app.terminal.height = 200.0;
            }
            if app.terminal.is_visible {
                #[cfg(not(target_arch = "wasm32"))]
                if app.terminal.ensure_default_terminal(
                    app.project_path.as_ref().map(std::path::PathBuf::from),
                ) {
                    app.focus_area = FocusArea::Terminal;
                } else if !app.terminal.tabs.is_empty() {
                    app.focus_area = FocusArea::Terminal;
                }

                #[cfg(not(target_arch = "wasm32"))]
                app.terminal.apply_app_theme(&app.app_theme);
            }
            set_config_field("show_terminal", serde_json::Value::Bool(app.terminal.is_visible));
            Task::none()
        }
        ViewMessage::FileManagerPanelVisible(b) => {
            app.show_file_manager = b;
            if !b {
                app.dragging_file_manager = false;
                app.file_manager_drag_anchor_x = None;
            }
            set_config_field("show_file_manager", serde_json::Value::Bool(b));
            Task::none()
        }
        ViewMessage::OpenTerminalPressed => {
            if let Some(_path) = crate::app::components::git_panel::git_repo_path_for_app(app) {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let _ = crate::app::components::git_panel::open_terminal(&_path);
                }
            }
            Task::none()
        }
        ViewMessage::AutoMaxToggled(b) => {
            app.auto_max_mode = b;
            set_config_field("auto_max_mode", serde_json::Value::Bool(b));
            Task::none()
        }
        ViewMessage::ToggleMenu(menu) => {
            if let Some(m) = menu {
                if app.active_menu == Some(m) {
                    app.active_menu = None;
                } else {
                    app.active_menu = Some(m);
                }
            } else {
                app.active_menu = None;
            }
            Task::none()
        }
        ViewMessage::GatewayServicesTabSelected(tab) => {
            app.top_bar_gateway_tab = tab;
            if tab == crate::app::state::TopBarGatewayTab::Plugins
                && app.skills_settings.plugin_catalog.is_empty()
                && !app.skills_settings.plugin_loading
            {
                app.skills_settings.plugin_loading = true;
                return Task::done(Message::Settings(SettingsMessage::PluginsRefresh));
            }
            if tab == crate::app::state::TopBarGatewayTab::Mcp
                && !app.mcp_settings.loading
                && !app.mcp_settings.saving
            {
                app.mcp_settings.loading = true;
                app.mcp_settings.save_error = None;
                return Task::done(Message::Settings(SettingsMessage::Mcp(McpMessage::Refresh)));
            }
            Task::none()
        }
        ViewMessage::MenuAction(msg) => {
            app.active_menu = None;
            Task::done(*msg)
        }
        ViewMessage::MenuHovered(menu) => {
            if app.active_menu.is_some() {
                app.active_menu = Some(menu);
            }
            Task::none()
        }
        _ => Task::none(),
    }
}

/// 关闭宠物窗口或启动一次有超时保护的应用退出流程。
pub fn close_requested(app: &mut App, window_id: iced::window::Id) -> Task<Message> {
    let windows = close_requested_windows(app.main_window_id, app.task_pet_window_id, window_id);

    if windows.main_window_id.is_some() {
        #[cfg(not(target_arch = "wasm32"))]
        return start_shutdown(app, window_id);

        #[cfg(target_arch = "wasm32")]
        return iced::window::close(window_id);
    }

    if let Some(task_pet_window_id) = windows.task_pet_window_id {
        return iced::window::close(task_pet_window_id);
    }

    Task::none()
}

#[cfg(not(target_arch = "wasm32"))]
fn start_shutdown(app: &mut App, window_id: iced::window::Id) -> Task<Message> {
    if !begin_shutdown(&mut app.shutdown_requested) {
        return Task::none();
    }

    arm_shutdown_exit_watchdog();
    let requests = take_shutdown_stream_requests(app);
    app.work_browser_controller.shutdown();
    if let Some(manager) = app.lsp_manager.as_mut() {
        manager.shutdown_all();
    }
    for child in &mut app.independent_webview_children {
        let _ = child.kill();
    }
    app.independent_webview_children.clear();

    Task::batch([
        iced::window::set_mode(window_id, iced::window::Mode::Hidden),
        Task::perform(stop_shutdown_streams(requests), |_| {
            Message::View(ViewMessage::ShutdownFinished)
        }),
    ])
}

#[cfg(all(not(target_arch = "wasm32"), not(test)))]
fn arm_shutdown_exit_watchdog() {
    let _ =
        std::thread::Builder::new().name("vw-desktop-shutdown-watchdog".to_string()).spawn(|| {
            std::thread::sleep(SHUTDOWN_EXIT_WATCHDOG_TIMEOUT);
            std::process::exit(0);
        });
}

#[cfg(all(not(target_arch = "wasm32"), test))]
fn arm_shutdown_exit_watchdog() {}

#[cfg(not(target_arch = "wasm32"))]
fn take_shutdown_stream_requests(app: &mut App) -> Vec<ShutdownStreamRequest> {
    let requests = app
        .session_runtime_states
        .values_mut()
        .filter_map(|runtime| runtime.active_agent_request.take())
        .map(|request| ShutdownStreamRequest {
            request_id: crate::app::impls::desktop_stream_request_id(&request),
            kind: if crate::app::impls::task_plan_run_active(
                request.workflow_mode_enabled,
                request.resume_history_only,
            ) {
                ShutdownStreamKind::TaskPlanRun
            } else {
                ShutdownStreamKind::Chat
            },
        })
        .collect();
    app.active_agent_request = None;
    app.is_requesting = false;
    requests
}

#[cfg(not(target_arch = "wasm32"))]
async fn stop_shutdown_streams(requests: Vec<ShutdownStreamRequest>) {
    let Ok(client) = crate::app::gateway_client() else {
        return;
    };
    let mut stops = tokio::task::JoinSet::new();
    for request in requests {
        let client = client.clone();
        stops.spawn(async move {
            match request.kind {
                ShutdownStreamKind::Chat => {
                    let _ = client.chat_stream_stop(&request.request_id).await;
                }
                ShutdownStreamKind::TaskPlanRun => {
                    let _ = client.task_plan_run_stream_stop(&request.request_id).await;
                }
            }
        });
    }

    let wait_for_stops = async { while stops.join_next().await.is_some() {} };
    if tokio::time::timeout(SHUTDOWN_TIMEOUT, wait_for_stops).await.is_err() {
        stops.abort_all();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn restart_application() -> Result<(), String> {
    let current_exe =
        std::env::current_exe().map_err(|e| format!("获取当前可执行文件失败: {e}"))?;
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    std::process::Command::new(current_exe)
        .args(args)
        .spawn()
        .map_err(|e| format!("启动新进程失败: {e}"))?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn restart_application() -> Result<(), String> {
    Err("当前平台不支持重启应用".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn install_cli_tool() -> Result<String, String> {
    let home = resolve_home_dir()?;
    let install_dir = vw_config_types::paths::home_config_dir(&home).join("bin");
    fs::create_dir_all(&install_dir).map_err(|e| format!("创建安装目录失败: {e}"))?;

    let target_name = if cfg!(windows) { "shiju.exe" } else { "shiju" };
    let target_bin = install_dir.join(target_name);
    let replacing_existing = target_bin.exists();

    let source_bin_res = resolve_agent_binary_path();
    if let Ok(source_bin) = source_bin_res {
        install_cli_binary_from_path(&source_bin, &target_bin)?;
    } else if let Ok(url) = std::env::var("VIBEWINDOW_CLI_URL") {
        install_cli_binary_from_url(&url, &target_bin)?;
    } else {
        return Err(format!(
            "未在应用包中发现 CLI，也未设置 VIBEWINDOW_CLI_URL 可供下载。\n请下载独立 CLI 包并手动安装：\n1) 解压后将可执行文件复制到 {}\n2) 重命名为 shiju 或 shiju.exe\n3) 确保 PATH 包含 {} 后重启终端",
            vw_config_types::paths::tilde_config_path("bin"),
            vw_config_types::paths::tilde_config_path("bin")
        ));
    }

    let updated_profiles = ensure_path_profiles(&home, &install_dir)?;
    let mut message =
        format!("已安装到 {}\n可执行文件: {}", install_dir.display(), target_bin.display());
    if replacing_existing {
        message.push_str(
            "\n已安全替换旧 CLI；如果后台 service 正在运行，请重启 service 以加载新版本。",
        );
    }
    if updated_profiles.is_empty() {
        message.push_str("\nPATH 配置已存在，无需重复写入。");
    } else {
        message.push_str("\n已更新 PATH 配置文件：");
        for profile in updated_profiles {
            message.push_str(&format!("\n- {profile}"));
        }
    }
    Ok(message)
}

#[cfg(target_arch = "wasm32")]
fn install_cli_tool() -> Result<String, String> {
    Err("当前平台不支持安装 CLI 工具".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn install_cli_binary_from_path(source_bin: &Path, target_bin: &Path) -> Result<(), String> {
    let staged_bin = staged_cli_binary_path(target_bin);
    remove_stale_staged_cli_binary(&staged_bin)?;
    fs::copy(source_bin, &staged_bin).map_err(|e| {
        let _ = fs::remove_file(&staged_bin);
        format!(
            "复制 CLI 可执行文件失败: {} -> {}: {e}",
            source_bin.display(),
            staged_bin.display()
        )
    })?;
    install_staged_cli_binary(&staged_bin, target_bin)
}

#[cfg(not(target_arch = "wasm32"))]
fn install_cli_binary_from_url(url: &str, target_bin: &Path) -> Result<(), String> {
    let staged_bin = staged_cli_binary_path(target_bin);
    remove_stale_staged_cli_binary(&staged_bin)?;
    if let Err(error) = download_file(url, &staged_bin) {
        let _ = fs::remove_file(&staged_bin);
        return Err(error);
    }
    install_staged_cli_binary(&staged_bin, target_bin)
}

#[cfg(not(target_arch = "wasm32"))]
fn install_staged_cli_binary(staged_bin: &Path, target_bin: &Path) -> Result<(), String> {
    ensure_cli_binary_executable(staged_bin)?;
    replace_cli_binary(staged_bin, target_bin).map_err(|error| {
        let _ = fs::remove_file(staged_bin);
        error
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn staged_cli_binary_path(target_bin: &Path) -> PathBuf {
    let file_name = target_bin.file_name().and_then(|value| value.to_str()).unwrap_or("shiju");
    target_bin.with_file_name(format!(".{file_name}.installing"))
}

#[cfg(not(target_arch = "wasm32"))]
fn remove_stale_staged_cli_binary(staged_bin: &Path) -> Result<(), String> {
    if staged_bin.exists() {
        fs::remove_file(staged_bin)
            .map_err(|e| format!("清理旧临时 CLI 文件失败 {}: {e}", staged_bin.display()))?;
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_cli_binary_executable(_path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(_path, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("设置执行权限失败: {e}"))?;
    }
    Ok(())
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
fn replace_cli_binary(staged_bin: &Path, target_bin: &Path) -> Result<(), String> {
    fs::rename(staged_bin, target_bin).map_err(|e| {
        format!(
            "替换 CLI 可执行文件失败: {} -> {}: {e}",
            staged_bin.display(),
            target_bin.display()
        )
    })
}

#[cfg(all(not(unix), not(target_arch = "wasm32")))]
fn replace_cli_binary(staged_bin: &Path, target_bin: &Path) -> Result<(), String> {
    let backup_bin = target_bin.with_extension("backup");
    if backup_bin.exists() {
        let _ = fs::remove_file(&backup_bin);
    }
    if target_bin.exists() {
        fs::rename(target_bin, &backup_bin).map_err(|e| {
            format!(
                "备份旧 CLI 可执行文件失败: {} -> {}: {e}",
                target_bin.display(),
                backup_bin.display()
            )
        })?;
    }
    if let Err(error) = fs::rename(staged_bin, target_bin) {
        let _ = fs::rename(&backup_bin, target_bin);
        return Err(format!(
            "替换 CLI 可执行文件失败: {} -> {}: {error}",
            staged_bin.display(),
            target_bin.display()
        ));
    }
    let _ = fs::remove_file(&backup_bin);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_latest_release_version(source: update::UpdateSource) -> Result<String, String> {
    fetch_latest_app_release(source).await.map(|release| release.version)
}

#[cfg(target_arch = "wasm32")]
async fn fetch_latest_release_version(
    _source: vw_shared::update::UpdateSource,
) -> Result<String, String> {
    Err("当前平台不支持检测更新".to_string())
}

fn app_update_error(app: &mut App, error: String) -> Task<Message> {
    app.cli_install_modal_show_install_action = app.desktop_update.installable();
    app.cli_install_modal_message = error.clone();
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ready) = &app.desktop_update.ready {
        app.cli_install_modal_title = "自动更新启动失败".into();
        app.cli_install_modal_show_install_action = false;
        app.cli_install_modal_message = format!("{error}\n{}", ready.instructions());
    }
    app.show_error_toast(error)
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_latest_app_release(
    source: update::UpdateSource,
) -> Result<update::DesktopRelease, String> {
    update::fetch_desktop_release(source).await.map_err(|e| format!("请求应用版本失败: {e}"))
}

#[cfg(target_arch = "wasm32")]
async fn fetch_latest_app_release(
    _source: vw_shared::update::UpdateSource,
) -> Result<vw_shared::update::DesktopRelease, String> {
    Err("当前平台不支持检测应用更新".into())
}

fn reset_cli_update_modal_state(app: &mut App) {
    app.cli_install_modal_current_version.clear();
    app.cli_install_modal_server_version.clear();
    app.cli_install_modal_show_update_action = false;
    app.cli_install_modal_show_install_action = false;
    app.cli_install_modal_use_app_update_action = false;
    app.cli_install_modal_is_checking_update = false;
    app.app_update_progress = None;
    app.app_update_progress_rx = None;
}

#[cfg(not(target_arch = "wasm32"))]
fn current_app_version_label() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}

#[cfg(target_arch = "wasm32")]
fn current_app_version_label() -> String {
    "当前平台不支持".to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn current_cli_tool_version_label() -> String {
    match read_installed_cli_version() {
        Ok(version) => version,
        Err(error) => error,
    }
}

#[cfg(target_arch = "wasm32")]
fn current_cli_tool_version_label() -> String {
    "当前平台不支持".to_string()
}

fn versions_match(current: &str, latest: &str) -> bool {
    normalize_version(current) == normalize_version(latest)
}

fn normalize_version(value: &str) -> String {
    let line = value
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim())
        .unwrap_or_else(|| value.trim());
    line.split_whitespace().last().unwrap_or(line).trim().trim_start_matches('v').to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn read_installed_cli_version() -> Result<String, String> {
    let output = match Command::new("shiju").arg("--version").output() {
        Ok(output) => output,
        Err(_) => {
            let cli_path = resolve_cli_binary_path()?;
            Command::new(cli_path)
                .arg("--version")
                .output()
                .map_err(|e| format!("执行 shiju --version 失败: {e}"))?
        }
    };
    extract_version_output(output.stdout, output.stderr)
}

#[cfg(not(target_arch = "wasm32"))]
fn extract_version_output(stdout: Vec<u8>, stderr: Vec<u8>) -> Result<String, String> {
    let combined = if !stdout.is_empty() {
        String::from_utf8_lossy(&stdout).into_owned()
    } else {
        String::from_utf8_lossy(&stderr).into_owned()
    };
    combined
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .ok_or_else(|| "CLI 未安装或未返回版本信息".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_cli_binary_path() -> Result<PathBuf, String> {
    let home = resolve_home_dir()?;
    let cli_path = vw_config_types::paths::home_config_dir(home)
        .join("bin")
        .join(if cfg!(windows) { "shiju.exe" } else { "shiju" });
    if cli_path.exists() {
        Ok(cli_path)
    } else {
        Err("CLI 未安装，无法执行 shiju --version".to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_home_dir() -> Result<PathBuf, String> {
    if let Some(user_dirs) = directories::UserDirs::new() {
        return Ok(user_dirs.home_dir().to_path_buf());
    }
    if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        return Ok(PathBuf::from(home));
    }
    if let Ok(home) = std::env::var("USERPROFILE")
        && !home.trim().is_empty()
    {
        return Ok(PathBuf::from(home));
    }
    Err("无法定位用户主目录".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_agent_binary_path() -> Result<PathBuf, String> {
    let current_exe =
        std::env::current_exe().map_err(|e| format!("获取当前可执行文件失败: {e}"))?;
    let exe_dir = current_exe.parent().ok_or_else(|| "无法定位当前可执行文件目录".to_string())?;
    let agent_names =
        if cfg!(windows) { ["shiju.exe", "vibe-agent.exe"] } else { ["shiju", "vibe-agent"] };
    let mut candidates = agent_names.iter().map(|name| exe_dir.join(name)).collect::<Vec<_>>();

    if let Ok(cwd) = std::env::current_dir() {
        for agent_name in agent_names {
            candidates.push(cwd.join("target").join("release").join(agent_name));
            candidates.push(cwd.join("target").join("debug").join(agent_name));
        }
    }

    for candidate in candidates {
        if candidate.exists() && candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err("未找到内置 CLI 可执行文件 shiju（兼容名 vibe-agent）".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    if cfg!(windows) {
        let status = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(format!("Invoke-WebRequest -Uri '{}' -OutFile '{}'", url, dest.display()))
            .status()
            .map_err(|e| format!("启动下载命令失败: {e}"))?;
        if status.success() {
            return Ok(());
        }
        Err("下载失败，请手动下载并安装，或确保 PowerShell 可用".to_string())
    } else {
        let curl =
            Command::new("curl").arg("-fsSL").arg(url).arg("-o").arg(dest.as_os_str()).status();
        if let Ok(status) = curl
            && status.success()
        {
            return Ok(());
        }
        let wget = Command::new("wget").arg("-qO").arg(dest.as_os_str()).arg(url).status();
        if let Ok(status) = wget
            && status.success()
        {
            return Ok(());
        }
        Err("下载失败，请手动下载并安装，或确保已安装 curl/wget".to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_path_profiles(home: &Path, install_dir: &Path) -> Result<Vec<String>, String> {
    #[cfg(windows)]
    let profiles = vec![
        home.join("Documents").join("WindowsPowerShell").join("Microsoft.PowerShell_profile.ps1"),
        home.join("Documents").join("PowerShell").join("Microsoft.PowerShell_profile.ps1"),
    ];
    #[cfg(not(windows))]
    let profiles = vec![home.join(".zshrc"), home.join(".bashrc"), home.join(".profile")];

    let mut updated = Vec::new();
    for profile in profiles {
        if ensure_profile_contains_path(&profile, install_dir)? {
            updated.push(profile.display().to_string());
        }
    }
    Ok(updated)
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_profile_contains_path(profile: &Path, install_dir: &Path) -> Result<bool, String> {
    #[cfg(windows)]
    let export_line = format!(r#"$env:Path = "{};$env:Path""#, install_dir.display());
    #[cfg(not(windows))]
    let export_line = format!("export PATH={}:$PATH", install_dir.display());

    if let Some(parent) = profile.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建配置文件目录失败 {}: {e}", parent.display()))?;
    }

    let install_path_text = install_dir.to_string_lossy();
    let existing = fs::read_to_string(profile).unwrap_or_default();
    if existing.contains(export_line.as_str()) || existing.contains(install_path_text.as_ref()) {
        return Ok(false);
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(profile)
        .map_err(|e| format!("打开配置文件失败 {}: {e}", profile.display()))?;

    if !existing.ends_with('\n') && !existing.is_empty() {
        file.write_all(b"\n")
            .map_err(|e| format!("写入配置文件失败 {}: {e}", profile.display()))?;
    }
    file.write_all(export_line.as_bytes())
        .map_err(|e| format!("写入配置文件失败 {}: {e}", profile.display()))?;
    file.write_all(b"\n").map_err(|e| format!("写入配置文件失败 {}: {e}", profile.display()))?;
    Ok(true)
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;

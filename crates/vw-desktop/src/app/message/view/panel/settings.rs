//! 处理设置面板、项目配置与桌面窗口关闭流程的消息。

use super::ViewMessage;
use crate::app::message::settings::McpMessage;
use crate::app::message::{ProjectMessage, SettingsMessage};
use crate::app::state::WorkspaceMode;
use crate::app::{App, Message, SettingsTab};
use iced::Task;
use desktop_update::panel::{PanelMessage, PanelState, Effect};
#[cfg(not(target_arch = "wasm32"))]
use std::ffi::OsString;

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

pub(crate) use desktop_navigation::window_lifecycle::close_requested_windows;

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
            if app.navigation_mode.workspace_mode == WorkspaceMode::Work {
                return Task::done(Message::View(ViewMessage::WorkSidebarToggled));
            }
            app.show_settings = !app.show_settings;
            let save = app.save_desktop_preference("show_settings", app.show_settings.into());
            if app.show_settings
                && let Some(path) = app.project_path.as_deref().map(str::to_owned)
                && !app.project_sessions.contains_key(&path)
                && !app.project_sessions_loading.contains(&path)
            {
                app.project_sessions_loading.insert(path.clone());
                let project_path_clone = path.clone();
                let sessions = Task::perform(
                    async move {
                        let client = crate::app::gateway_client().map_err(|err| err.to_string())?;
                        crate::app::message::project::helpers::list_gateway_project_sessions(
                            &client,
                            &project_path_clone,
                        )
                        .await
                    },
                    |res| Message::Project(ProjectMessage::ProjectSessionsLoaded(path, res)),
                );
                return Task::batch([save, sessions]);
            }
            save
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
            app.settings_tab = SettingsTab::Sessions;
            app.save_desktop_preference("show_settings", true.into())
        }
        ViewMessage::ProjectFileShowProjects => {
            app.show_settings = true;
            app.settings_tab = SettingsTab::Projects;
            app.save_desktop_preference("show_settings", true.into())
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
            app.project_tools_menu_path = None;
            app.system_settings_help_tab = None;
            #[cfg(not(target_arch = "wasm32"))]
            {
                app.agent_apps_auth.form = None;
                if app.show_system_settings
                    && app.system_settings_tab
                        == crate::app::components::system_settings::SystemTab::Auth
                {
                    return crate::app::message::agent_apps_auth::update(
                        app,
                        crate::app::message::agent_apps_auth::Event::Refresh,
                    );
                }
            }
            if app.show_system_settings {
                Task::none()
            } else {
                crate::app::message::editor::metrics::commit(app, None)
            }
        }
        ViewMessage::OpenAcpSettings => {
            app.show_system_settings = true;
            app.system_settings_help_tab = None;
            app.active_menu = None;
            app.show_acp_popover = false;
            app.system_settings_tab = crate::app::components::system_settings::SystemTab::Agents;
            Task::done(Message::Settings(SettingsMessage::AgentConfigTab(
                crate::app::message::settings::AgentConfigTabMessage::Selected(
                    crate::app::state::AgentConfigTab::Acp,
                ),
            )))
        }
        ViewMessage::OpenSystemSettingsTab(tab) => {
            app.show_system_settings = true;
            app.system_settings_help_tab = None;
            app.active_menu = None;
            app.show_model_popover = false;
            app.show_mode_popover = false;
            app.show_file_popover = false;
            app.show_usage_popover = false;
            app.show_session_tool_selector_popover = false;
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
            #[cfg(not(target_arch = "wasm32"))]
            let (
                editor_commit,
                editor_saves,
                routing_commit,
                routing_saves,
                worktree_saves,
                preference_saves,
            ) = {
                if !begin_shutdown(&mut app.shutdown_requested) {
                    return Task::none();
                }
                let commit = crate::app::message::editor::metrics::commit(app, None);
                let (routing_commit, routing_saves) = crate::app::message::settings::routing_persistence::flush(app);
                (
                    commit,
                    app.editor_config_persistence.wait_for_idle(),
                    routing_commit,
                    routing_saves,
                    crate::app::message::settings::wait_for_worktree_saves(app),
                    app.desktop_preferences_persistence.wait_for_idle(),
                )
            };
            #[cfg(target_arch = "wasm32")]
            let (editor_commit, routing_commit) = (Task::none(), Task::none());
            Task::batch([
                editor_commit,
                routing_commit,
                Task::perform(
                    async move {
                        #[cfg(not(target_arch = "wasm32"))]
                        tokio::join!(
                            wait_for_editor_settings_saves(editor_saves),
                            wait_for_settings_saves(routing_saves),
                            wait_for_settings_saves(worktree_saves),
                            wait_for_settings_saves(preference_saves)
                        );
                        restart_application()
                    },
                    |res| Message::View(ViewMessage::RestartAppFinished(res)),
                ),
            ])
        }
        ViewMessage::RestartAppFinished(res) => match res {
            Ok(()) => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    app.web_install.runtime.shutdown();
                    std::process::exit(0);
                }
                #[cfg(target_arch = "wasm32")]
                {
                    app.error_message = Some("当前平台不支持重启应用".to_string());
                    Task::none()
                }
            }
            Err(error) => {
                app.shutdown_requested = false;
                app.error_message = Some(error);
                Task::none()
            }
        },
        ViewMessage::CliCommandsTabSelected(tab) => {
            app.cli_commands_tab = tab;
            Task::none()
        }
        ViewMessage::InstallCliTool => update_panel(app, PanelMessage::InstallCliTool),
        ViewMessage::RunInstallCliTool => update_panel(app, PanelMessage::RunInstallCliTool),
        ViewMessage::CheckCliToolUpdate => update_panel(app, PanelMessage::CheckCliToolUpdate),
        ViewMessage::CheckCliToolUpdateFinished(v0, v1, v2) => update_panel(app, PanelMessage::CheckCliToolUpdateFinished(v0, v1, v2)),
        ViewMessage::OpenAppUpdateModal => update_panel(app, PanelMessage::OpenAppUpdateModal),
        ViewMessage::AppUpdateSourceSelected(v0) => update_panel(app, PanelMessage::AppUpdateSourceSelected(v0)),
        ViewMessage::AppUpdateSourceSaved(v0) => update_panel(app, PanelMessage::AppUpdateSourceSaved(v0)),
        ViewMessage::CheckAppUpdate => update_panel(app, PanelMessage::CheckAppUpdate),
        ViewMessage::CheckAppUpdateFinished(v0, v1, v2) => update_panel(app, PanelMessage::CheckAppUpdateFinished(v0, v1, v2)),
        ViewMessage::RunAppUpdate => update_panel(app, PanelMessage::RunAppUpdate),
        ViewMessage::CancelAppUpdate => update_panel(app, PanelMessage::CancelAppUpdate),
        #[cfg(not(target_arch = "wasm32"))]
        ViewMessage::AppUpdateDownloadPrepared(v0, v1) => update_panel(app, PanelMessage::AppUpdateDownloadPrepared(v0, v1)),
        #[cfg(not(target_arch = "wasm32"))]
        ViewMessage::AppUpdatePrepared(v0) => update_panel(app, PanelMessage::AppUpdatePrepared(v0)),
        ViewMessage::OpenAppInstaller => update_panel(app, PanelMessage::OpenAppInstaller),
        ViewMessage::OpenAppInstallerDirectory => update_panel(app, PanelMessage::OpenAppInstallerDirectory),
        ViewMessage::AppInstallerDirectoryOpened(v0) => update_panel(app, PanelMessage::AppInstallerDirectoryOpened(v0)),
        ViewMessage::AppUpdateProgressTick => update_panel(app, PanelMessage::AppUpdateProgressTick),
        ViewMessage::AppUpdateFinished(v0) => update_panel(app, PanelMessage::AppUpdateFinished(v0)),
        ViewMessage::InstallCliToolFinished(v0) => update_panel(app, PanelMessage::InstallCliToolFinished(v0)),
        ViewMessage::CloseInstallCliModal => update_panel(app, PanelMessage::CloseInstallCliModal),
        ViewMessage::ToggleDiffPanel => {
            if app.navigation_mode.workspace_mode == WorkspaceMode::Work {
                return Task::none();
            }
            app.show_diff = !app.show_diff;
            Task::none()
        }
        ViewMessage::ToggleGitDiffSummary => {
            app.git.show_git_diff_summary = !app.git.show_git_diff_summary;
            app.save_desktop_preference("show_git_diff_summary", app.git.show_git_diff_summary.into())
        }
        ViewMessage::ToggleTerminalPanel => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                app.terminal.content_menu = None;
            }
            app.terminal.set_visible(!app.terminal.is_visible);
            if app.terminal.is_visible && app.terminal.height < 160.0 {
                app.terminal.height = 200.0;
            }
            if app.terminal.is_visible {
                #[cfg(not(target_arch = "wasm32"))]
                app.terminal.ensure_default_terminal(
                    app.project_path.as_ref().map(std::path::PathBuf::from),
                );

                #[cfg(not(target_arch = "wasm32"))]
                app.terminal.apply_app_theme(&app.app_theme);
            }
            let save = app.save_desktop_preference("show_terminal", app.terminal.is_visible.into());
            #[cfg(not(target_arch = "wasm32"))]
            if app.terminal.is_visible {
                let focus = crate::app::message::terminal::event::focus_active_terminal(app);
                return Task::batch([save, focus]);
            }
            save
        }
        ViewMessage::FileManagerPanelVisible(b) => {
            app.file_manager.show_file_manager = b;
            if !b {
                app.layout_drag.dragging_file_manager = false;
                app.layout_drag.file_manager_drag_anchor_x = None;
            }
            app.save_desktop_preference("show_file_manager", b.into())
        }
        ViewMessage::OpenTerminalPressed => {
            if !crate::app::config::gateway_client_endpoint().is_loopback() {
                return app.show_error_toast("远端终端只能通过网关 PTY 打开");
            }
            if let Some(_path) = crate::app::components::git_panel::git_repo_path_for_app(app) {
                #[cfg(not(target_arch = "wasm32"))]
                if let Err(error) = crate::app::components::git_panel::open_terminal(&_path) {
                    return app.show_error_toast(format!("无法打开终端：{error}"));
                }
            } else {
                return app.show_error_toast("没有可打开终端的本机仓库目录");
            }
            Task::none()
        }
        ViewMessage::AutoMaxToggled(b) => {
            app.auto_max_mode = b;
            app.save_desktop_preference("auto_max_mode", b.into())
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
        if app.close_to_tray && !app.shutdown_requested {
            if let Err(error) = crate::app::desktop_lifecycle::ensure_tray(app.desktop_locale) {
                return app.show_error_toast(format!("无法关闭到托盘：{error}"));
            }
            #[cfg(target_os = "macos")]
            return app.release_main_window();
            #[cfg(not(target_os = "macos"))]
            return crate::app::desktop_lifecycle::hide_main_window(window_id);
        }
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
pub(crate) fn start_shutdown(app: &mut App, window_id: iced::window::Id) -> Task<Message> {
    shutdown(app, Some(window_id))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn shutdown(app: &mut App, window_id: Option<iced::window::Id>) -> Task<Message> {
    if !begin_shutdown(&mut app.shutdown_requested) {
        return Task::none();
    }

    app.voice.hotkeys.reload();
    crate::app::message::voice::cancel(app);
    arm_shutdown_exit_watchdog();
    let requests = take_shutdown_stream_requests(app);
    let editor_commit = crate::app::message::editor::metrics::commit(app, None);
    let editor_saves = app.editor_config_persistence.wait_for_idle();
    let (routing_commit, routing_saves) = crate::app::message::settings::routing_persistence::flush(app);
    let worktree_saves = crate::app::message::settings::wait_for_worktree_saves(app);
    let preference_saves = app.desktop_preferences_persistence.wait_for_idle();
    app.web_install.runtime.shutdown();
    app.work_browser_controller.shutdown();
    if let Some(manager) = app.lsp_manager.as_mut() {
        manager.shutdown_all();
    }
    for child in &mut app.independent_webview_children {
        let _ = child.kill();
    }
    app.independent_webview_children.clear();

    Task::batch([
        editor_commit,
        routing_commit,
        window_id
            .map_or_else(Task::none, |id| iced::window::set_mode(id, iced::window::Mode::Hidden)),
        Task::perform(
            async move {
                tokio::join!(
                    stop_shutdown_streams(requests),
                    wait_for_editor_settings_saves(editor_saves),
                    wait_for_settings_saves(routing_saves),
                    wait_for_settings_saves(worktree_saves),
                    wait_for_settings_saves(preference_saves)
                );
            },
            |_| Message::View(ViewMessage::ShutdownFinished),
        ),
    ])
}

#[cfg(not(target_arch = "wasm32"))]
async fn wait_for_settings_saves(saves: Option<tokio::sync::watch::Receiver<bool>>) {
    if let Some(mut saves) = saves {
        let _ = tokio::time::timeout(SHUTDOWN_TIMEOUT, async {
            while !*saves.borrow_and_update() {
                if saves.changed().await.is_err() {
                    break;
                }
            }
        })
        .await;
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn wait_for_editor_settings_saves(editor_saves: Option<tokio::sync::oneshot::Receiver<()>>) {
    if let Some(saves) = editor_saves
        && !matches!(tokio::time::timeout(SHUTDOWN_TIMEOUT, saves).await, Ok(Ok(())))
    {
        tracing::warn!(target: "vw_desktop", "exiting before editor settings saves finished");
    }
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
        .filter_map(|runtime| runtime.request_lifecycle.active_agent_request.take())
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
                    let _ = gateway_session::GatewaySessionClient::new(client.transport().clone()).chat_stream_stop(&request.request_id).await;
                }
                ShutdownStreamKind::TaskPlanRun => {
                    let _ = gateway_task_pool::GatewayTaskPoolClient::new(client.transport().clone()).task_plan_run_stream_stop(&request.request_id).await;
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

#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;

fn update_panel(app: &mut App, message: PanelMessage) -> Task<Message> {
    // Move the projection through the domain update without cloning strings or receivers.
    let mut panel = PanelState {
        show_cli_install_modal: std::mem::take(&mut app.show_cli_install_modal),
        cli_install_modal_title: std::mem::take(&mut app.cli_install_modal_title),
        cli_install_modal_message: std::mem::take(&mut app.cli_install_modal_message),
        cli_install_modal_current_version: std::mem::take(&mut app.cli_install_modal_current_version),
        cli_install_modal_server_version: std::mem::take(&mut app.cli_install_modal_server_version),
        cli_install_modal_show_update_action: std::mem::take(&mut app.cli_install_modal_show_update_action),
        cli_install_modal_show_install_action: std::mem::take(&mut app.cli_install_modal_show_install_action),
        cli_install_modal_use_app_update_action: std::mem::take(&mut app.cli_install_modal_use_app_update_action),
        cli_install_modal_is_checking_update: std::mem::take(&mut app.cli_install_modal_is_checking_update),
        app_update_progress: std::mem::take(&mut app.app_update_progress),
        app_update_progress_rx: std::mem::take(&mut app.app_update_progress_rx),
    };
    let mut effects = Vec::new();
    let task = desktop_update::panel::update(&mut panel, &mut app.desktop_update, &mut effects, env!("CARGO_PKG_VERSION"), #[cfg(not(target_arch = "wasm32"))] app.desktop_locale, message);
    app.show_cli_install_modal = panel.show_cli_install_modal;
    app.cli_install_modal_title = panel.cli_install_modal_title;
    app.cli_install_modal_message = panel.cli_install_modal_message;
    app.cli_install_modal_current_version = panel.cli_install_modal_current_version;
    app.cli_install_modal_server_version = panel.cli_install_modal_server_version;
    app.cli_install_modal_show_update_action = panel.cli_install_modal_show_update_action;
    app.cli_install_modal_show_install_action = panel.cli_install_modal_show_install_action;
    app.cli_install_modal_use_app_update_action = panel.cli_install_modal_use_app_update_action;
    app.cli_install_modal_is_checking_update = panel.cli_install_modal_is_checking_update;
    app.app_update_progress = panel.app_update_progress;
    app.app_update_progress_rx = panel.app_update_progress_rx;
    let mut tasks = vec![task.map(|message| Message::View(panel_message(message)))];
    for effect in effects {
        tasks.push(match effect {
            Effect::CloseMenu => { app.active_menu = None; Task::none() }
            Effect::SaveSource(source) => Task::perform(
                crate::app::config::save_app_config_async(crate::app::desktop_update::source_patch(source)),
                |result| Message::View(ViewMessage::AppUpdateSourceSaved(result)),
            ),
            Effect::Error(error) => app.show_error_toast(error),
            Effect::Shutdown => { let window_id = app.main_window_id; shutdown(app, window_id) },
        });
    }
    Task::batch(tasks)
}
fn panel_message(message: PanelMessage) -> ViewMessage {
    match message {
        PanelMessage::InstallCliTool => ViewMessage::InstallCliTool,
        PanelMessage::RunInstallCliTool => ViewMessage::RunInstallCliTool,
        PanelMessage::CheckCliToolUpdate => ViewMessage::CheckCliToolUpdate,
        PanelMessage::CheckCliToolUpdateFinished(v0, v1, v2) => ViewMessage::CheckCliToolUpdateFinished(v0, v1, v2),
        PanelMessage::OpenAppUpdateModal => ViewMessage::OpenAppUpdateModal,
        PanelMessage::AppUpdateSourceSelected(v0) => ViewMessage::AppUpdateSourceSelected(v0),
        PanelMessage::AppUpdateSourceSaved(v0) => ViewMessage::AppUpdateSourceSaved(v0),
        PanelMessage::CheckAppUpdate => ViewMessage::CheckAppUpdate,
        PanelMessage::CheckAppUpdateFinished(v0, v1, v2) => ViewMessage::CheckAppUpdateFinished(v0, v1, v2),
        PanelMessage::RunAppUpdate => ViewMessage::RunAppUpdate,
        PanelMessage::CancelAppUpdate => ViewMessage::CancelAppUpdate,
        #[cfg(not(target_arch = "wasm32"))]
        PanelMessage::AppUpdateDownloadPrepared(v0, v1) => ViewMessage::AppUpdateDownloadPrepared(v0, v1),
        #[cfg(not(target_arch = "wasm32"))]
        PanelMessage::AppUpdatePrepared(v0) => ViewMessage::AppUpdatePrepared(v0),
        PanelMessage::OpenAppInstaller => ViewMessage::OpenAppInstaller,
        PanelMessage::OpenAppInstallerDirectory => ViewMessage::OpenAppInstallerDirectory,
        PanelMessage::AppInstallerDirectoryOpened(v0) => ViewMessage::AppInstallerDirectoryOpened(v0),
        PanelMessage::AppUpdateProgressTick => ViewMessage::AppUpdateProgressTick,
        PanelMessage::AppUpdateFinished(v0) => ViewMessage::AppUpdateFinished(v0),
        PanelMessage::InstallCliToolFinished(v0) => ViewMessage::InstallCliToolFinished(v0),
        PanelMessage::CloseInstallCliModal => ViewMessage::CloseInstallCliModal,
    }
}

//! 处理设置面板、项目配置与桌面窗口关闭流程的消息。

use super::ViewMessage;
use crate::app::message::settings::McpMessage;
use crate::app::message::{ProjectMessage, SettingsMessage};
use crate::app::{App, Message};
use desktop_update::panel::{Effect, PanelMessage};
use iced::Task;
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

pub(crate) use desktop_navigation::window_lifecycle::begin_shutdown;

/// 处理设置、更新、项目与窗口生命周期相关的面板消息。
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    if let Some(local) = navigation_message(&message) {
        return update_navigation(app, local);
    }
    match message {
        ViewMessage::RestartApp => {
            app.navigation_panels.close_menu();
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
                let (routing_commit, routing_saves) =
                    crate::app::message::settings::routing_persistence::flush(app);
                (
                    commit,
                    app.preview_state.editor_config_persistence.wait_for_idle(),
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
                    app.input_feedback.error_message = Some("当前平台不支持重启应用".to_string());
                    Task::none()
                }
            }
            Err(error) => {
                app.shutdown_requested = false;
                app.input_feedback.error_message = Some(error);
                Task::none()
            }
        },
        ViewMessage::InstallCliTool => update_panel(app, PanelMessage::InstallCliTool),
        ViewMessage::RunInstallCliTool => update_panel(app, PanelMessage::RunInstallCliTool),
        ViewMessage::CheckCliToolUpdate => update_panel(app, PanelMessage::CheckCliToolUpdate),
        ViewMessage::CheckCliToolUpdateFinished(v0, v1, v2) => {
            update_panel(app, PanelMessage::CheckCliToolUpdateFinished(v0, v1, v2))
        }
        ViewMessage::OpenAppUpdateModal => update_panel(app, PanelMessage::OpenAppUpdateModal),
        ViewMessage::AppUpdateSourceSelected(v0) => {
            update_panel(app, PanelMessage::AppUpdateSourceSelected(v0))
        }
        ViewMessage::AppUpdateSourceSaved(v0) => {
            update_panel(app, PanelMessage::AppUpdateSourceSaved(v0))
        }
        ViewMessage::CheckAppUpdate => update_panel(app, PanelMessage::CheckAppUpdate),
        ViewMessage::CheckAppUpdateFinished(v0, v1, v2) => {
            update_panel(app, PanelMessage::CheckAppUpdateFinished(v0, v1, v2))
        }
        ViewMessage::RunAppUpdate => update_panel(app, PanelMessage::RunAppUpdate),
        ViewMessage::CancelAppUpdate => update_panel(app, PanelMessage::CancelAppUpdate),
        #[cfg(not(target_arch = "wasm32"))]
        ViewMessage::AppUpdateDownloadPrepared(v0, v1) => {
            update_panel(app, PanelMessage::AppUpdateDownloadPrepared(v0, v1))
        }
        #[cfg(not(target_arch = "wasm32"))]
        ViewMessage::AppUpdatePrepared(v0) => {
            update_panel(app, PanelMessage::AppUpdatePrepared(v0))
        }
        ViewMessage::OpenAppInstaller => update_panel(app, PanelMessage::OpenAppInstaller),
        ViewMessage::OpenAppInstallerDirectory => {
            update_panel(app, PanelMessage::OpenAppInstallerDirectory)
        }
        ViewMessage::AppInstallerDirectoryOpened(v0) => {
            update_panel(app, PanelMessage::AppInstallerDirectoryOpened(v0))
        }
        ViewMessage::AppUpdateProgressTick => {
            update_panel(app, PanelMessage::AppUpdateProgressTick)
        }
        ViewMessage::AppUpdateFinished(v0) => {
            update_panel(app, PanelMessage::AppUpdateFinished(v0))
        }
        ViewMessage::InstallCliToolFinished(v0) => {
            update_panel(app, PanelMessage::InstallCliToolFinished(v0))
        }
        ViewMessage::CloseInstallCliModal => update_panel(app, PanelMessage::CloseInstallCliModal),
        ViewMessage::ToggleTerminalPanel => {
            app.terminal.toggle_panel();
            if app.terminal.is_visible {
                #[cfg(not(target_arch = "wasm32"))]
                app.terminal.ensure_default_terminal(
                    app.navigation_tabs.project_path.as_ref().map(std::path::PathBuf::from),
                );

                #[cfg(not(target_arch = "wasm32"))]
                app.terminal.apply_app_theme(&app.appearance.app_theme);
            }
            let save = app.save_desktop_preference("show_terminal", app.terminal.is_visible.into());
            #[cfg(not(target_arch = "wasm32"))]
            if app.terminal.is_visible {
                let focus = crate::app::message::terminal::event::focus_active_terminal(app);
                return Task::batch([save, focus]);
            }
            save
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
        ViewMessage::MenuAction(msg) => {
            let _ = update_navigation(app, desktop_navigation::panel::settings::Message::MenuAction);
            Task::done(*msg)
        }
        _ => Task::none(),
    }
}

/// 关闭宠物窗口或启动一次有超时保护的应用退出流程。
pub fn close_requested(app: &mut App, window_id: iced::window::Id) -> Task<Message> {
    let windows = close_requested_windows(app.main_window_id, app.task_pet_window_id, window_id);

    if windows.main_window_id.is_some() {
        if app.lifecycle.close_to_tray && !app.shutdown_requested {
            if let Err(error) = crate::app::desktop_lifecycle::ensure_tray(app.general_settings.desktop_locale) {
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
    let editor_saves = app.preview_state.editor_config_persistence.wait_for_idle();
    let (routing_commit, routing_saves) =
        crate::app::message::settings::routing_persistence::flush(app);
    let worktree_saves = crate::app::message::settings::wait_for_worktree_saves(app);
    let preference_saves = app.desktop_preferences_persistence.wait_for_idle();
    app.web_install.runtime.shutdown();
    app.work_browser_controller.shutdown();
    if let Some(manager) = app.lsp_manager.as_mut() {
        manager.shutdown_all();
    }
    for child in &mut app.web_navigation.independent_webview_children {
        let _ = child.kill();
    }
    app.web_navigation.independent_webview_children.clear();

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
    let requests = app.session_state.session_runtime_states
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
    app.session_state.active_agent_request = None;
    app.session_state.is_requesting = false;
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
                    let _ = gateway_session::GatewaySessionClient::new(client.transport().clone())
                        .chat_stream_stop(&request.request_id)
                        .await;
                }
                ShutdownStreamKind::TaskPlanRun => {
                    let _ =
                        gateway_task_pool::GatewayTaskPoolClient::new(client.transport().clone())
                            .task_plan_run_stream_stop(&request.request_id)
                            .await;
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
    let mut effects = Vec::new();
    let task = desktop_update::panel::update(
        &mut app.update_panel,
        &mut app.desktop_update,
        &mut effects,
        env!("CARGO_PKG_VERSION"),
        #[cfg(not(target_arch = "wasm32"))]
        app.general_settings.desktop_locale,
        message,
    );
    let mut tasks = vec![task.map(|message| Message::View(panel_message(message)))];
    for effect in effects {
        tasks.push(match effect {
            Effect::CloseMenu => {
                app.navigation_panels.close_menu();
                Task::none()
            }
            Effect::SaveSource(source) => Task::perform(
                crate::app::config::save_app_config_async(
                    crate::app::desktop_update::source_patch(source),
                ),
                |result| Message::View(ViewMessage::AppUpdateSourceSaved(result)),
            ),
            Effect::Error(error) => app.show_error_toast(error),
            Effect::Shutdown => {
                let window_id = app.main_window_id;
                shutdown(app, window_id)
            }
        });
    }
    Task::batch(tasks)
}
fn panel_message(message: PanelMessage) -> ViewMessage {
    match message {
        PanelMessage::InstallCliTool => ViewMessage::InstallCliTool,
        PanelMessage::RunInstallCliTool => ViewMessage::RunInstallCliTool,
        PanelMessage::CheckCliToolUpdate => ViewMessage::CheckCliToolUpdate,
        PanelMessage::CheckCliToolUpdateFinished(v0, v1, v2) => {
            ViewMessage::CheckCliToolUpdateFinished(v0, v1, v2)
        }
        PanelMessage::OpenAppUpdateModal => ViewMessage::OpenAppUpdateModal,
        PanelMessage::AppUpdateSourceSelected(v0) => ViewMessage::AppUpdateSourceSelected(v0),
        PanelMessage::AppUpdateSourceSaved(v0) => ViewMessage::AppUpdateSourceSaved(v0),
        PanelMessage::CheckAppUpdate => ViewMessage::CheckAppUpdate,
        PanelMessage::CheckAppUpdateFinished(v0, v1, v2) => {
            ViewMessage::CheckAppUpdateFinished(v0, v1, v2)
        }
        PanelMessage::RunAppUpdate => ViewMessage::RunAppUpdate,
        PanelMessage::CancelAppUpdate => ViewMessage::CancelAppUpdate,
        #[cfg(not(target_arch = "wasm32"))]
        PanelMessage::AppUpdateDownloadPrepared(v0, v1) => {
            ViewMessage::AppUpdateDownloadPrepared(v0, v1)
        }
        #[cfg(not(target_arch = "wasm32"))]
        PanelMessage::AppUpdatePrepared(v0) => ViewMessage::AppUpdatePrepared(v0),
        PanelMessage::OpenAppInstaller => ViewMessage::OpenAppInstaller,
        PanelMessage::OpenAppInstallerDirectory => ViewMessage::OpenAppInstallerDirectory,
        PanelMessage::AppInstallerDirectoryOpened(v0) => {
            ViewMessage::AppInstallerDirectoryOpened(v0)
        }
        PanelMessage::AppUpdateProgressTick => ViewMessage::AppUpdateProgressTick,
        PanelMessage::AppUpdateFinished(v0) => ViewMessage::AppUpdateFinished(v0),
        PanelMessage::InstallCliToolFinished(v0) => ViewMessage::InstallCliToolFinished(v0),
        PanelMessage::CloseInstallCliModal => ViewMessage::CloseInstallCliModal,
    }
}

fn navigation_message(
    message: &ViewMessage,
) -> Option<desktop_navigation::panel::settings::Message> {
    use desktop_navigation::panel::settings::Message as Local;
    Some(match message {
        ViewMessage::ToggleSettingsPanel => Local::ToggleSettingsPanel,
        ViewMessage::ProjectFileNewSession => Local::ProjectFileNewSession,
        ViewMessage::ProjectFileNewProject => Local::ProjectFileNewProject,
        ViewMessage::ProjectFileShowSessions => Local::ProjectFileShowSessions,
        ViewMessage::ProjectFileShowProjects => Local::ProjectFileShowProjects,
        ViewMessage::ProjectFileSaveAll => Local::ProjectFileSaveAll,
        ViewMessage::ToggleSystemSettings => Local::ToggleSystemSettings,
        ViewMessage::OpenAcpSettings => Local::OpenAcpSettings,
        ViewMessage::OpenSystemSettingsTab(value) => Local::OpenSystemSettingsTab(*value),
        ViewMessage::OpenSystemSettingsModelDetail(provider, model) => {
            Local::OpenSystemSettingsModelDetail(provider.clone(), model.clone())
        }
        ViewMessage::ToggleAboutModal => Local::ToggleAboutModal,
        ViewMessage::CliCommandsTabSelected(value) => Local::CliCommandsTabSelected(*value),
        ViewMessage::ToggleDiffPanel => Local::ToggleDiffPanel,
        ViewMessage::ToggleGitDiffSummary => Local::ToggleGitDiffSummary,
        ViewMessage::FileManagerPanelVisible(value) => Local::FileManagerPanelVisible(*value),
        ViewMessage::AutoMaxToggled(value) => Local::AutoMaxToggled(*value),
        ViewMessage::ToggleMenu(value) => Local::ToggleMenu(*value),
        ViewMessage::GatewayServicesTabSelected(value) => Local::GatewayServicesTabSelected(*value),
        ViewMessage::MenuHovered(value) => Local::MenuHovered(*value),
        _ => return None,
    })
}

fn update_navigation(
    app: &mut App,
    message: desktop_navigation::panel::settings::Message,
) -> Task<Message> {
    use desktop_navigation::panel::settings as domain;
    let effect = domain::update(
        domain::Context {
            state: &mut app.navigation_panels,
            popovers: &mut app.popover_visibility,
            drag: &mut app.layout_drag,
            git_diff_summary: &mut app.git.show_git_diff_summary,
            file_manager_visible: &mut app.file_manager.show_file_manager,
            plugin_catalog_empty: app.skills_settings.plugin_catalog.is_empty(),
            plugin_loading: app.skills_settings.plugin_loading,
            mcp_loading: app.mcp_settings.loading,
            mcp_saving: app.mcp_settings.saving,
        },
        app.navigation_mode.workspace_mode,
        app.navigation_tabs.project_path.as_deref(),
        message,
    );
    match effect {
        None => Task::none(),
        Some(domain::Effect::ToggleWorkSidebar) => {
            Task::done(Message::View(ViewMessage::WorkSidebarToggled))
        }
        Some(domain::Effect::SavePreference(key, value)) => app.save_desktop_preference(key, value),
        Some(domain::Effect::SettingsVisibilityChanged(visible)) => {
            let save = app.save_desktop_preference("show_settings", visible.into());
            if visible
                && let Some(path) = app.navigation_tabs.project_path.clone()
                && !app.workspace_state.project_sessions.contains_key(&path)
                && !app.workspace_state.project_sessions_loading.contains(&path)
            {
                app.workspace_state.project_sessions_loading.insert(path.clone());
                let directory = path.clone();
                let sessions = Task::perform(
                    async move {
                        let client = crate::app::gateway_client().map_err(|err| err.to_string())?;
                        crate::app::message::project::helpers::list_gateway_project_sessions(
                            &client, &directory,
                        )
                        .await
                    },
                    |result| Message::Project(ProjectMessage::ProjectSessionsLoaded(path, result)),
                );
                return Task::batch([save, sessions]);
            }
            save
        }
        Some(domain::Effect::CreateSession(path)) => {
            Task::done(Message::Project(ProjectMessage::ProjectCreateSession(path)))
        }
        Some(domain::Effect::NewProject) => {
            Task::done(Message::Project(ProjectMessage::OpenFolderPressed))
        }
        Some(domain::Effect::SaveAllFiles) => {
            app.preview_state.show_preview_context_menu = false;
            Task::batch(
                app.navigation_tabs
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
                    .collect::<Vec<_>>(),
            )
        }
        Some(domain::Effect::SystemSettingsVisibilityChanged(visible)) => {
            app.workspace_state.project_tools_menu_path = None;
            #[cfg(not(target_arch = "wasm32"))]
            {
                app.agent_apps_auth.form = None;
                if visible
                    && app.navigation_panels.system_settings_tab
                        == desktop_settings_shell::SystemTab::Auth
                {
                    return crate::app::message::agent_apps_auth::update(
                        app,
                        crate::app::message::agent_apps_auth::Event::Refresh,
                    );
                }
            }
            if visible {
                Task::none()
            } else {
                crate::app::message::editor::metrics::commit(app, None)
            }
        }
        Some(domain::Effect::OpenAcpSettings) => {
            Task::done(Message::Settings(SettingsMessage::AgentConfigTab(
                crate::app::message::settings::AgentConfigTabMessage::Selected(
                    crate::app::state::AgentConfigTab::Acp,
                ),
            )))
        }
        Some(domain::Effect::SelectSystemTab(tab)) => {
            Task::done(Message::Settings(SettingsMessage::SystemTabSelected(tab)))
        }
        Some(domain::Effect::OpenModelDetail(provider, model)) => {
            app.model_settings.active_tab = crate::app::state::ModelConfigTab::Models;
            Task::done(Message::Settings(SettingsMessage::ModelDetailOpen(provider, model)))
        }
        Some(domain::Effect::RefreshPlugins) => {
            app.skills_settings.plugin_loading = true;
            Task::done(Message::Settings(SettingsMessage::PluginsRefresh))
        }
        Some(domain::Effect::RefreshMcp) => {
            app.mcp_settings.loading = true;
            app.mcp_settings.save_error = None;
            Task::done(Message::Settings(SettingsMessage::Mcp(McpMessage::Refresh)))
        }
    }
}

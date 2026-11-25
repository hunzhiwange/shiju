//! Desktop orchestration for terminal-local updates.
use crate::app::{App, FocusArea, Message};
#[cfg(not(target_arch = "wasm32"))]
pub use desktop_terminal::message::ContentAction;
pub use desktop_terminal::message::TerminalMessage;
use iced::Task;
pub mod config;
pub mod event;
pub mod tabs;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod workspace;
pub(crate) use tabs::reap_retired_task;

pub fn update(app: &mut App, message: TerminalMessage) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    if matches!(message, TerminalMessage::CheckpointWorkspace) {
        if !app.shutdown_requested {
            workspace::checkpoint(app);
        }
        return Task::none();
    }

    #[cfg(not(target_arch = "wasm32"))]
    match &message {
        TerminalMessage::ToggleProjects => {
            app.terminal_projects_collapsed = !app.terminal_projects_collapsed;
            let save = app.save_desktop_preference(
                "terminal_projects_collapsed",
                app.terminal_projects_collapsed.into(),
            );
            return Task::batch([save, crate::app::workbench_projects::ensure_terminal_projects(app)]);
        }
        TerminalMessage::SelectProject(path) => {
            let directory = std::path::Path::new(path);
            if !app.terminal.is_visible
                || !(app.navigation_tabs.project_path.as_ref() == Some(path)
                    || app.workspace_state.recent_projects.contains(path)
                    || app.workbench_projects.terminal_projects().is_some_and(|projects| projects.contains_key(path)))
                || app.workbench_projects.active_removed().is_some_and(|removed| removed.contains(path))
            {
                return Task::none();
            }
            if !directory.is_absolute() || !directory.is_dir() {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "项目目录不存在或不可访问",
                    vec![],
                );
            }
            if !app.select_terminal_project(path.clone()) {
                return Task::none();
            }
            return Task::batch([
                reap_retired_task(app),
                event::focus_active_terminal(app),
            ]);
        }
        _ => {}
    }

    let project_path = app.navigation_tabs.project_path.as_deref();
    #[cfg(not(target_arch = "wasm32"))]
    let project_path = app.terminal_project_override.as_deref().or(project_path);
    #[cfg(not(target_arch = "wasm32"))]
    let dock_tab = matches!(message, TerminalMessage::DockTab(..));
    #[cfg(not(target_arch = "wasm32"))]
    let group_change = match &message {
        TerminalMessage::NewGroup | TerminalMessage::SelectGroup(_) => Some(None),
        TerminalMessage::CloseGroup(id) => Some(Some(*id)),
        _ => None,
    };
    let fullscreen = app.terminal.is_fullscreen;
    let mut focused = app.focus_area == FocusArea::Terminal;
    let mut effects = Vec::new();
    let task = desktop_terminal::updates::update(
        &mut app.terminal,
        &mut app.terminal_config_persistence,
        &app.appearance.app_theme,
        project_path,
        app.general_settings.desktop_locale,
        app.cursor_position.y,
        &mut focused,
        &mut effects,
        message,
    )
    .map(Message::Terminal);
    #[cfg(not(target_arch = "wasm32"))]
    if dock_tab && !app.shutdown_requested {
        workspace::checkpoint(app);
    }
    if focused {
        app.focus_area = FocusArea::Terminal;
    } else if app.focus_area == FocusArea::Terminal {
        app.focus_area = FocusArea::None;
    }
    let mut tasks = vec![task];
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(closed) = group_change {
        let directory = app.terminal_project_path().map(str::to_owned);
        app.terminal_ssh.sync_terminal_group(
            directory.as_deref(),
            &app.terminal,
            closed,
        );
    }
    for effect in effects {
        use desktop_terminal::updates::Effect;
        tasks.push(match effect {
            #[cfg(not(target_arch = "wasm32"))]
            Effect::Focus => {
                for tab in &mut app.navigation_tabs.preview_tabs {
                    tab.editor.inner.lose_focus();
                }
                Task::none()
            }
            #[cfg(not(target_arch = "wasm32"))]
            Effect::OpenPath { path, directory } => {
                if crate::app::config::gateway_client_endpoint().is_loopback() {
                    open_path(&path, directory.as_deref())
                } else {
                    Task::none()
                }
            }
            Effect::Delay(duration, message) => super::after(duration, Message::Terminal(message)),
            Effect::SaveConfig(patch) => {
                let client = crate::app::config::gateway_client();
                Task::perform(
                    async move {
                        gateway_desktop_settings::GatewayDesktopSettingsClient::new(
                            client?.transport().clone(),
                        )
                        .desktop_system_settings_patch(&patch)
                        .await
                    },
                    |result| Message::Terminal(TerminalMessage::ConfigSaved(result)),
                )
            }
            Effect::Error(error) => app.show_message_toast(crate::app::state::ToastKind::Error, desktop_i18n::input::feedback(&(error))),
            Effect::LocalizedError { source, arguments } => app.show_localized_toast(crate::app::state::ToastKind::Error, source, arguments),
            Effect::SettingEdited(key) => {
                #[cfg(not(target_arch = "wasm32"))]
                app.note_startup_setting_edit(key);
                #[cfg(target_arch = "wasm32")]
                let _ = key;
                Task::none()
            }
            Effect::SplitFailed => app.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "无法启动拆分终端",
                vec![],
            ),
            Effect::AppendChat(text) => {
                let append = super::chat::update(app, super::ChatMessage::AppendText(text));
                Task::batch([append, iced::widget::operation::focus(app.session_state.input_editor_id.clone())])
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    if fullscreen != app.terminal.is_fullscreen {
        tasks.push(super::view::panel::sync_work_browser(app));
    }
    #[cfg(target_arch = "wasm32")]
    let _ = fullscreen;
    Task::batch(tasks)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn open_path(path: &str, directory: Option<&std::path::Path>) -> Task<Message> {
    use super::ViewMessage;
    use desktop_chat_view::message_view::workspace_links::{LinkEffect, LinkTarget, link_click_message};

    let target = if path.get(..7).is_some_and(|prefix| prefix.eq_ignore_ascii_case("file://")) {
        LinkTarget::parse(path)
    } else {
        LinkTarget::local_path(path)
    };
    let directory = directory.and_then(std::path::Path::to_str);
    match target.map(|target| link_click_message(target, directory)) {
        Some(LinkEffect::OpenChatPath(path)) => {
            Task::done(Message::View(ViewMessage::OpenChatPath(path)))
        }
        Some(LinkEffect::Unresolved(path)) => {
            Task::done(Message::View(ViewMessage::ChatPathKindResolved {
                path,
                result: Err("无法确定本地路径的绝对位置".to_owned()),
            }))
        }
        _ => Task::none(),
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod focus_tests;

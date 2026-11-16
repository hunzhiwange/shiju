//! Desktop orchestration for terminal-local updates.
use crate::app::{App, FocusArea, Message};
#[cfg(not(target_arch = "wasm32"))]
pub use desktop_terminal::message::ContentAction;
pub use desktop_terminal::message::TerminalMessage;
use iced::Task;
pub mod config;
pub mod event;
pub mod tabs;
pub(crate) use tabs::reap_retired_task;

pub fn update(app: &mut App, message: TerminalMessage) -> Task<Message> {
    let fullscreen = app.terminal.is_fullscreen;
    let mut focused = app.focus_area == FocusArea::Terminal;
    let mut effects = Vec::new();
    let task = desktop_terminal::updates::update(
        &mut app.terminal,
        &mut app.terminal_config_persistence,
        &app.app_theme,
        app.navigation_tabs.project_path.as_deref(),
        app.desktop_locale,
        app.cursor_position.y,
        &mut focused,
        &mut effects,
        message,
    )
    .map(Message::Terminal);
    if focused {
        app.focus_area = FocusArea::Terminal;
    } else if app.focus_area == FocusArea::Terminal {
        app.focus_area = FocusArea::None;
    }
    let mut tasks = vec![task];
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
            Effect::Error(error) => app.show_error_toast(error),
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
                Task::batch([append, iced::widget::operation::focus(app.input_editor_id.clone())])
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
#[cfg(all(test, not(target_arch = "wasm32")))]
mod focus_tests;

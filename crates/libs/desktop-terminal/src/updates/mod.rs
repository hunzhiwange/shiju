// Terminal-local updates. Effects are applied synchronously by the desktop shell.
use iced::Task;
use crate::message::TerminalMessage;
type TerminalState = crate::TerminalState<crate::Shell>;
pub mod config;
pub mod rename;
pub mod tabs;
pub mod event;
#[cfg(not(target_arch = "wasm32"))]
pub mod panes;
#[cfg(not(target_arch = "wasm32"))]
pub mod content_menu;
pub enum Effect {
    #[cfg(not(target_arch = "wasm32"))]
    Focus,
    SaveConfig(serde_json::Value),
    Delay(std::time::Duration, TerminalMessage),
    Error(String),
    SettingEdited(&'static str),
    SplitFailed,
    AppendChat(String),
}
pub fn update(
    terminal: &mut TerminalState,
    persistence: &mut config::ConfigPersistence,
    theme: &iced::Theme,
    project_path: Option<&str>,
    locale: config_ui::DesktopLocale,
    cursor_y: f32,
    focused: &mut bool,
    effects: &mut Vec<Effect>,
    message: TerminalMessage,
) -> Task<TerminalMessage> {
    match message {
        TerminalMessage::Add
        | TerminalMessage::Select(_)
        | TerminalMessage::Close(_)
        | TerminalMessage::ReapRetired
        | TerminalMessage::TabContextOpen(_, _, _)
        | TerminalMessage::TabContextClose => tabs::update(terminal, theme, project_path, focused, effects, message),

        TerminalMessage::ThemeSelected(_)
        | TerminalMessage::FontSelected(_)
        | TerminalMessage::FontSizeChanged(_)
        | TerminalMessage::FontSizePreview(_)
        | TerminalMessage::FontSizePreviewCommit
        | TerminalMessage::FontSizePreviewCommitDue(_)
        | TerminalMessage::ShellSelected(_)
        | TerminalMessage::ConfigSaveDue(_)
        | TerminalMessage::ConfigSaved(_) => config::update(terminal, persistence, theme, effects, message),

        TerminalMessage::RenameStart(_)
        | TerminalMessage::RenameChanged(_, _)
        | TerminalMessage::RenameSave(_)
        | TerminalMessage::RenameCancel(_) => rename::update(terminal, locale, message),

        #[cfg(not(target_arch = "wasm32"))]
        TerminalMessage::AddToPane(_)
        | TerminalMessage::ToggleSplitMenu(_)
        | TerminalMessage::CloseSplitMenu
        | TerminalMessage::Split(..)
        | TerminalMessage::ResizePane(..)
        | TerminalMessage::DockPane(..)
        | TerminalMessage::TogglePaneMaximized(..) => panes::update(terminal, theme, project_path, focused, effects, message),
        TerminalMessage::Event(_) | TerminalMessage::DragStarted => event::update(terminal, cursor_y, focused, effects, message),
        #[cfg(not(target_arch = "wasm32"))]
        TerminalMessage::ToggleFullscreen => event::update(terminal, cursor_y, focused, effects, message),
        #[cfg(not(target_arch = "wasm32"))]
        TerminalMessage::ContentMenuOpen(..)
        | TerminalMessage::ContentMenuClose
        | TerminalMessage::ContentMenuAction(..)
        | TerminalMessage::PasteResolved(..) => content_menu::update(terminal, theme, project_path, locale, focused, effects, message),
    }

}
fn after(effects: &mut Vec<Effect>, duration: std::time::Duration, message: TerminalMessage) -> Task<TerminalMessage> {
    effects.push(Effect::Delay(duration, message));
    Task::none()
}

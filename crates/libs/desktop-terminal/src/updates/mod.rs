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
    LocalizedError { source: &'static str, arguments: Vec<desktop_i18n::format::Argument> },
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
        #[cfg(not(target_arch = "wasm32"))]
        TerminalMessage::CheckpointWorkspace
        | TerminalMessage::SelectProject(_)
        | TerminalMessage::ToggleProjects => Task::none(),
        #[cfg(not(target_arch = "wasm32"))]
        TerminalMessage::NewGroup | TerminalMessage::SelectGroup(_) | TerminalMessage::CloseGroup(_) => {
            let changed = match message {
                TerminalMessage::NewGroup => terminal.new_group(),
                TerminalMessage::SelectGroup(id) => terminal.select_group(id),
                TerminalMessage::CloseGroup(id) => terminal.close_group(id),
                _ => unreachable!(),
            };
            if !changed {
                return Task::none();
            }
            if terminal.ensure_default_terminal(project_path.map(std::path::PathBuf::from)) {
                terminal.apply_app_theme(theme);
            }
            Task::batch([
                tabs::reap_retired_task(terminal),
                event::focus_active_terminal(terminal, focused, effects),
            ])
        }
        #[cfg(not(target_arch = "wasm32"))]
        TerminalMessage::GroupMenuOpen(..)
        | TerminalMessage::GroupMenuClose
        | TerminalMessage::RenameGroupStart(..)
        | TerminalMessage::RenameGroupChanged(_)
        | TerminalMessage::RenameGroupSave
        | TerminalMessage::RenameGroupCancel => {
            match message {
                TerminalMessage::GroupMenuOpen(id, point) if terminal.has_group(id) => {
                    terminal.tab_context_menu_id = None;
                    terminal.tab_context_menu_pos = None;
                    terminal.content_menu = None;
                    terminal.panes.split_menu = None;
                    terminal.group_menu = Some((id, point));
                }
                TerminalMessage::GroupMenuClose => terminal.group_menu = None,
                TerminalMessage::RenameGroupStart(id, title) if terminal.has_group(id) => {
                    terminal.group_menu = None;
                    terminal.group_rename = Some((id, title));
                    return iced::widget::operation::focus(crate::terminal_panel::RENAME_INPUT);
                }
                TerminalMessage::RenameGroupChanged(title) => {
                    if let Some((_, draft)) = &mut terminal.group_rename {
                        *draft = title;
                    }
                }
                TerminalMessage::RenameGroupSave => {
                    if let Some((id, draft)) = terminal.group_rename.take() {
                        terminal.rename_group(id, &draft);
                    }
                    return event::focus_active_terminal(terminal, focused, effects);
                }
                TerminalMessage::RenameGroupCancel => {
                    terminal.group_rename = None;
                    return event::focus_active_terminal(terminal, focused, effects);
                }
                _ => {}
            }
            Task::none()
        }
        #[cfg(not(target_arch = "wasm32"))]
        TerminalMessage::PtyEvent(widget, pty_event) => {
            if !terminal.tabs.iter().any(|tab| tab.term.widget_id() == &widget) {
                return Task::none();
            }
            event::update(terminal, cursor_y, focused, effects, TerminalMessage::Event(pty_event))
        }
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

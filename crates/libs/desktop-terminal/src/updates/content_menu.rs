use crate::Shell;
use super::{TerminalMessage, Effect};
type TerminalState = crate::TerminalState<Shell>;
use iced::Task;
use desktop_i18n::I18n;



pub use crate::message::ContentAction;

pub(super) fn update(
    terminal: &mut TerminalState,
    theme: &iced::Theme,
    project_path: Option<&str>,
    locale: config_ui::DesktopLocale,
    focused: &mut bool,
    effects: &mut Vec<Effect>,
    message: TerminalMessage,
) -> Task<TerminalMessage> {
    match message {
        TerminalMessage::ContentMenuOpen(id, position) => {
            if terminal.activate_visible_tab(id) {
                terminal.panes.split_menu = None;
                terminal.tab_context_menu_id = None;
                terminal.tab_context_menu_pos = None;
                terminal.content_menu = Some((id, position));
                return super::event::focus_active_terminal(terminal, focused, effects);
            }
            Task::none()
        }
        TerminalMessage::ContentMenuClose => {
            terminal.content_menu = None;
            Task::none()
        }
        TerminalMessage::ContentMenuAction(id, action) => {
            if !terminal.content_menu.is_some_and(|(menu_id, _)| menu_id == id) {
                return Task::none();
            }
            terminal.content_menu = None;
            run_action(terminal, theme, project_path, locale, focused, effects, id, action)
        }
        TerminalMessage::PasteResolved(widget_id, text) => {
            if let Some(text) = text.filter(|text| !text.is_empty())
                && terminal.is_visible
                && let Some(tab) = terminal.tabs.iter_mut().find(|tab| {
                    terminal.active_id == Some(tab.id)
                        && tab.term.widget_id() == &widget_id
                        && tab.edit_title.is_none()
                })
            {
                tab.term.handle(iced_term::Command::ProxyToBackend(
                    iced_term::BackendCommand::Paste(text),
                ));
            }
            Task::none()
        }
        _ => unreachable!("unexpected terminal content menu message"),
    }
}

fn run_action(terminal: &mut TerminalState, theme: &iced::Theme, project_path: Option<&str>, locale: config_ui::DesktopLocale, focused: &mut bool, effects: &mut Vec<Effect>, id: u64, action: ContentAction) -> Task<TerminalMessage> {
    if !terminal.is_visible || terminal.active_id != Some(id) {
        return Task::none();
    }
    let Some(tab) = terminal.tabs.iter_mut().find(|tab| tab.id == id) else {
        return Task::none();
    };

    match action {
        ContentAction::NewTerminal => super::tabs::update(terminal, theme, project_path, focused, effects, TerminalMessage::Add),
        ContentAction::Split(direction) => {
            let panes = &terminal.panes;
            let message = TerminalMessage::Split(panes.instance.clone(), panes.focused, direction);
            super::panes::update(terminal, theme, project_path, focused, effects, message)
        }
        ContentAction::Copy => {
            let text = tab.term.selected_text();
            if text.is_empty() { Task::none() } else { iced::clipboard::write(text) }
        }
        ContentAction::Paste | ContentAction::PasteText => {
            let widget_id = tab.term.widget_id().clone();
            let resolve = move |text| {
                TerminalMessage::PasteResolved(widget_id.clone(), text)
            };
            if matches!(action, ContentAction::PasteText) {
                return iced::clipboard::read().map(resolve);
            }
            Task::future(async { clipboard_file_paths() }).then(move |paths| {
                let resolve = resolve.clone();
                match paths {
                    Some(text) => Task::done(resolve(Some(text))),
                    None => iced::clipboard::read().map(resolve),
                }
            })
        }
        ContentAction::SelectAll | ContentAction::Clear => {
            let command = match action {
                ContentAction::SelectAll => iced_term::BackendCommand::SelectAll,
                _ => iced_term::BackendCommand::Clear,
            };
            tab.term.handle(iced_term::Command::ProxyToBackend(command));
            Task::none()
        }
        ContentAction::AskInChat | ContentAction::InsertInChat => {
            let selection = tab.term.selected_text();
            let text = if matches!(action, ContentAction::AskInChat) {
                let prompt = I18n::new(locale).__("请帮我排查这个终端问题。");
                if selection.is_empty() {
                    prompt.to_string()
                } else {
                    format!("{prompt}\n{selection}")
                }
            } else {
                selection
            };
            if text.is_empty() {
                return Task::none();
            }
            *focused = false;
            effects.push(Effect::AppendChat(text));
            Task::none()
        }
        ContentAction::CloseTab => super::tabs::close_tab(terminal, focused, effects, id),
    }
}

fn clipboard_file_paths() -> Option<String> {
    let paths = arboard::Clipboard::new().ok()?.get().file_list().ok()?;
    if paths.is_empty() {
        return None;
    }
    let mut text = String::new();
    for path in &paths {
        // File names with terminal control characters cannot safely be typed into a PTY.
        let Some(path) = path.to_str().filter(|path| !path.chars().any(char::is_control)) else {
            continue;
        };
        text.push('\'');
        text.push_str(&path.replace('\'', "'\\''"));
        text.push_str("' ");
    }
    Some(text)
}

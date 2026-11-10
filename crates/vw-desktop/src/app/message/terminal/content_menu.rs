use iced::{Task, widget::operation};

use crate::app::i18n::I18n;
use crate::app::message::ChatMessage;
use crate::app::{App, FocusArea, Message};

use super::TerminalMessage;

#[derive(Debug, Clone, Copy)]
pub enum ContentAction {
    NewTerminal,
    Copy,
    Paste,
    PasteText,
    SelectAll,
    Clear,
    AskInChat,
    InsertInChat,
    CloseTab,
}

pub(super) fn update(app: &mut App, message: TerminalMessage) -> Task<Message> {
    match message {
        TerminalMessage::ContentMenuOpen(id, position) => {
            if app.terminal.activate_visible_tab(id) {
                app.terminal.panes.split_menu = None;
                app.terminal.tab_context_menu_id = None;
                app.terminal.tab_context_menu_pos = None;
                app.terminal.content_menu = Some((id, position));
                return super::event::focus_active_terminal(app);
            }
            Task::none()
        }
        TerminalMessage::ContentMenuClose => {
            app.terminal.content_menu = None;
            Task::none()
        }
        TerminalMessage::ContentMenuAction(id, action) => {
            if !app.terminal.content_menu.is_some_and(|(menu_id, _)| menu_id == id) {
                return Task::none();
            }
            app.terminal.content_menu = None;
            run_action(app, id, action)
        }
        TerminalMessage::PasteResolved(widget_id, text) => {
            if let Some(text) = text.filter(|text| !text.is_empty())
                && app.terminal.is_visible
                && let Some(tab) = app.terminal.tabs.iter_mut().find(|tab| {
                    app.terminal.active_id == Some(tab.id)
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

fn run_action(app: &mut App, id: u64, action: ContentAction) -> Task<Message> {
    if !app.terminal.is_visible || app.terminal.active_id != Some(id) {
        return Task::none();
    }
    let Some(tab) = app.terminal.tabs.iter_mut().find(|tab| tab.id == id) else {
        return Task::none();
    };

    match action {
        ContentAction::NewTerminal => super::tabs::update(app, TerminalMessage::Add),
        ContentAction::Copy => {
            let text = tab.term.selected_text();
            if text.is_empty() { Task::none() } else { iced::clipboard::write(text) }
        }
        ContentAction::Paste | ContentAction::PasteText => {
            let widget_id = tab.term.widget_id().clone();
            let resolve = move |text| {
                Message::Terminal(TerminalMessage::PasteResolved(widget_id.clone(), text))
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
                let prompt = I18n::new(app.desktop_locale).__("请帮我排查这个终端问题。");
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
            app.focus_area = FocusArea::None;
            let append = super::super::chat::update(app, ChatMessage::AppendText(text));
            Task::batch([append, operation::focus(app.input_editor_id.clone())])
        }
        ContentAction::CloseTab => super::tabs::close_tab(app, id),
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

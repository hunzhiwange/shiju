use super::{App, Message, message};
use iced::Element;
#[cfg(test)]
use super::message::chat::ForkSessionTarget;
pub(crate) fn with_chat_fork_dialog<'a>(app: &App, root: Element<'a, Message>) -> Element<'a, Message> {
    desktop_chat_view::modals::chat_fork::with_chat_fork_dialog(app.chat_fork_dialog.as_ref().filter(|dialog| crate::app::message::chat::fork_session_anchor_is_current(&dialog.anchor, app)), app.project_is_git_repo, app.desktop_locale, |event| {
        use desktop_chat_view::modals::chat_fork::Event;
        Message::Chat(match event {
            Event::CloseForkSessionDialog => message::ChatMessage::CloseForkSessionDialog,
            Event::ForkSessionAt { anchor, target } => message::ChatMessage::ForkSessionAt { anchor, target },
        })
    }, root)
}

#[cfg(test)]
#[path = "chat_fork_tests.rs"]
mod chat_fork_tests;

#[cfg(test)]
#[path = "chat_fork_ui_refresh_32_tests.rs"]
mod ui_refresh_32_tests;

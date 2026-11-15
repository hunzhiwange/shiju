use crate::app::{App, Message, message::ViewMessage};
use desktop_navigation::components::about_modal as shared;
use shared::Message as Event;
pub fn view(app: &App) -> iced::Element<'_, Message> {
    shared::view(&shared::Input { desktop_locale: app.desktop_locale, last_copied_code_hash: app.last_copied_code_hash }).map(|event| match event {
        Event::Close => Message::View(ViewMessage::ToggleAboutModal),
        Event::OpenUrl(url) => Message::View(ViewMessage::OpenUrlExternal(url)),
        Event::CopyCode(text) => Message::CopyCode(text),
    })
}

#[cfg(test)]
#[path="about_modal_ui_refresh_41_tests.rs"] mod ui_refresh_41_tests;

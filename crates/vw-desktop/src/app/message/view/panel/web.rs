use super::ViewMessage;
use crate::app::{App, Message};
use iced::Task;
use desktop_apps_ui::web as domain;

pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    let message = match message {
        ViewMessage::OpenUrlExternal(_url) => domain::Message::OpenUrlExternal(_url),
        ViewMessage::OpenWebUrl(_url) => domain::Message::OpenWebUrl(_url),
        ViewMessage::OpenWebUrlWithTitle(_url, _title) => domain::Message::OpenWebUrlWithTitle(_url, _title),
        ViewMessage::OpenWebUrlWithTitleAndSize(_url, _title, _w, _h) => domain::Message::OpenWebUrlWithTitleAndSize(_url, _title, _w, _h),
        ViewMessage::ToggleWebLinksMenu => domain::Message::ToggleWebLinksMenu,
        ViewMessage::WebBookmarkTitleChanged(v) => domain::Message::WebBookmarkTitleChanged(v),
        ViewMessage::WebBookmarkUrlChanged(v) => domain::Message::WebBookmarkUrlChanged(v),
        ViewMessage::WebBookmarkWidthChanged(v) => domain::Message::WebBookmarkWidthChanged(v),
        ViewMessage::WebBookmarkHeightChanged(v) => domain::Message::WebBookmarkHeightChanged(v),
        ViewMessage::WebBookmarkAddSave => domain::Message::WebBookmarkAddSave,
        ViewMessage::WebBookmarkAddCancel => domain::Message::WebBookmarkAddCancel,
        ViewMessage::WebBookmarkEditStart(idx) => domain::Message::WebBookmarkEditStart(idx),
        ViewMessage::WebBookmarkEditTitleChanged(v) => domain::Message::WebBookmarkEditTitleChanged(v),
        ViewMessage::WebBookmarkEditUrlChanged(v) => domain::Message::WebBookmarkEditUrlChanged(v),
        ViewMessage::WebBookmarkEditWidthChanged(v) => domain::Message::WebBookmarkEditWidthChanged(v),
        ViewMessage::WebBookmarkEditHeightChanged(v) => domain::Message::WebBookmarkEditHeightChanged(v),
        ViewMessage::WebBookmarkEditCookieConfigsChanged(action) => domain::Message::WebBookmarkEditCookieConfigsChanged(action),
        ViewMessage::WebBookmarkEditCookieConfigsInsertExample => domain::Message::WebBookmarkEditCookieConfigsInsertExample,
        ViewMessage::WebBookmarkEditSave => domain::Message::WebBookmarkEditSave,
        ViewMessage::WebBookmarkEditCancel => domain::Message::WebBookmarkEditCancel,
        ViewMessage::WebBookmarkRemove(idx) => domain::Message::WebBookmarkRemove(idx),
        _ => return Task::none(),
    };
    let effect = domain::update(&mut app.web_navigation, message);
    match effect {
        Some(domain::Effect::Save(bookmarks)) => app.save_desktop_preference("web_bookmarks", bookmarks),
        Some(domain::Effect::Error(message)) => app.show_localized_toast(crate::app::state::ToastKind::Error, message, vec![]),
        None => Task::none(),
    }
}

#[cfg(test)]
#[path = "web_tests.rs"]
mod web_tests;

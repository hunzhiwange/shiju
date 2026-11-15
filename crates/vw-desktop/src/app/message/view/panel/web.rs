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
    let mut state = domain::State {
        web_bookmarks: std::mem::take(&mut app.web_bookmarks),
        #[cfg(not(target_arch = "wasm32"))]
        independent_webview_children: std::mem::take(&mut app.independent_webview_children),
        show_web_links_menu: std::mem::take(&mut app.show_web_links_menu),
        web_bookmark_title_input: std::mem::take(&mut app.web_bookmark_title_input),
        web_bookmark_url_input: std::mem::take(&mut app.web_bookmark_url_input),
        web_bookmark_width_input: std::mem::take(&mut app.web_bookmark_width_input),
        web_bookmark_height_input: std::mem::take(&mut app.web_bookmark_height_input),
        editing_web_bookmark: std::mem::take(&mut app.editing_web_bookmark),
        edit_web_bookmark_title_input: std::mem::take(&mut app.edit_web_bookmark_title_input),
        edit_web_bookmark_url_input: std::mem::take(&mut app.edit_web_bookmark_url_input),
        edit_web_bookmark_width_input: std::mem::take(&mut app.edit_web_bookmark_width_input),
        edit_web_bookmark_height_input: std::mem::take(&mut app.edit_web_bookmark_height_input),
        edit_web_bookmark_cookie_configs_editor: std::mem::take(&mut app.edit_web_bookmark_cookie_configs_editor),
    };
    let effect = domain::update(&mut state, message);
    { app.web_bookmarks = state.web_bookmarks; }
    #[cfg(not(target_arch = "wasm32"))]
    { app.independent_webview_children = state.independent_webview_children; }
    { app.show_web_links_menu = state.show_web_links_menu; }
    { app.web_bookmark_title_input = state.web_bookmark_title_input; }
    { app.web_bookmark_url_input = state.web_bookmark_url_input; }
    { app.web_bookmark_width_input = state.web_bookmark_width_input; }
    { app.web_bookmark_height_input = state.web_bookmark_height_input; }
    { app.editing_web_bookmark = state.editing_web_bookmark; }
    { app.edit_web_bookmark_title_input = state.edit_web_bookmark_title_input; }
    { app.edit_web_bookmark_url_input = state.edit_web_bookmark_url_input; }
    { app.edit_web_bookmark_width_input = state.edit_web_bookmark_width_input; }
    { app.edit_web_bookmark_height_input = state.edit_web_bookmark_height_input; }
    { app.edit_web_bookmark_cookie_configs_editor = state.edit_web_bookmark_cookie_configs_editor; }
    match effect {
        Some(domain::Effect::Save(bookmarks)) => app.save_desktop_preference("web_bookmarks", bookmarks),
        Some(domain::Effect::Error(message)) => app.show_localized_toast(crate::app::state::ToastKind::Error, message, vec![]),
        None => Task::none(),
    }
}

#[cfg(test)]
#[path = "web_tests.rs"]
mod web_tests;

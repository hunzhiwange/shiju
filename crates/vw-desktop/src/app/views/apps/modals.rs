//! Desktop rich help and message adapters for bookmark forms.
use crate::app::{App, Message};
use crate::app::message::ViewMessage;
use crate::app::i18n::I18n;
use crate::app::components::system_settings_common::{settings_group_heading_with_help, settings_hover_help_label, settings_field_label};
use iced::Element;
use desktop_apps_ui::bookmark_modals::{self as domain, Event};
#[cfg(test)]
const WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE: &str = domain::WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE;
fn message(event: Event) -> Message {
    Message::View(match event {
        Event::WebBookmarkEditCancel => ViewMessage::WebBookmarkEditCancel,
        Event::WebBookmarkEditTitleChanged(value) => ViewMessage::WebBookmarkEditTitleChanged(value),
        Event::WebBookmarkEditUrlChanged(value) => ViewMessage::WebBookmarkEditUrlChanged(value),
        Event::WebBookmarkEditWidthChanged(value) => ViewMessage::WebBookmarkEditWidthChanged(value),
        Event::WebBookmarkEditHeightChanged(value) => ViewMessage::WebBookmarkEditHeightChanged(value),
        Event::WebBookmarkEditCookieConfigsInsertExample => ViewMessage::WebBookmarkEditCookieConfigsInsertExample,
        Event::WebBookmarkEditCookieConfigsChanged(value) => ViewMessage::WebBookmarkEditCookieConfigsChanged(value),
        Event::WebBookmarkRemove(value) => ViewMessage::WebBookmarkRemove(value),
        Event::WebBookmarkEditSave => ViewMessage::WebBookmarkEditSave,
        Event::ToggleWebLinksMenu => ViewMessage::ToggleWebLinksMenu,
        Event::WebBookmarkTitleChanged(value) => ViewMessage::WebBookmarkTitleChanged(value),
        Event::WebBookmarkUrlChanged(value) => ViewMessage::WebBookmarkUrlChanged(value),
        Event::WebBookmarkWidthChanged(value) => ViewMessage::WebBookmarkWidthChanged(value),
        Event::WebBookmarkHeightChanged(value) => ViewMessage::WebBookmarkHeightChanged(value),
        Event::WebBookmarkAddSave => ViewMessage::WebBookmarkAddSave,
    })
}
fn input(app: &App, editing: bool) -> domain::Input<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let title = if editing { "书签信息" } else { "新增网址书签" };
    let description = if editing { "设置标题、网址以及独立窗口的默认尺寸。" } else { "保存一个可在应用中心快速打开的网页入口。" };
    let cookie_label = i18n.__("Cookie配置(JSON)");
    let help = format!("{}\n\n{}\n{}", i18n.__("支持按 JSON 为指定站点注入 Cookie。"), i18n.__("配置示例"), domain::WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE);
    domain::Input {
        i18n, message, editing_web_bookmark: app.editing_web_bookmark,
        heading: settings_group_heading_with_help(i18n, &app.app_theme, i18n.__(title), i18n.__(description)).into(),
        cookie_help: settings_hover_help_label(i18n, &app.app_theme, cookie_label, settings_field_label(cookie_label), help).wrap().into(),
        edit_web_bookmark_cookie_configs_editor: &app.edit_web_bookmark_cookie_configs_editor,
        web_bookmark_title_input: &app.web_bookmark_title_input,
        web_bookmark_url_input: &app.web_bookmark_url_input,
        web_bookmark_width_input: &app.web_bookmark_width_input,
        web_bookmark_height_input: &app.web_bookmark_height_input,
        edit_web_bookmark_title_input: &app.edit_web_bookmark_title_input,
        edit_web_bookmark_url_input: &app.edit_web_bookmark_url_input,
        edit_web_bookmark_width_input: &app.edit_web_bookmark_width_input,
        edit_web_bookmark_height_input: &app.edit_web_bookmark_height_input,
    }
}
fn add_web_bookmark_modal(app: &App) -> Option<(Element<'_, Message>, Message)> {
    domain::add_web_bookmark_modal(input(app, false))
}
fn edit_web_bookmark_modal(app: &App) -> Option<(Element<'_, Message>, Message)> {
    domain::edit_web_bookmark_modal(input(app, true))
}
pub(super) fn active_modal(app: &App) -> Option<(Element<'_, Message>, Message)> {
    if app.editing_web_bookmark.is_some() { edit_web_bookmark_modal(app) }
    else if app.show_web_links_menu { add_web_bookmark_modal(app) }
    else { None }
}
#[cfg(test)]
#[path = "modals_tests.rs"]
mod modals_tests;

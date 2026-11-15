//! 桌面只注入专家状态和主题，并映射局部消息。
use crate::app::{App, Message};
use desktop_expert_hub::views::{self, ViewInput};
use iced::Element;
fn input(app: &App) -> ViewInput<'_> { ViewInput { expert_hub: &app.expert_hub, desktop_locale: app.desktop_locale, app_theme: &app.app_theme } }
fn message(value: desktop_expert_hub::message::connectors::ConnectorMessage) -> Message {
    Message::SkillHub(crate::app::message::SkillHubMessage::Connectors(value))
}
fn help<'a>(title: &'a str, description: std::borrow::Cow<'a, str>, color: Option<iced::Color>, i18n: crate::app::i18n::I18n) -> Element<'a, Message> {
    desktop_chat_view::message_view::markdown_selection::selectable_titled_text(title, description, color, i18n)
}
pub(super) fn view(app: &App) -> Element<'_, Message> { views::connectors::view(&input(app), help, message) }
pub(super) fn with_binding_modal<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    match views::connectors::binding_modal(&input(app), help, message) {
        Some((close, card)) => desktop_ui_controls::system_settings_common::settings_modal_overlay(Some(base), close, card),
        None => base,
    }
}
#[cfg(test)]
#[path = "connectors_ui_refresh_39_tests.rs"]
mod ui_refresh_39_tests;

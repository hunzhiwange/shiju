//! 桌面只注入专家状态和主题，并映射局部消息。
use crate::app::{App, Message};
use desktop_expert_hub::views::{self, ViewInput};
use iced::Element;
fn input(app: &App) -> ViewInput<'_> { ViewInput { expert_hub: &app.expert_hub, desktop_locale: app.desktop_locale, app_theme: &app.app_theme } }
fn map(message: views::Message) -> Message {
    match message {
        views::Message::SkillHub(value) => Message::SkillHub(crate::app::message::skill_hub::map_domain(value)),
        views::Message::Connectors(value) => Message::SkillHub(crate::app::message::SkillHubMessage::Connectors(value)),
        views::Message::None => Message::None,
    }
}
pub(super) fn view(app: &App) -> Element<'_, Message> { views::connectors::view(&input(app)).map(map) }
pub(super) fn with_binding_modal<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    match views::connectors::binding_modal(&input(app)) {
        Some((close, card)) => desktop_ui_controls::system_settings_common::settings_modal_overlay(Some(base), map(close), card.map(map)),
        None => base,
    }
}
#[cfg(test)]
#[path = "connectors_ui_refresh_39_tests.rs"]
mod ui_refresh_39_tests;

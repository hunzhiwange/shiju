//! 桌面仅注入应用状态和导航并映射局部消息。
use crate::app::{App, Message};
use iced::Element;
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    desktop_expert_hub::views::shiju_apps::view(&input(app), super::section_tabs(app), map)
}
fn input(app: &App) -> desktop_expert_hub::views::shiju_apps::ViewInput<'_> {
 desktop_expert_hub::views::shiju_apps::ViewInput { shiju_apps: &app.shiju_apps, desktop_locale: app.desktop_locale }
}
fn map(message: desktop_expert_hub::views::shiju_apps::Message) -> Message {
 match message { desktop_expert_hub::views::shiju_apps::Message::ShijuApps(m) => Message::ShijuApps(m), desktop_expert_hub::views::shiju_apps::Message::StartBrowserLogin => Message::Account(crate::app::message::AccountMessage::StartBrowserLogin) }
}

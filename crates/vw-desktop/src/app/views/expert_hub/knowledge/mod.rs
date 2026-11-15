//! 桌面仅映射知识视图消息。
use crate::app::{App, Message};
use iced::Element;
use desktop_expert_hub::views::{ViewInput, knowledge};
fn map(message: desktop_expert_hub::views::Message) -> Message {
 match message {
  desktop_expert_hub::views::Message::SkillHub(value) => Message::SkillHub(crate::app::message::skill_hub::map_domain(value)),
  desktop_expert_hub::views::Message::Connectors(value) => Message::SkillHub(crate::app::message::SkillHubMessage::Connectors(value)),
  desktop_expert_hub::views::Message::StartBrowserLogin => Message::Account(crate::app::message::AccountMessage::StartBrowserLogin),
        desktop_expert_hub::views::Message::None => Message::None,
 }
}
pub(super) fn view(app: &App) -> Element<'_, Message> { knowledge::view(&ViewInput { expert_hub: &app.expert_hub, desktop_locale: app.desktop_locale, app_theme: &app.app_theme }).map(map) }

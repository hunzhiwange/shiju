use crate::app::{App, Message};
use iced::Element;
use desktop_expert_hub::views::{upgrade_actions as domain, ViewInput};
pub(super) fn view(app: &App) -> Element<'_, Message> {
 domain::view(&ViewInput { expert_hub: &app.expert_hub, desktop_locale: app.desktop_locale, app_theme: &app.app_theme }).map(map)
}
fn map(message: desktop_expert_hub::views::Message) -> crate::app::Message {
 match message {
 desktop_expert_hub::views::Message::SkillHub(m) => crate::app::Message::SkillHub(m),
 desktop_expert_hub::views::Message::Connectors(m) => crate::app::Message::Connectors(m),
 desktop_expert_hub::views::Message::StartBrowserLogin => crate::app::Message::Account(crate::app::message::AccountMessage::StartBrowserLogin),
 desktop_expert_hub::views::Message::None => crate::app::Message::None,
 }
}

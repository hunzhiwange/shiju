//! 桌面仅映射知识视图消息。
use crate::views::workspace::{Message, ViewInput as WorkspaceInput};
use crate::views::{ViewInput, knowledge};
use iced::Element;
fn map(message: crate::views::Message) -> Message {
    match message {
        crate::views::Message::SkillHub(value) => Message::SkillHub(value),
        crate::views::Message::Connectors(value) => {
            Message::SkillHub(crate::views::workspace::SkillHubMessage::Connectors(value))
        }
        crate::views::Message::StartBrowserLogin => {
            Message::Account(crate::views::workspace::AccountMessage::StartBrowserLogin)
        }
        crate::views::Message::None => Message::None,
    }
}
pub(super) fn view<'a>(app: &WorkspaceInput<'a>) -> Element<'a, Message> {
    knowledge::view(&ViewInput {
        expert_hub: &app.expert_hub,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
    })
    .map(map)
}

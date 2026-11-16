//! 桌面只注入专家状态和主题，并映射局部消息。
use crate::views::workspace::{Message, ViewInput as WorkspaceInput};
use crate::views::{self, ViewInput};
use iced::Element;
fn input<'a>(app: &WorkspaceInput<'a>) -> ViewInput<'a> {
    ViewInput {
        expert_hub: &app.expert_hub,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
    }
}
fn map(message: views::Message) -> Message {
    match message {
        views::Message::SkillHub(value) => Message::SkillHub(value),
        views::Message::Connectors(value) => {
            Message::SkillHub(crate::views::workspace::SkillHubMessage::Connectors(value))
        }
        views::Message::StartBrowserLogin => {
            Message::Account(crate::views::workspace::AccountMessage::StartBrowserLogin)
        }
        views::Message::None => Message::None,
    }
}
pub(super) fn view<'a>(app: &WorkspaceInput<'a>) -> Element<'a, Message> {
    views::detail::view(&input(app)).map(map)
}

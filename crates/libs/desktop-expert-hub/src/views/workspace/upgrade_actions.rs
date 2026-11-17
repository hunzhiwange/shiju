#[cfg(test)]
pub(super) fn view<'a>(app: &WorkspaceInput<'a>) -> Element<'a, Message> {
    domain::view(&ViewInput {
        expert_hub: &app.expert_hub,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
    })
    .map(map)
}
#[cfg(test)]
fn map(message: crate::views::Message) -> crate::views::workspace::Message {
    match message {
        crate::views::Message::SkillHub(m) => crate::views::workspace::Message::SkillHub(m),
        crate::views::Message::Connectors(m) => crate::views::workspace::Message::Connectors(m),
        crate::views::Message::StartBrowserLogin => crate::views::workspace::Message::Account(
            crate::views::workspace::AccountMessage::StartBrowserLogin,
        ),
        crate::views::Message::None => crate::views::workspace::Message::None,
    }
}

#[cfg(test)]
use crate::views::ViewInput;
#[cfg(test)]
use crate::views::upgrade_actions as domain;
#[cfg(test)]
use crate::views::workspace::Message;
#[cfg(test)]
use crate::views::workspace::ViewInput as WorkspaceInput;
#[cfg(test)]
use iced::Element;

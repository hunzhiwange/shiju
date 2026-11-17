use crate::views::workspace::{Message, ViewInput as WorkspaceInput};
use crate::views::{ViewInput, public_skills as domain};
use iced::Element;
pub(super) fn view<'a>(app: &WorkspaceInput<'a>) -> Element<'a, Message> {
    domain::view(&ViewInput {
        expert_hub: &app.expert_hub,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
    })
    .map(map)
}
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
pub(super) fn loading_view() -> Element<'static, Message> {
    domain::loading_view().map(map)
}
pub(super) fn error_view(i18n: desktop_i18n::I18n, error: &str) -> Element<'static, Message> {
    domain::error_view(i18n, error).map(map)
}
pub(super) fn suites_content(
    i18n: desktop_i18n::I18n,
    state: &desktop_skill_hub::skill_hub::PublicSkillMarketState,
) -> Element<'static, Message> {
    domain::suites_content(i18n, state).map(map)
}
pub(super) fn source_selector(
    i18n: desktop_i18n::I18n,
    state: &desktop_skill_hub::skill_hub::PublicSkillMarketState,
) -> Element<'static, Message> {
    domain::source_selector(i18n, state).map(map)
}

#[cfg(test)]
pub(super) use domain::grid_column_count;
#[cfg(test)]
pub(super) use domain::public_tab_specs;
#[cfg(test)]
pub(super) use domain::source_selector_visible;

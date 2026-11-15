pub(crate) type State =
    desktop_settings_skills::prompts::sync::State<crate::app::state::AgentAppsContext>;
pub(crate) fn overlay<'a>(
    state: &'a State,
    base: iced::Element<'a, crate::app::Message>,
) -> iced::Element<'a, crate::app::Message> {
    desktop_settings_skills::prompts::sync::overlay(state, base, super::ui_message)
}

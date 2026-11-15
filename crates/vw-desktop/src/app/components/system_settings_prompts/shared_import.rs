pub(crate) use desktop_settings_skills::prompts::shared_import::Draft;
pub(crate) fn overlay<'a>(
    state: &'a super::State,
    base: iced::Element<'a, crate::app::Message>,
) -> iced::Element<'a, crate::app::Message> {
    desktop_settings_skills::prompts::shared_import::overlay(state, base, super::ui_message)
}

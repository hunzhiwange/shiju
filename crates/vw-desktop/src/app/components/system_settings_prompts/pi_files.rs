pub(crate) use desktop_settings_skills::prompts::pi_files::FileDraft;
pub(crate) use desktop_settings_skills::prompts::pi_files::State;
pub(crate) use desktop_settings_skills::prompts::pi_files::kind;
pub(crate) fn overlay<'a>(
    state: &'a State,
    base: iced::Element<'a, crate::app::Message>,
) -> iced::Element<'a, crate::app::Message> {
    desktop_settings_skills::prompts::pi_files::overlay(state, base, super::ui_message)
}

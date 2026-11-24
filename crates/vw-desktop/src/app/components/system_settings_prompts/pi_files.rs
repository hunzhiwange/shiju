pub(crate) use desktop_settings_skills::prompts::pi_files::State;
pub(crate) fn overlay<'a>(
    i18n: desktop_i18n::I18n,
    state: &'a State,
    base: iced::Element<'a, crate::app::Message>,
) -> iced::Element<'a, crate::app::Message> {
    desktop_settings_skills::prompts::pi_files::overlay(i18n, state, base, super::ui_message)
}

#[cfg(test)]
pub(crate) use desktop_settings_skills::prompts::pi_files::FileDraft;
#[cfg(test)]
pub(crate) use desktop_settings_skills::prompts::pi_files::kind;

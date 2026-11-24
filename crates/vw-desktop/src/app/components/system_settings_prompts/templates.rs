pub(crate) use desktop_settings_skills::prompts::templates::State;
pub(crate) fn overlay<'a>(
    i18n: desktop_i18n::I18n,
    state: &'a State,
    base: iced::Element<'a, crate::app::Message>,
) -> iced::Element<'a, crate::app::Message> {
    desktop_settings_skills::prompts::templates::overlay(i18n, state, base, super::ui_message)
}

#[cfg(test)]
pub(crate) use desktop_settings_skills::prompts::templates::Draft;

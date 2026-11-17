pub(crate) use desktop_settings_skills::prompts::templates::State;
pub(crate) fn overlay<'a>(
    state: &'a State,
    base: iced::Element<'a, crate::app::Message>,
) -> iced::Element<'a, crate::app::Message> {
    desktop_settings_skills::prompts::templates::overlay(state, base, super::ui_message)
}

#[cfg(test)]
pub(crate) use desktop_settings_skills::prompts::templates::Draft;

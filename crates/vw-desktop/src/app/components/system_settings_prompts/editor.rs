#[cfg(test)]
pub(crate) fn view<'a>(
    draft: &'a Draft,
    base: iced::Element<'a, crate::app::Message>,
) -> iced::Element<'a, crate::app::Message> {
    desktop_settings_skills::prompts::editor::view(draft, base, super::ui_message)
}

#[cfg(test)]
pub(crate) use desktop_settings_skills::prompts::editor::Draft;

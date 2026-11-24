#[cfg(test)]
pub(crate) fn view<'a>(
    draft: &'a Draft,
    base: iced::Element<'a, crate::app::Message>,
) -> iced::Element<'a, crate::app::Message> {
    desktop_settings_skills::prompts::editor::view(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        draft,
        base,
        super::ui_message,
    )
}

#[cfg(test)]
pub(crate) use desktop_settings_skills::prompts::editor::Draft;

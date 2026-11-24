#[cfg(test)]
pub(crate) fn overlay<'a>(
    state: &'a super::State,
    base: iced::Element<'a, crate::app::Message>,
) -> iced::Element<'a, crate::app::Message> {
    desktop_settings_skills::prompts::shared_import::overlay(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        state,
        base,
        super::ui_message,
    )
}

#[cfg(test)]
pub(crate) use desktop_settings_skills::prompts::shared_import::Draft;

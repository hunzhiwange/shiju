#[cfg(test)]
pub(crate) type State =
    desktop_settings_skills::prompts::sync::State<crate::app::state::AgentAppsContext>;
#[cfg(test)]
pub(crate) fn overlay<'a>(
    state: &'a State,
    base: iced::Element<'a, crate::app::Message>,
) -> iced::Element<'a, crate::app::Message> {
    desktop_settings_skills::prompts::sync::overlay(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        state,
        base,
        super::ui_message,
    )
}

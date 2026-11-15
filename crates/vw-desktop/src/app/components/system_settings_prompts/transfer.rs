pub(crate) use desktop_settings_skills::prompts::transfer::State;
pub(crate) use desktop_settings_skills::prompts::transfer::TARGETS;
pub(crate) fn overlay<'a>(
    state: &'a State,
    current: vw_api_app_identity::AppId,
    busy: bool,
    base: iced::Element<'a, crate::app::Message>,
) -> iced::Element<'a, crate::app::Message> {
    desktop_settings_skills::prompts::transfer::overlay(
        state,
        current,
        busy,
        base,
        super::ui_message,
    )
}

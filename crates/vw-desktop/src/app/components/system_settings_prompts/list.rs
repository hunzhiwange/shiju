pub(crate) type State =
    desktop_settings_skills::prompts::list::State<crate::app::state::AgentAppsContext>;
pub(crate) fn view(
    state: &State,
    id: vw_api_app_identity::AppId,
) -> iced::Element<'_, crate::app::Message> {
    desktop_settings_skills::prompts::list::view(state, id, super::ui_message)
}

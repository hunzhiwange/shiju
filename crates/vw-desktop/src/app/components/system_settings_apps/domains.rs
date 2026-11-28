pub(super) fn view(i18n: desktop_i18n::I18n, state: &crate::app::state::AppsCatalogState) -> iced::Element<'_, crate::app::Message> {
    use desktop_settings_apps::system_settings_apps::domains::Target;
    use crate::app::{components::{system_settings::SystemTab, system_settings_search::SystemSettingsSearchTarget}, state::AgentConfigTab};
    desktop_settings_apps::system_settings_apps::domains::view(i18n, iced::widget::container(crate::app::components::external_sessions::view(
        &state.external_sessions,
        i18n,
    ))
        .height(if state.external_sessions.open { iced::Length::Fixed(640.0) } else { iced::Length::Shrink }).into(), |target| {
        let target = match target {
            Target::Skills => {
                return crate::app::Message::SkillHub(crate::app::message::SkillHubMessage::OpenLocalSkills);
            }
            Target::Mcp => SystemSettingsSearchTarget::Agents(AgentConfigTab::Mcp),
            Target::Plugins => SystemSettingsSearchTarget::Agents(AgentConfigTab::Plugins),
            Target::General => SystemSettingsSearchTarget::TopLevel(SystemTab::General),
            #[cfg(not(target_arch = "wasm32"))]
            Target::Auth => SystemSettingsSearchTarget::TopLevel(SystemTab::Auth),
        };
        crate::app::Message::Settings(crate::app::message::SettingsMessage::SystemSearchTargetSelected(target))
    })
}

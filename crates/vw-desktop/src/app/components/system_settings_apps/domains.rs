pub(super) fn view(state: &crate::app::state::AppsCatalogState) -> iced::Element<'_, crate::app::Message> {
    use desktop_settings_apps::system_settings_apps::domains::Target;
    use crate::app::{components::{system_settings::SystemTab, system_settings_search::SystemSettingsSearchTarget}, state::AgentConfigTab};
    desktop_settings_apps::system_settings_apps::domains::view(crate::app::components::external_sessions::view(&state.external_sessions), |target| {
        let target = match target { Target::Mcp => SystemSettingsSearchTarget::Agents(AgentConfigTab::Mcp), Target::Skills => SystemSettingsSearchTarget::Agents(AgentConfigTab::Skills), Target::Plugins => SystemSettingsSearchTarget::Agents(AgentConfigTab::Plugins), Target::General => SystemSettingsSearchTarget::TopLevel(SystemTab::General), #[cfg(not(target_arch = "wasm32"))] Target::Auth => SystemSettingsSearchTarget::TopLevel(SystemTab::Auth) };
        crate::app::Message::Settings(crate::app::message::SettingsMessage::SystemSearchTargetSelected(target))
    })
}

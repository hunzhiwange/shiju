use crate::app::App;
#[cfg(test)] use crate::app::Message;
use crate::app::state::DelegateAgentSettingsEntry;
use config_agent_profile::AgentDefaults;
#[cfg(test)] use iced::Element;
#[cfg(test)]
pub(super) fn toggle_message(defaults: &AgentDefaults, id: &str, checked: bool, available: bool) -> Message {
    super::domain::event(desktop_settings_agents::mcp::toggle_message(defaults, id, checked, available))
}
pub(super) fn input<'a>(app: &'a App, entry: &'a DelegateAgentSettingsEntry) -> desktop_settings_agents::mcp::Input<'a> {

desktop_settings_agents::mcp::Input { help: super::domain::help,
        locale: app.desktop_locale, theme: &app.app_theme,
        inspect: app.mcp_settings.inspect.as_ref(), inspect_loading: app.mcp_settings.inspect_loading,
        inspect_error: app.mcp_settings.inspect_error.as_ref(), defaults: super::domain::defaults_input(app, entry),
    }
}
#[cfg(test)]
pub(super) fn view<'a>(app: &'a App, entry: &'a DelegateAgentSettingsEntry) -> Element<'a, Message> { desktop_settings_agents::mcp::view(input(app, entry), entry).map(super::domain::event) }

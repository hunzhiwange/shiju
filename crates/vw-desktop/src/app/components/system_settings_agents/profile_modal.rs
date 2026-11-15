//! Desktop composition for the agent profile modal.
use crate::app::{App, Message};
use iced::Element;
use desktop_settings_agents as domain;
pub(in crate::app::components) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let state = &app.agents_settings;
    let fields = if let Some(entry) = state.profile_modal.as_ref() {
        let native = matches!(entry.runtime, config_agent_profile::AgentRuntime::Native);
        match state.active_detail_tab.as_str() {
            domain::state::AGENT_DETAIL_IDENTITY_TAB => domain::identity::view(super::identity::input(app, entry), entry),
            domain::state::AGENT_DETAIL_TOOLS_TAB if native => domain::tools::view(domain::tools::Input { locale: app.desktop_locale, theme: &app.app_theme, help: super::domain::help, available_tools: &state.available_tools, active_tools_list_tab: &state.active_tools_list_tab }, entry),
            domain::state::AGENT_DETAIL_SKILLS_TAB => domain::skills::view(super::skills::input(app, super::agent_project_options(app)), entry),
            domain::state::AGENT_DETAIL_MCP_TAB => domain::mcp::view(super::mcp::input(app, entry), entry),
            _ => domain::basic::view(super::basic::input(app, entry), entry, Vec::new()),
        }
    } else { iced::widget::Space::new().into() };
    let card = domain::profile_modal::card(domain::profile_modal::Input { help: super::domain::help, locale: app.desktop_locale, theme: &app.app_theme, config_ready: state.config_ready, profile_entering_key: state.profile_entering_key, new_agent_key_input: &state.new_agent_key_input, draft: state.profile_modal.as_ref(), profile_saving: state.profile_saving, active_detail_tab: &state.active_detail_tab }, fields);
    crate::app::components::system_settings_common::settings_optional_modal_overlay(base, card.map(|card| (super::domain::event(domain::message::Event::Agents(domain::message::AgentsMessage::ProfileCancel)), card.map(super::domain::event))))
}

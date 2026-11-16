//! Desktop mapping for the agents domain's local events and borrowed inputs.
use crate::app::{App, Message};
use crate::app::message::settings::{AcpMessage, SettingsMessage, AgentConfigTabMessage, McpMessage};
use crate::app::state::AgentConfigTab;
use desktop_settings_agents::{self as domain, message::{Event, AcpMessage as LocalAcp}};

pub(in crate::app::components) fn event(event: Event) -> Message {
    match event {
        Event::CopyText(text) => Message::CopyCode(text),
        Event::OpenSkills => Message::Settings(SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(AgentConfigTab::Skills))),
        Event::None => Message::None,
        Event::RefreshMcp => Message::Settings(SettingsMessage::Mcp(McpMessage::InspectRefresh)),
        Event::OpenMcp => Message::Settings(SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(AgentConfigTab::Mcp))),
        Event::Agents(value) => Message::Settings(SettingsMessage::Agents(value)),
        Event::Acp(value) => Message::Settings(SettingsMessage::Acp(match value {
        LocalAcp::StartDiagnostic => AcpMessage::StartDiagnostic,
        LocalAcp::TestAgent(v0) => AcpMessage::TestAgent(v0),
        LocalAcp::InstallAgent(v0) => AcpMessage::InstallAgent(v0),
        LocalAcp::CancelDiagnostic => AcpMessage::CancelDiagnostic,
        LocalAcp::EditAgent(v0) => AcpMessage::EditAgent(v0),
        LocalAcp::CancelEditor => AcpMessage::CancelEditor,
        LocalAcp::RestoreDefaultArguments => AcpMessage::RestoreDefaultArguments,
        LocalAcp::RestoreDefaultEnvironment => AcpMessage::RestoreDefaultEnvironment,
        LocalAcp::ArgumentChanged(v0, v1) => AcpMessage::ArgumentChanged(v0, v1),
        LocalAcp::AddArgument => AcpMessage::AddArgument,
        LocalAcp::RemoveArgument(v0) => AcpMessage::RemoveArgument(v0),
        LocalAcp::EnvironmentChanged(v0, v1, v2) => AcpMessage::EnvironmentChanged(v0, v1, v2),
        LocalAcp::AddEnvironment => AcpMessage::AddEnvironment,
        LocalAcp::RemoveEnvironment(v0) => AcpMessage::RemoveEnvironment(v0),
        LocalAcp::Refresh => AcpMessage::Refresh,
        LocalAcp::NewAgentPanelToggled => AcpMessage::NewAgentPanelToggled,
        LocalAcp::NewAgentNameChanged(v0) => AcpMessage::NewAgentNameChanged(v0),
        LocalAcp::NewAgentCommandChanged(v0) => AcpMessage::NewAgentCommandChanged(v0),
        LocalAcp::AddAgent => AcpMessage::AddAgent,
        LocalAcp::CopyCommand { agent, command } => AcpMessage::CopyCommand { agent, command },
        LocalAcp::SetEnabled { agent, enabled } => AcpMessage::SetEnabled { agent, enabled },
        })),
    }
}

pub(in crate::app::components) fn icon(agent: &str, size: f32) -> iced::Element<'static, Event> {
    crate::app::components::input_panel::icons::acp_agent_icon(agent, size).map(|_| Event::None)
}

pub(in crate::app::components) fn profile_input(app: &App) -> domain::basic_profile::Input<'_> {
    domain::basic_profile::Input {
        locale: app.desktop_locale, entries: &app.agents_settings.entries,
        acp_agents: &app.acp_agents, saving: app.agents_settings.profile_saving,
        acp_loading: app.acp_settings.loading, acp_error: app.acp_settings.save_error.as_ref(),
    }
}

pub(in crate::app::components) fn defaults_input<'a>(app: &'a App, draft: &domain::state::DelegateAgentSettingsEntry) -> domain::defaults::Input<'a> {
    let model_source_available = domain::defaults::supports_fixed_model(&draft.runtime,
        app.session_runtime_states.values().map(|state|
            (state.acp_agent.as_deref(), state.acp_options_state.options.model_source_capabilities.as_slice())));
    domain::defaults::catalog_input(&app.agents_settings, app.desktop_locale, &app.app_theme, help,
        model_source_available, app.mcp_settings.inspect.as_ref(), app.mcp_settings.inspect_loading,
        app.mcp_settings.inspect_error.as_ref())
}

pub(in crate::app::components) fn catalog_input(app: &App) -> domain::catalog::Input<'_> {
    let state = &app.agents_settings;
    domain::catalog::Input {
        locale: app.desktop_locale, entries: &state.entries, selected_agent: &state.selected_agent,
        catalog_official: state.catalog_official, catalog_query: &state.catalog_query,
        catalog_busy: state.catalog_busy, save_in_flight: state.save_in_flight,
        profile_saving: state.profile_saving, dirty: state.dirty, profile_entering_key: state.profile_entering_key,
        diagnostic_results: &app.acp_settings.diagnostic_results,
        new_session_creating: app.new_session_creating, icon,
    }
}

pub(in crate::app::components) fn acp_input(app: &App) -> domain::acp_view::Input<'_> {
    domain::acp_view::Input { help, state: &app.acp_settings, locale: app.desktop_locale,
        theme: &app.app_theme, frame: app.status_animation_frame, icon }
}

pub(in crate::app::components) fn help<'a>(i18n: desktop_i18n::I18n, theme: Option<&iced::Theme>, title: &'a str, label: iced::widget::Text<'a>, description: std::borrow::Cow<'a, str>) -> iced::widget::Row<'a, Event> {
    let content = crate::app::components::chat_panel::message_view::selectable_titled_text(
        title, description, theme.map(|theme| theme.palette().text), i18n,
    ).map(|message| match message {
        Message::CopyCode(text) => Event::CopyText(text),
        Message::None => Event::None,
        _ => unreachable!("plain selectable help only emits copy or no-op events"),
    });
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(label, content)
}

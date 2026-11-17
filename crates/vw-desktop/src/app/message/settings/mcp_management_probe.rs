//! Captured authority is used only to revoke the original job, including late registration.
use crate::app::{
    App, Message,
    message::agent_apps_diagnostics::{
        self as diagnostics, McpDiagnosticMessage as Event,
    },
};
use iced::Task;

#[cfg(test)]
pub(crate) fn cleanup(
    transport: ProbeTransport,
    context: Context,
    job: DiagnosticJobIdentity,
) -> Task<Message> {
    desktop_settings_mcp::configuration_update::probe::cleanup(transport, context, job)
        .map(|result| diagnostics::message(Event::Cleaned(result)))
}
/// Called after navigation/context/mutations settle; it never starts a connection check.
pub(crate) fn invalidate(app: &mut App) -> Task<Message> {
    use crate::app::{components::system_settings::SystemTab, state::AgentConfigTab};
    let on_page = app.navigation_panels.show_system_settings
        && app.navigation_panels.system_settings_tab == SystemTab::Agents
        && app.agents_settings.active_config_tab == AgentConfigTab::Mcp;
    let changed =
        diagnostics::context(app).as_ref() != app.mcp_settings.diagnostics.context.as_ref();
    let mutating = app.mcp_settings.mutating();
    desktop_settings_mcp::configuration_update::probe::invalidate(
        &mut app.mcp_settings.diagnostics,
        on_page,
        changed,
        mutating,
    )
    .map(|result| diagnostics::message(Event::Cleaned(result)))
}

#[cfg(test)]
use crate::app::message::agent_apps_diagnostics::Context;
#[cfg(test)]
use vw_api_app_diagnostics::*;
#[cfg(test)]
pub use desktop_settings_mcp::configuration_update::probe::ProbeTransport;

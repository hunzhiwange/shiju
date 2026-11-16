//! Desktop context assembly and MCP message mapping.
use crate::app::{App, Message, message::settings::{McpMessage, SettingsMessage, mcp_management_list as catalog}};
use iced::Task;
pub(crate) use desktop_apps_ui::agent_apps_diagnostics::mcp_state::*;
pub use desktop_apps_ui::agent_apps_diagnostics::mcp::McpDiagnosticMessage;
pub(crate) mod mcp_results;
pub(crate) mod mcp_state;
pub(crate) mod endpoint_state;
pub(crate) mod endpoints;
pub(crate) mod model;
pub(crate) mod model_state;
pub(crate) fn message(event: McpDiagnosticMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::Diagnostic(event)))
}
pub(crate) fn context(app: &App) -> Option<Context> {
    desktop_settings_mcp::diagnostic_context::current(catalog::context(app), app.project_id.as_deref(), app.mcp_settings.diagnostics.app.as_ref(),
        &app.apps_catalog.token, app.apps_catalog.context.as_ref(), app.apps_catalog.rows.iter().map(|r| r.entry.id), catalog::visible(app))
}

pub(crate) fn update(app: &mut App, event: McpDiagnosticMessage) -> Task<Message> {
    let current = context(app);
    let catalog_visible = catalog::visible(app).is_some();
    let client = matches!(event, McpDiagnosticMessage::Start(_)).then(crate::app::config::gateway_client);
    let mut notice = None;
    let catalog = catalog_visible.then_some(app.mcp_settings.catalog.result.as_ref()).flatten();
    let task = desktop_apps_ui::agent_apps_diagnostics::mcp::update(
        &mut app.mcp_settings.diagnostics, current, catalog, &app.apps_catalog.token,
        client, &mut notice, event,
    ).map(message);
    let toast = match notice {
        Some(desktop_apps_ui::Notice::Error(text)) => app.show_error_toast(text),
        Some(desktop_apps_ui::Notice::Success(text)) => app.show_success_toast(text),
        None => Task::none(),
    };
    Task::batch([task, toast])
}

use crate::app::{
    App, Message,
    message::{agent_apps_diagnostics as diagnostics, settings::mcp_management_list as catalog},
};
use desktop_apps_ui::agent_apps_diagnostics::mcp_panel as domain;
fn message(event: domain::Event<diagnostics::AppChoice>) -> Message {
    diagnostics::message(match event {
        domain::Event::Select(v) => diagnostics::McpDiagnosticMessage::Select(v),
        domain::Event::Start(v) => diagnostics::McpDiagnosticMessage::Start(v),
        domain::Event::Cancel => diagnostics::McpDiagnosticMessage::Cancel,
        domain::Event::Refresh => diagnostics::McpDiagnosticMessage::Refresh,
    })
}
pub(crate) fn view(app: &App) -> iced::Element<'_, Message> {
    let state = &app.mcp_settings.diagnostics;
    let current = diagnostics::context(app);
    let visible = current.is_some() && current.as_ref() == state.context.as_ref();
    domain::view(
        domain::Panel {
            visible,
            active: state.active(),
            cancelling: state.cancelling,
            choices: app
                .apps_catalog
                .rows
                .iter()
                .map(|r| diagnostics::AppChoice(r.entry.id, r.entry.name.clone()))
                .collect(),
            app: state.app.clone(),
            catalog: catalog::visible(app).and_then(|r| r.as_ref().ok()),
            labels: state.targets.iter().map(|(r, p)| (r.id.clone(), p.label())).collect(),
            error: &state.error,
        },
        message,
        diagnostics::mcp_results::view(state),
    )
}

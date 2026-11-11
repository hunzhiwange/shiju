//! Captured authority is used only to revoke the original job, including late registration.
use crate::app::{
    App, Message,
    message::agent_apps_diagnostics::{
        self as diagnostics, Context, McpDiagnosticMessage as Event,
    },
};
use iced::Task;
use vw_gateway_client::{GatewayClient, vw_api_types::agent_apps_diagnostics::*};

#[derive(Clone)]
pub struct ProbeTransport {
    pub(crate) client: GatewayClient,
    pub(crate) token: String,
}
impl std::fmt::Debug for ProbeTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProbeTransport").finish_non_exhaustive()
    }
}
pub(crate) fn cleanup(
    transport: ProbeTransport,
    context: Context,
    job: DiagnosticJobIdentity,
) -> Task<Message> {
    Task::perform(
        async move {
            let result = async {
                let mut snapshot = transport
                    .client
                    .mcp_management_check_cancel(&context.query, &job.task_id, &transport.token)
                    .await
                    .map_err(|_| ())?;
                loop {
                    if snapshot.job != job {
                        return Err(());
                    }
                    if matches!(
                        snapshot.state,
                        DiagnosticJobState::Succeeded {}
                            | DiagnosticJobState::Failed { .. }
                            | DiagnosticJobState::Cancelled {}
                            | DiagnosticJobState::Interrupted {}
                    ) {
                        return Ok(());
                    }
                    crate::app::message::sleep(std::time::Duration::from_millis(350)).await;
                    snapshot = transport
                        .client
                        .mcp_management_check_status(&context.query, &job.task_id, &transport.token)
                        .await
                        .map_err(|_| ())?;
                }
            }
            .await;
            Event::Cleaned(
                result.map_err(|()| "旧 MCP 检查取消未确认，请在原主机核对作业".to_owned()),
            )
        },
        diagnostics::message,
    )
}
/// Called after navigation/context/mutations settle; it never starts a connection check.
pub(crate) fn invalidate(app: &mut App) -> Task<Message> {
    use crate::app::{components::system_settings::SystemTab, state::AgentConfigTab};
    if app.mcp_settings.diagnostics.context.is_none() {
        return Task::none();
    }
    let on_page = app.show_system_settings
        && app.system_settings_tab == SystemTab::Agents
        && app.agents_settings.active_config_tab == AgentConfigTab::Mcp;
    let changed =
        diagnostics::context(app).as_ref() != app.mcp_settings.diagnostics.context.as_ref();
    let mutating = app.mcp_settings.bindings.running
        || app.mcp_settings.saving
        || app.mcp_settings.connection_form.as_ref().is_some_and(|d| d.saving);
    if on_page && !changed && !mutating {
        return Task::none();
    }
    let state = &mut app.mcp_settings.diagnostics;
    let active = state.active();
    let context = state.context.take();
    let job = state.job.take();
    let transport = state.transport.take();
    state.terminal = true;
    state.targets.clear();
    state.error = None;
    if active && let (Some(context), Some(job), Some(transport)) = (context, job, transport) {
        return cleanup(transport, context, job);
    }
    Task::none()
}

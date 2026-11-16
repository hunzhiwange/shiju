pub use desktop_apps_ui::agent_apps_diagnostics::mcp_cleanup::{cleanup, sleep};
pub use desktop_apps_ui::agent_apps_diagnostics::mcp_transport::ProbeTransport;

pub fn invalidate(
    state: &mut desktop_apps_ui::agent_apps_diagnostics::mcp_state::McpState,
    on_page: bool,
    changed: bool,
    mutating: bool,
) -> iced::Task<Result<(), String>> {
    if state.context.is_none() || (on_page && !changed && !mutating) {
        return iced::Task::none();
    }
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
    iced::Task::none()
}

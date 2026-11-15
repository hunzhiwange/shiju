//! Validated tool-state projection and local message mapping.
use crate::app::{App, Message};
use crate::app::message::settings::{mcp_management_tools as tools, mcp_management_list as catalog};
use desktop_settings_mcp::tools as ui;
use iced::{Element, widget::column};
fn message(event: ui::ToolsMessage) -> Message {
    use tools::ToolsMessage as M;
    tools::message(match event {
        ui::ToolsMessage::Open(v) => M::Open(v), ui::ToolsMessage::Close => M::Close,
        ui::ToolsMessage::AcpSessionChanged(v) => M::AcpSessionChanged(v),
        ui::ToolsMessage::ReadAcp => M::ReadAcp, ui::ToolsMessage::Cancel => M::Cancel,
        ui::ToolsMessage::Refresh => M::Refresh,
    })
}
pub(super) fn panel(app: &App) -> Element<'_, Message> { ui::panel(catalog::visible(app)).map(message) }
pub(crate) fn card(app: &App) -> Element<'_, Message> {
    let Some(state) = tools::visible(app) else { return column![].into(); };
    let context = state.context.as_ref().expect("visible tool context");
    ui::card(ui::ToolsView {
        record: &context.record, acp_session: &state.acp_session, acp_loading: state.acp_loading,
        acp: &state.acp, result: &state.result, error: &state.error,
        cancelling: state.cancelling, terminal: state.terminal,
    }, catalog::visible(app)).map(message)
}

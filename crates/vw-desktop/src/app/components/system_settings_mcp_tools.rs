//! Validated tool-state projection and local message mapping.
use crate::app::message::settings::{
    mcp_management_list as catalog, mcp_management_tools as tools,
};
use crate::app::{App, Message};
use desktop_settings_mcp::tools as ui;
use iced::{Element, widget::column};
fn message(event: ui::ToolsMessage) -> Message {
    use tools::ToolsMessage as M;
    tools::message(match event {
        ui::ToolsMessage::Open(v) => M::Open(v),
        ui::ToolsMessage::Close => M::Close,
        ui::ToolsMessage::AcpSessionChanged(v) => M::AcpSessionChanged(v),
        ui::ToolsMessage::ReadAcp => M::ReadAcp,
        ui::ToolsMessage::Cancel => M::Cancel,
        ui::ToolsMessage::Refresh => M::Refresh,
    })
}
pub(super) fn panel(app: &App) -> Element<'_, Message> {
    ui::panel(catalog::visible(app)).map(message)
}
pub(crate) fn card(app: &App) -> Element<'_, Message> {
    let Some(state) = tools::visible(app) else {
        return column![].into();
    };
    desktop_settings_mcp::management_tools::card(state, catalog::visible(app)).map(tools::message)
}

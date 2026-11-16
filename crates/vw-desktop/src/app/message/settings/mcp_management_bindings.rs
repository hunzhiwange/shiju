//! Native bindings use trusted installation identities; runtime and ACP stay independent.
use super::{McpMessage, SettingsMessage, mcp_management_list as catalog};
use crate::app::{App, Message};
#[cfg(test)]
use gateway_mcp_distribution::{McpBindingRequest, McpBulkRequest};
use iced::Task;
use vw_api_mcp_management::*;
#[cfg(test)]
use vw_gateway_client::GatewayClient;

#[cfg(test)]
use desktop_settings_mcp::distribution::{APPS, item_label};
pub(crate) use desktop_settings_mcp::distribution::{
    project, summary, supported, unsupported_reason,
};
pub use desktop_settings_mcp::management_bindings::{BindingsMessage, BindingsState, apply};
pub(crate) fn message(value: BindingsMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::Bindings(value)))
}
pub(crate) fn visible(app: &App) -> Option<&BindingsState> {
    let state = &app.mcp_settings.bindings;
    (catalog::context(app).as_ref() == state.context.as_ref()).then_some(state)
}
pub(crate) fn update(app: &mut App, event: BindingsMessage) -> Task<Message> {
    let mut input = super::mcp_management_list::input(app);
    let task = desktop_settings_mcp::configuration_update::bindings::update(&mut input, event).map(message);
    let effects = std::mem::take(&mut input.effects);
    super::mcp_management_tools::apply_effects(app, task, effects)
}
#[cfg(test)]
#[path = "mcp_management_bindings_tests.rs"]
mod tests;

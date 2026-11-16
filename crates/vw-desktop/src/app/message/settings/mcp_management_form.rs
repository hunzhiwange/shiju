//! Connection drafts use the public DTO; only the gateway validates and persists definitions.
use super::{McpMessage, SettingsMessage, mcp_management_list as catalog};
use crate::app::{App, Message};
use iced::Task;
#[cfg(test)]
use std::collections::BTreeMap;
use vw_api_mcp_management::*;
#[cfg(test)]
use vw_gateway_client::GatewayClient;

pub use desktop_settings_mcp::management_form::{Draft, FormMessage, save};
pub(crate) fn message(message: FormMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::ConnectionForm(message)))
}
pub(crate) fn update(app: &mut App, event: FormMessage) -> Task<Message> {
    let mut input = super::mcp_management_list::input(app);
    let task = desktop_settings_mcp::configuration_update::form::update(&mut input, event).map(message);
    let effects = std::mem::take(&mut input.effects);
    super::mcp_management_tools::apply_effects(app, task, effects)
}
#[cfg(test)]
#[path = "mcp_management_form_tests.rs"]
mod tests;

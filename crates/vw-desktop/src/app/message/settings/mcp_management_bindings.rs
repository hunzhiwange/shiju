//! Native bindings use trusted installation identities; runtime and ACP stay independent.
use super::{McpMessage, SettingsMessage, mcp_management_list as catalog};
use crate::app::{App, Message};
use iced::Task;

pub use desktop_settings_mcp::management_bindings::{BindingsMessage, BindingsState};
pub(crate) fn message(value: BindingsMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::Bindings(value)))
}
pub(crate) fn visible(app: &App) -> Option<&BindingsState> {
    desktop_settings_mcp::management_bindings::visible(
        &app.mcp_settings.bindings,
        catalog::context(app).as_ref(),
    )
}

pub(crate) fn update(app: &mut App, event: BindingsMessage) -> Task<Message> {
    let mut input = super::mcp_management_list::input(app);
    let task = desktop_settings_mcp::configuration_update::bindings::update(&mut input, event)
        .map(message);
    let effects = std::mem::take(&mut input.effects);
    super::mcp_management_tools::apply_effects(app, task, effects)
}

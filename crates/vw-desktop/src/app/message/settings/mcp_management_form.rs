//! Connection drafts use the public DTO; only the gateway validates and persists definitions.
use super::{McpMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::Task;

pub use desktop_settings_mcp::management_form::{Draft, FormMessage};
pub(crate) fn message(message: FormMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::ConnectionForm(message)))
}
pub(crate) fn update(app: &mut App, event: FormMessage) -> Task<Message> {
    let refresh_global = matches!(&event, FormMessage::GlobalSaved { written: Some(_), .. });
    if matches!(&event, FormMessage::Open(_) | FormMessage::OpenGlobal(..) | FormMessage::Sync(_))
        && !crate::app::components::system_settings_mcp::is_visible(app)
    {
        return Task::none();
    }
    app.mcp_settings.page.global_sync_menu = None;
    let mut input = super::mcp_management_list::input(app);
    let task =
        desktop_settings_mcp::configuration_update::form::update(&mut input, event).map(message);
    let effects = std::mem::take(&mut input.effects);
    let task = super::mcp_management_tools::apply_effects(app, task, effects);
    if refresh_global {
        Task::batch([task, super::mcp::refresh_configuration(app)])
    } else {
        task
    }
}

//! Shell collects cross-page inputs and applies domain effects.
use super::{McpMessage, SettingsMessage, mcp_management_list as catalog};
use crate::app::{App, Message};
pub use desktop_settings_mcp::configuration_update::resync::{ResyncMessage, ResyncState};
use iced::Task;
pub(crate) fn message(value: ResyncMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::Resync(value)))
}
pub(crate) fn visible(app: &App) -> Option<&ResyncState> {
    desktop_settings_mcp::configuration_update::resync::visible(
        &app.mcp_settings.resync,
        catalog::context(app).as_ref(),
    )
}

pub(crate) fn update(app: &mut App, value: ResyncMessage) -> Task<Message> {
    let context = catalog::context(app);
    let authorized =
        context.as_ref().is_some_and(|c| app.apps_catalog.context.as_ref() == Some(&c.endpoint));
    let targets = super::mcp_management_import::targets(app);
    let catalog_current = context.as_ref() == app.mcp_settings.catalog.context.as_ref();
    let catalog = catalog_current.then_some(app.mcp_settings.catalog.result.as_ref()).flatten();
    let modal_open = app.mcp_settings.modal_open();
    let mut input = desktop_settings_mcp::configuration_update::resync::Update {
        state: &mut app.mcp_settings.resync,
        context,
        authorized,
        token: app.apps_catalog.token.clone(),
        client: crate::app::config::gateway_client(),
        modal_open,
        targets,
        selected: &app.mcp_settings.bindings.selected,
        catalog,
        effects: Vec::new(),
    };
    let task =
        desktop_settings_mcp::configuration_update::resync::update(&mut input, value).map(message);
    let effects = std::mem::take(&mut input.effects);
    super::mcp_management_tools::apply_effects(app, task, effects)
}

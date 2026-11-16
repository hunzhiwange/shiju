//! Confirmation only: files and credentials stay on the selected execution host.
use super::{McpMessage, SettingsMessage, mcp_management_list as catalog};
use crate::app::{App, Message};
use iced::Task;
use vw_api_mcp_management::{McpAppTarget, McpCreateRequest};

pub use desktop_settings_mcp::management_import::{ImportMessage, ImportState};
pub(crate) fn message(event: ImportMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::Import(event)))
}
pub(crate) fn targets(app: &App) -> Vec<McpAppTarget> {
    let context = catalog::context(app);
    let authorized = context.as_ref().is_some_and(|c| authorized(app, c));
    desktop_settings_mcp::configuration_update::import::trusted_targets(
        context.as_ref(),
        authorized,
        app.apps_catalog.loading,
        app.apps_catalog.rows.iter().flat_map(|row| {
            row.observation
                .as_ref()
                .ok()
                .into_iter()
                .flat_map(move |o| o.installations.iter().map(move |i| (row.entry.id, i)))
        }),
    )
}

fn authorized(app: &App, context: &catalog::Context) -> bool {
    catalog::context(app).as_ref() == Some(context)
        && !context.host.is_empty()
        && !app.apps_catalog.token.is_empty()
        && app.apps_catalog.context.as_ref() == Some(&context.endpoint)
}
pub(crate) fn visible(app: &App) -> Option<&ImportState> {
    desktop_settings_mcp::management_import::visible(
        app.mcp_settings.import.as_ref(),
        catalog::context(app).as_ref(),
    )
}
pub(crate) fn open_link(app: &mut App, drafts: Vec<Result<McpCreateRequest, String>>) -> bool {
    let mut input = super::mcp_management_list::input(app);
    desktop_settings_mcp::configuration_update::import::open_link(&mut input, drafts)
}
pub(crate) fn update(app: &mut App, event: ImportMessage) -> Task<Message> {
    let mut input = super::mcp_management_list::input(app);
    let task =
        desktop_settings_mcp::configuration_update::import::update(&mut input, event).map(message);
    let effects = std::mem::take(&mut input.effects);
    super::mcp_management_tools::apply_effects(app, task, effects)
}
#[cfg(test)]
#[path = "mcp_management_import_tests.rs"]
mod tests;

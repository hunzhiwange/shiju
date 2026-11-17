//! Managed catalog reads stay on the selected gateway; native observations are not runtime mounts.
use super::{McpMessage, SettingsMessage};
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;
use vw_api_mcp_management::*;

pub use desktop_settings_mcp::configuration_update::catalog::CatalogMessage;
pub use desktop_settings_mcp::management_context::Context;
pub(crate) fn context(app: &App) -> Option<Context> {
    let client = crate::app::config::gateway_client().ok()?;
    Some(Context {
        endpoint: AgentAppsContext {
            gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(),
            project: app.navigation_tabs.project_path.clone(),
        },
        host: local_app_auth::HOST_ID.into(),
        credentials_generation: app.apps_catalog.generation,
    })
}
pub(crate) fn loading(app: &App) -> bool {
    desktop_settings_mcp::configuration_update::catalog::loading(
        &app.mcp_settings.catalog,
        context(app).as_ref(),
    )
}
pub(crate) fn visible(app: &App) -> Option<&Result<McpCatalogResponse, String>> {
    desktop_settings_mcp::configuration_update::catalog::visible(
        &app.mcp_settings.catalog,
        context(app).as_ref(),
    )
}

pub(crate) fn update(app: &mut App, message: CatalogMessage) -> Task<Message> {
    let context = context(app);
    let authorized =
        context.is_some();
    let mut input = desktop_settings_mcp::configuration_update::catalog::Update {
        state: &mut app.mcp_settings.catalog,
        context,
        authorized,
        token: app.apps_catalog.token.clone(),
        client: crate::app::config::gateway_client(),
        effects: Vec::new(),
    };
    let task = desktop_settings_mcp::configuration_update::catalog::update(&mut input, message)
        .map(|message| Message::Settings(SettingsMessage::Mcp(McpMessage::Catalog(message))));
    let effects = std::mem::take(&mut input.effects);
    super::mcp_management_tools::apply_effects(app, task, effects)
}

pub(crate) fn input(app: &mut App) -> desktop_settings_mcp::management_update::Update<'_> {
    let context = context(app);
    let authorized =
        context.is_some();
    let targets = super::mcp_management_import::targets(app);
    desktop_settings_mcp::management_update::Update {
        context,
        authorized,
        targets,
        token: app.apps_catalog.token.clone(),
        client: crate::app::config::gateway_client().map_err(|e| e.to_string()),
        state: &mut app.mcp_settings,
        effects: Vec::new(),
    }
}

#[cfg(test)]
pub use desktop_settings_mcp::configuration_update::catalog::CatalogState;

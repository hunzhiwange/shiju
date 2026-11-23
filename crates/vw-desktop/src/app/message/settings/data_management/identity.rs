use crate::app::{App, state::AgentAppsContext};
use desktop_settings_memory::data_management::identity::Identity;
pub(crate) fn identity(app: &App) -> Option<Identity> {
    let client = crate::app::config::gateway_client().ok()?;
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.navigation_tabs.project_path.clone() };
    Some(Identity { context, project: vw_api_transport::local_settings_project(app.workspace_state.project_id.as_deref()).to_owned(), host: app.apps_catalog.host.clone(), catalog: app.apps_catalog.generation })
}

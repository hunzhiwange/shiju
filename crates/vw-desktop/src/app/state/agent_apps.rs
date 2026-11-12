//! Application catalog state is independent of the custom icon library.
use super::AgentAppsContext;
use crate::app::components::system_settings_apps::catalog::CatalogRow;

#[derive(Clone, Default)]
pub(crate) struct AppsCatalogState {
    pub launch: crate::app::components::system_settings_apps::launch::LaunchState,
    pub preferences: crate::app::components::system_settings_apps::preferences::PreferencesState,
    pub lifecycle: crate::app::components::system_settings_apps::lifecycle::LifecycleState,
    pub model_diagnostics: crate::app::message::agent_apps_diagnostics::model_state::ModelState,
    pub query: String,
    pub hovered: Option<vw_gateway_client::vw_api_types::agent_apps::AppId>,
    pub expanded: Option<vw_gateway_client::vw_api_types::agent_apps::AppId>,
    pub drag: crate::app::components::system_settings_apps::drag::DragState,
    pub host: String,
    pub token: String,
    pub connection_open: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub rows: Vec<CatalogRow>,
    pub context: Option<AgentAppsContext>,
    pub generation: u64,
}
// Credentials must never enter App/Message debug output.
impl std::fmt::Debug for AppsCatalogState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppsCatalogState").field("loading", &self.loading).finish_non_exhaustive()
    }
}
impl AppsCatalogState {
    pub fn reset_if_context_changed(&mut self, context: &AgentAppsContext) -> bool {
        if self.context.as_ref() == Some(context) {
            return false;
        }
        self.generation = self.generation.wrapping_add(1);
        self.loading = false;
        self.rows.clear();
        self.drag = Default::default();
        self.expanded = None;
        self.hovered = None;
        self.model_diagnostics.invalidate_access();
        self.token.clear();
        self.lifecycle.invalidate();
        self.preferences.invalidate();
        self.launch.invalidate();
        self.connection_open = false;
        self.error = Some("连接已更改，正在重新连接应用".into());
        true
    }

    pub fn begin(&mut self, context: AgentAppsContext) -> u64 {
        self.preferences.invalidate();
        self.launch.invalidate();
        self.generation = self.generation.wrapping_add(1);
        self.context = Some(context);
        self.loading = true;
        self.rows.clear();
        self.drag = Default::default();
        self.expanded = None;
        self.hovered = None;
        self.error = None;
        self.generation
    }
    pub fn finish(
        &mut self,
        context: &AgentAppsContext,
        generation: u64,
        result: Result<Vec<CatalogRow>, String>,
    ) -> bool {
        if self.context.as_ref() != Some(context) || self.generation != generation || !self.loading
        {
            return false;
        }
        self.loading = false;
        match result {
            Ok(rows) => self.rows = rows,
            Err(error) => self.error = Some(error),
        }
        true
    }
}

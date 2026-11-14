//! Application catalog state is independent of the custom icon library.
use super::AgentAppsContext;
use crate::app::components::system_settings_apps::catalog::CatalogRow;

#[derive(Clone, Default)]
pub(crate) struct AppsCatalogState {
    pub external_sessions: crate::app::components::external_sessions::State,
    pub routing: crate::app::components::app_routing::lifecycle::State,
    pub openclaw_tools: crate::app::components::openclaw_tools::State,
    pub openclaw_env: crate::app::components::openclaw_env::State,
    pub openclaw_catalog: crate::app::components::openclaw_catalog::State,
    pub openclaw_model: crate::app::components::openclaw_model::State,
    pub openclaw_defaults: crate::app::components::openclaw_defaults::State,
    pub workspace_files: crate::app::components::workspace_files::State,
    pub daily_memory: crate::app::components::daily_memory::State,
    pub hermes_memory: crate::app::components::hermes_memory::State,
    pub hermes_dashboard: crate::app::components::hermes_dashboard::State,
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
    pub loaded_at: Option<web_time::Instant>,
}
// Credentials must never enter App/Message debug output.
impl std::fmt::Debug for AppsCatalogState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppsCatalogState").field("loading", &self.loading).finish_non_exhaustive()
    }
}
impl AppsCatalogState {
    pub fn cache_is_fresh(&self) -> bool {
        self.error.is_none()
            && !self.host.trim().is_empty()
            && !self.token.is_empty()
            && self.loaded_at.is_some_and(|at| at.elapsed() < std::time::Duration::from_secs(30))
    }

    pub fn reset_if_context_changed(&mut self, context: &AgentAppsContext) -> bool {
        if self.context.as_ref() == Some(context) {
            return false;
        }
        self.generation = self.generation.wrapping_add(1);
        self.loading = false;
        self.rows.clear();
        self.loaded_at = None;
        self.drag = Default::default();
        self.expanded = None;
        self.hovered = None;
        self.model_diagnostics.invalidate_access();
        self.token.clear();
        self.lifecycle.invalidate();
        self.routing.invalidate();
        self.preferences.invalidate();
        self.launch.invalidate();
        self.openclaw_catalog.invalidate();
        self.external_sessions.invalidate();
        self.openclaw_model.invalidate();
        self.openclaw_tools.invalidate();
        self.openclaw_env.invalidate();
        self.openclaw_defaults.invalidate();
        self.hermes_dashboard.invalidate();
        self.workspace_files.invalidate();
        self.daily_memory.invalidate();
        self.hermes_memory.invalidate();
        self.connection_open = false;
        self.error = Some("连接已更改，正在重新连接应用".into());
        true
    }

    pub fn begin(&mut self, context: AgentAppsContext) -> u64 {
        self.routing.invalidate();
        self.preferences.invalidate();
        self.launch.invalidate();
        self.openclaw_catalog.invalidate();
        self.openclaw_model.invalidate();
        self.openclaw_tools.invalidate();
        self.openclaw_env.invalidate();
        self.external_sessions.invalidate();
        self.openclaw_defaults.invalidate();
        self.hermes_dashboard.invalidate();
        self.workspace_files.invalidate();
        self.daily_memory.invalidate();
        self.hermes_memory.invalidate();
        self.generation = self.generation.wrapping_add(1);
        self.context = Some(context);
        self.loading = true;
        self.rows.clear();
        self.loaded_at = None;
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
            Ok(rows) => {
                // Failed observations should be retried when the page is reopened.
                self.loaded_at =
                    rows.iter().all(|row| row.observation.is_ok()).then(web_time::Instant::now);
                self.rows = rows;
            }
            Err(error) => self.error = Some(error),
        }
        true
    }
}

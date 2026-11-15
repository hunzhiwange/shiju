//! Application catalog state is independent of the custom icon library.
use super::AgentAppsContext;
use crate::app::components::system_settings_apps::catalog::CatalogRow;

pub(crate) type AppsCatalogState = desktop_apps_ui::catalog_state::AppsCatalogState<
    crate::app::message::settings::device_sync_webdav::State,
    crate::app::message::settings::device_sync_s3::State,
    crate::app::components::system_settings_data::migration_preview_ui::State,
    crate::app::message::settings::data_management::backup_list_ui::State,
    crate::app::message::settings::data_management::recovery_ui::State,
    crate::app::message::settings::data_management::export_ui::State,
    crate::app::message::settings::backup_inventory::State,
    crate::app::components::external_sessions::State,
    super::project_profiles::State,
    crate::app::components::system_settings_prompts::State,
    crate::app::components::workspace_files::State,
    crate::app::components::daily_memory::State,
    crate::app::components::hermes_memory::State,
    crate::app::components::openclaw_catalog::State,
    crate::app::components::openclaw_tools::State,
    crate::app::components::openclaw_env::State,
    crate::app::components::openclaw_model::State,
    crate::app::components::openclaw_defaults::State,
    crate::app::components::hermes_dashboard::State,
    crate::app::components::system_settings_apps::launch::LaunchState,
    crate::app::components::system_settings_apps::preferences::PreferencesState,
    crate::app::components::system_settings_apps::lifecycle::LifecycleState,
    crate::app::components::system_settings_apps::drag::DragState,
    CatalogRow
>;
pub fn reset_if_context_changed(state: &mut AppsCatalogState, context: &AgentAppsContext) -> bool {
    if state.context.as_ref() == Some(context) {
        return false;
    }
    state.generation = state.generation.wrapping_add(1);
    state.loading = false;
    state.rows.clear();
    state.loaded_at = None;
    state.drag = Default::default();
    state.expanded = None;
    state.hovered = None;
    state.model_diagnostics.invalidate_access();
    state.token.clear();
    state.lifecycle.invalidate();
    state.external_sessions.invalidate();
    state.routing.invalidate();
    state.high_availability.invalidate();
    state.preferences.invalidate();
    state.launch.invalidate();
    state.hermes_dashboard.invalidate();
    state.openclaw_catalog.invalidate();
    state.openclaw_model.invalidate();
    state.openclaw_tools.invalidate();
    state.openclaw_env.invalidate();
    state.openclaw_defaults.invalidate();
    state.workspace_files.invalidate();
    state.daily_memory.invalidate();
    state.hermes_memory.invalidate();
    state.prompts.invalidate();
    state.project_profiles.invalidate();
    state.connection_open = false;
    state.error = Some("连接已更改，正在重新连接应用".into());
    true
}

pub fn begin(state: &mut AppsCatalogState, context: AgentAppsContext) -> u64 {
    state.external_sessions.invalidate();
    state.routing.invalidate();
    state.high_availability.invalidate();
    state.preferences.invalidate();
    state.launch.invalidate();
    state.hermes_dashboard.invalidate();
    state.openclaw_catalog.invalidate();
    state.openclaw_model.invalidate();
    state.openclaw_tools.invalidate();
    state.openclaw_env.invalidate();
    state.openclaw_defaults.invalidate();
    state.workspace_files.invalidate();
    state.daily_memory.invalidate();
    state.hermes_memory.invalidate();
    state.prompts.invalidate();
    state.project_profiles.invalidate();
    state.begin(context)
}

pub fn finish(state: &mut AppsCatalogState, context: &AgentAppsContext, generation: u64, result: Result<Vec<CatalogRow>, String>) -> bool {
    state.finish(context, generation, result, |row| row.observation.is_ok())
}

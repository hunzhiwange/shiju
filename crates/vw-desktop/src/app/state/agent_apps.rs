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
    desktop_settings_skills::third_party::State,
    Option<desktop_settings_mcp::third_party::Scope>,
    CatalogRow
>;
pub fn reset_if_context_changed(state: &mut AppsCatalogState, context: &AgentAppsContext) -> bool {
    if !crate::app::message::settings::apps_catalog::catalog_input(state).reset_if_context_changed(context) { return false; }
    state.model_diagnostics.invalidate_access();
    state.external_sessions.invalidate();
    state.routing.invalidate();
    state.high_availability.invalidate();
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
    true
}

pub fn begin(state: &mut AppsCatalogState, context: AgentAppsContext) -> u64 {
    state.external_sessions.invalidate();
    state.routing.invalidate();
    state.high_availability.invalidate();
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
    crate::app::message::settings::apps_catalog::catalog_input(state).begin(context)
}

pub fn finish(state: &mut AppsCatalogState, context: &AgentAppsContext, generation: u64, result: Result<Vec<CatalogRow>, String>) -> bool {
    crate::app::message::settings::apps_catalog::catalog_input(state).finish(context, generation, result)
}

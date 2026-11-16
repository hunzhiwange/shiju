//! Application gateway route composition and native startup binding.
use axum::{Extension, Router};

pub(crate) fn router() -> Router {
    let router = Router::new()
        .merge(super::device_sync_probe::router())
        .merge(super::backup_inventory::router())
        .merge(super::device_sync_config::router())
        .merge(super::device_sync_jobs::router())
        .merge(super::device_sync_tasks::router())
        .merge(super::agent_apps_data::backup_crud_api::router())
        .merge(super::agent_apps_data::migration_read_api::router())
        .merge(read::router())
        .merge(super::external_sessions::router())
        .merge(super::project_profiles::router())
        .merge(super::agent_app_prompt_files::router())
        .merge(super::agent_app_prompt_templates::router())
        .merge(super::agent_app_prompts_write::router())
        .merge(super::agent_app_prompts_activation::router())
        .merge(super::agent_app_prompt_shared_import::router())
        .merge(super::agent_app_prompts_read::router())
        .merge(super::agent_apps_skills::repos_api::router())
        .merge(super::agent_apps_skills::inventory_api::router())
        .merge(super::agent_apps_skills::install_api::router())
        .merge(super::agent_apps_skills::bindings_api::router())
        .merge(super::agent_apps_skills::storage_api::router())
        .merge(super::agent_apps_skills::jobs_api::router())
        .merge(super::agent_apps_skills::updates_api::router())
        .merge(super::agent_apps_skills::uninstall_api::router())
        .merge(super::agent_apps_skills::backups_api::router())
        .merge(super::agent_apps_skills::import_api::router())
        .merge(super::agent_apps_skills::discovery_api::router())
        .merge(super::agent_apps_skills::claude_api::router())
        .merge(super::agent_apps_skills::search_api::router())
        .merge(super::aggregation_read::router())
        .merge(super::aggregation_desktop::router())
        .merge(super::aggregation_codex_settings::router())
        .merge(super::aggregation_clients::router())
        .merge(super::aggregation_members::router())
        .merge(super::aggregation_omo::router())
        .merge(super::mcp_management_read::router())
        .merge(lifecycle::router())
        .merge(preferences::router())
        .merge(launch::router())
        .merge(usage_read::router_with_policy(
            std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from),
        ))
        .merge(gateway_agent_apps::icons::router());
    #[cfg(target_os = "macos")]
    let router = {
        let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
        router
            .merge(super::agent_apps_data::data_aux_api::router())
            .merge(super::agent_apps_data::backup_restore_api::router())
            .merge(super::agent_app_prompt_sync::router())
            .layer(Extension(super::agent_app_prompt_sync::Jobs::default()))
            .merge(hermes_dashboard::router())
            .merge(hermes_memory::router())
            .merge(openclaw_settings::router())
            .merge(workspace_memory::router())
            .merge(workspace_directory::router())
            .merge(apps_usage_refresh::router_with_policy(policy.clone()))
            .merge(apps_usage_script::router_with_policy(policy))
    };
    router.layer(Extension(super::mcp_management_resync::Jobs::default()))
}

// Icon request handling is owned by gateway-agent-apps::icons.

mod read;
#[cfg(all(test, unix))]
mod read_tests;

mod access;
pub(super) use access::authorize_backup_inventory;
// Account termination authorization is owned by gateway-agent-apps.
#[cfg(target_os = "macos")]
pub(crate) use access::{AuthorizedMcpProbe, McpProbeAccess};

// Authorization handler host resolution lives in gateway-agent-apps.

pub(super) mod lifecycle;

mod preferences;

mod launch;





#[cfg(all(test, unix))]
mod local_access_tests;

pub(super) use gateway_agent_apps::host_context::{provider_config_home, provider_preview_context, provider_editor_context, provider_editor_authorization, authorize_provider_config_access, authorize_provider_login};
#[cfg(target_os = "macos")]
pub(crate) use gateway_agent_apps::host_context::{provider_discovery_host, recover_providers_on_startup, claude_editor_authorization};

#[cfg(target_os = "macos")]
mod usage_refresh;
#[cfg(target_os = "macos")]
pub(crate) use access::{
    RoutingControlScope, authorize_routing_control, authorize_routing_mode, routing_setup,
};
#[cfg(target_os = "macos")]
pub(crate) use usage_refresh::{init as init_usage_refresh, run as run_usage_refresh};

mod usage_read;





#[cfg(target_os = "macos")]
mod apps_usage_refresh;

#[cfg(target_os = "macos")]
mod usage_codex;
#[cfg(target_os = "macos")]
mod usage_native;

#[cfg(target_os = "macos")]
mod apps_usage_script;


pub(super) use gateway_agent_apps::host_context::authorize_mcp_catalog_read;

/// Write to the explicitly granted catalog scope on the requested host.
pub(super) use access::authorize_mcp_catalog_write;

#[cfg(target_os = "macos")]
pub(super) use gateway_agent_apps::host_context::{authorize_mcp_native_read, authorize_mcp_native_write};
#[cfg(target_os = "macos")]
pub(super) use access::{authorize_aggregation_read, authorize_aggregation_write, authorize_pi_directory};

#[cfg(target_os = "macos")]
pub(super) use access::{authorize_diagnostic_job, authorize_diagnostic_probe};

#[cfg(target_os = "macos")]
pub(crate) use access::{aggregation_lifecycle_host, authorize_aggregation_lifecycle};

#[cfg(target_os = "macos")]
pub(super) use access::authorize_codex_restart;

#[cfg(target_os = "macos")]
pub(super) use access::authorize_mcp_acp_read;

pub(super) use access::authorize_skill_repos;

pub(super) use access::authorize_claude_config;

pub(super) use access::authorize_skill_inventory;

pub(super) use access::{authorize_skill_install, authorize_skill_install_owner};

pub(super) use access::authorize_skill_import;

#[cfg(target_os = "macos")]
pub(super) use access::authorize_skill_bindings;

pub(super) use access::authorize_skill_job;

#[cfg(target_os = "macos")]
#[path = "../hermes_dashboard.rs"]
mod hermes_dashboard;

#[cfg(target_os = "macos")]
pub(super) use access::authorize_prompts_read;

#[cfg(target_os = "macos")]
#[path = "../hermes_memory.rs"]
mod hermes_memory;

#[cfg(target_os = "macos")]
pub(super) use access::authorize_prompts_write;

#[cfg(target_os = "macos")]
pub(in super::super) use access::authorize_prompts_activation;

#[cfg(target_os = "macos")]
#[path = "../workspace_memory.rs"]
mod workspace_memory;

#[cfg(target_os = "macos")]
#[path = "../openclaw_settings.rs"]
mod openclaw_settings;

#[cfg(target_os = "macos")]
#[path = "../workspace_directory.rs"]
mod workspace_directory;

pub(super) use access::{authorize_project_profiles, authorize_project_profiles_maintenance};

pub(super) use access::{authorize_external_history, authorize_external_history_delete};

pub(super) use access::authorize_data_backups;

pub(super) use access::authorize_external_history_resume;

pub(super) use access::authorize_external_history_maintenance;

#[cfg(target_os = "macos")]
pub(super) use access::periodic_data_backup_targets;

pub(super) use access::{authenticate_data_migration, authorize_data_migration};

pub(super) use access::authorize_device_sync_upload;

pub(super) use access::authorize_device_sync_config;

pub(super) use access::authorize_device_sync_probe;

#[cfg(target_os = "macos")]
pub(super) use access::data_location_roots;

pub(super) use access::authorize_data_export;

pub(super) use access::authorize_device_sync_download;

pub(super) use access::usage_scan_targets;

pub(super) use access::ExternalHistoryInstance;

pub(super) use access::authorize_usage_ledger_read;

pub(super) use access::authorize_usage_events;

//! Tenant icon library API.
use super::agent_apps_error::AgentAppsApiError;
use crate::{agent_apps::store::IconLibraryStore, global_database::GlobalDatabase};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, rejection::JsonRejection},
    routing::{get, patch},
};
use vw_api_types::{
    agent_apps::{
        AddCustomIconRequest, AddCustomIconResult, AgentAppsErrorCode, CustomIconEntry,
        RenameCustomIconRequest,
    },
    id::TenantScope,
};

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
        .route(
            "/agent-apps/custom-icons",
            get(read_icons).post(add_icon).layer(DefaultBodyLimit::max(128 * 1024)),
        )
        .route(
            "/agent-apps/custom-icons/{id}",
            patch(rename_icon).delete(delete_icon).layer(DefaultBodyLimit::max(128 * 1024)),
        );
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

async fn read_icons(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
) -> Result<Json<Vec<CustomIconEntry>>, AgentAppsApiError> {
    tokio::task::spawn_blocking(move || {
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(AgentAppsApiError::storage_unavailable)?,
        };
        let store =
            IconLibraryStore::new(database).map_err(AgentAppsApiError::storage_unavailable)?;
        store.read_icons(&tenant).map(Json).map_err(AgentAppsApiError::storage_unavailable)
    })
    .await
    .map_err(|error| AgentAppsApiError::storage_unavailable(error.into()))?
}

async fn add_icon(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    request: Result<Json<AddCustomIconRequest>, JsonRejection>,
) -> Result<Json<AddCustomIconResult>, axum::response::Response> {
    use axum::response::IntoResponse;
    let Json(request) = request.map_err(|error| {
        if error.status() == axum::http::StatusCode::PAYLOAD_TOO_LARGE {
            error.into_response()
        } else {
            AgentAppsApiError::from(AgentAppsErrorCode::InvalidInput).into_response()
        }
    })?;
    tokio::task::spawn_blocking(move || -> Result<_, AgentAppsApiError> {
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(AgentAppsApiError::storage_unavailable)?,
        };
        let store =
            IconLibraryStore::new(database).map_err(AgentAppsApiError::storage_unavailable)?;
        store
            .add_icon(
                &tenant,
                &request.name,
                &request.data,
                &uuid::Uuid::new_v4().to_string(),
                chrono::Utc::now().timestamp_millis(),
            )
            .map(Json)
            .map_err(AgentAppsApiError::from)
    })
    .await
    .map_err(|error| AgentAppsApiError::storage_unavailable(error.into()).into_response())?
    .map_err(IntoResponse::into_response)
}

async fn rename_icon(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path(id): Path<String>,
    request: Result<Json<RenameCustomIconRequest>, JsonRejection>,
) -> Result<Json<CustomIconEntry>, axum::response::Response> {
    use axum::response::IntoResponse;
    let Json(request) = request.map_err(|error| {
        if error.status() == axum::http::StatusCode::PAYLOAD_TOO_LARGE {
            error.into_response()
        } else {
            AgentAppsApiError::from(AgentAppsErrorCode::InvalidInput).into_response()
        }
    })?;
    tokio::task::spawn_blocking(move || -> Result<_, AgentAppsApiError> {
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(AgentAppsApiError::storage_unavailable)?,
        };
        let store =
            IconLibraryStore::new(database).map_err(AgentAppsApiError::storage_unavailable)?;
        store.rename_icon(&tenant, &id, &request.name).map(Json).map_err(AgentAppsApiError::from)
    })
    .await
    .map_err(|error| AgentAppsApiError::storage_unavailable(error.into()).into_response())?
    .map_err(IntoResponse::into_response)
}

async fn delete_icon(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, AgentAppsApiError> {
    tokio::task::spawn_blocking(move || {
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(AgentAppsApiError::storage_unavailable)?,
        };
        let store =
            IconLibraryStore::new(database).map_err(AgentAppsApiError::storage_unavailable)?;
        store.delete_icon(&tenant, &id).map_err(AgentAppsApiError::from)?;
        Ok(axum::http::StatusCode::NO_CONTENT)
    })
    .await
    .map_err(|error| AgentAppsApiError::storage_unavailable(error.into()))?
}

mod read;
#[cfg(all(test, unix))]
mod read_tests;

mod access;
pub(super) use access::authorize_backup_inventory;
pub(super) use access::AccountTermination;
#[cfg(target_os = "macos")]
pub(crate) use access::{AuthorizedMcpProbe, McpProbeAccess};

pub(super) struct AuthAccountHost {
    pub id: String,
    pub home: std::path::PathBuf,
    pub credential_root: std::path::PathBuf,
    pub principal: String,
    pub owns_native_slot: bool,
    pub can_set_default: bool,
    pub can_revoke: bool,
}

pub(super) fn authorize_auth_account_read(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: vw_api_types::agent_apps_auth::AuthService,
    bindings: bool,
) -> Result<AuthAccountHost, axum::http::StatusCode> {
    use axum::http::StatusCode;
    use vw_api_types::agent_apps_auth::AuthService;
    let authorized =
        access::authorize_auth_account_read(path, headers, tenant, target, service, bindings)?;
    let owns_native_slot = path.is_some() && authorized.native_login_owner(service) == Some(true);
    if matches!(service, AuthService::AgyOauth | AuthService::DshAccount) && !owns_native_slot {
        return Err(StatusCode::FORBIDDEN);
    }
    account_host(&authorized, owns_native_slot)
}

pub(super) fn authorize_auth_binding(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: vw_api_types::agent_apps_auth::AuthService,
) -> Result<AuthAccountHost, axum::http::StatusCode> {
    use axum::http::StatusCode;
    use vw_api_types::agent_apps_auth::AuthService;

    let authorized = access::authorize_auth_binding(path, headers, tenant, target, service)?;
    let owns_native_slot = path.is_some() && authorized.native_login_owner(service) == Some(true);
    if matches!(service, AuthService::AgyOauth | AuthService::KimiOauth) && !owns_native_slot {
        return Err(StatusCode::FORBIDDEN);
    }
    account_host(&authorized, owns_native_slot)
}

pub(super) fn authorize_auth_default(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: vw_api_types::agent_apps_auth::AuthService,
) -> Result<AuthAccountHost, axum::http::StatusCode> {
    let authorized = access::authorize_auth_default(path, headers, tenant, target, service)?;
    let owns_native_slot = path.is_some() && authorized.native_login_owner(service) == Some(true);
    account_host(&authorized, owns_native_slot)
}

pub(super) fn authorize_auth_account_delete(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: vw_api_types::agent_apps_auth::AuthService,
) -> Result<AuthAccountHost, axum::http::StatusCode> {
    use axum::http::StatusCode;
    use vw_api_types::agent_apps_auth::AuthService;

    let authorized = access::authorize_auth_account_delete(path, headers, tenant, target, service)?;
    let owns_native_slot = path.is_some() && authorized.native_login_owner(service) == Some(true);
    if matches!(service, AuthService::AgyOauth | AuthService::DshAccount) && !owns_native_slot {
        return Err(StatusCode::FORBIDDEN);
    }
    account_host(&authorized, owns_native_slot)
}

pub(super) fn authorize_auth_account_termination(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: vw_api_types::agent_apps_auth::AuthService,
    action: AccountTermination,
) -> Result<AuthAccountHost, axum::http::StatusCode> {
    use axum::http::StatusCode;
    use vw_api_types::agent_apps_auth::AuthService;

    let authorized =
        access::authorize_auth_account_termination(path, headers, tenant, target, service, action)?;
    let owns_native_slot = path.is_some() && authorized.native_login_owner(service) == Some(true);
    if (service == AuthService::DshAccount
        || service == AuthService::AgyOauth && matches!(action, AccountTermination::Logout))
        && !owns_native_slot
    {
        return Err(StatusCode::FORBIDDEN);
    }
    account_host(&authorized, owns_native_slot)
}

fn account_host(
    authorized: &access::Authorization,
    owns_native_slot: bool,
) -> Result<AuthAccountHost, axum::http::StatusCode> {
    use axum::http::StatusCode;
    Ok(AuthAccountHost {
        id: authorized.host.id.clone(),
        home: authorized.host.home.clone(),
        credential_root: authorized
            .host
            .credential_root
            .clone()
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)?,
        principal: authorized.principal.clone(),
        owns_native_slot,
        can_set_default: authorized.can_set_default,
        can_revoke: authorized.can_revoke,
    })
}

pub(super) struct AuthStartHost {
    pub id: String,
    pub principal: String,
    pub credential_root: std::path::PathBuf,
    pub browser_local: bool,
    pub github_enterprise_authorities: Vec<String>,
    #[cfg(target_os = "macos")]
    pub environment: vw_agent_apps::apps::discovery::read::LocalReadEnvironment,
}

pub(super) fn authorize_auth_flow_start(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: vw_api_types::agent_apps_auth::AuthService,
) -> Result<AuthStartHost, axum::http::StatusCode> {
    use axum::http::StatusCode;
    use vw_api_types::agent_apps_auth::AuthService;

    let authorized = access::authorize_auth_flow_start(path, headers, tenant, target, service)?;
    if matches!(service, AuthService::AgyOauth | AuthService::DshAccount)
        && !(path.is_some() && authorized.native_login_owner(service) == Some(true))
    {
        return Err(StatusCode::FORBIDDEN);
    }
    let host = authorized.host;
    Ok(AuthStartHost {
        #[cfg(target_os = "macos")]
        environment: vw_agent_apps::apps::discovery::read::LocalReadEnvironment {
            host_id: host.id.clone(),
            home: host.home,
            search_paths: host.search_paths,
            applications: host.applications,
        },
        id: host.id,
        principal: authorized.principal,
        credential_root: host.credential_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?,
        browser_local: authorized.auth_browser_local,
        github_enterprise_authorities: host.github_enterprise_authorities,
    })
}

pub(super) mod lifecycle;
#[cfg(all(test, unix))]
mod lifecycle_tests;

mod preferences;
#[cfg(all(test, unix))]
mod preferences_tests;

mod launch;

mod launch_identities;
mod launch_sessions;

#[cfg(all(test, unix))]
mod remote_tests_tests;

#[cfg(all(test, unix))]
mod local_access_tests;

/// Share host authorization without exposing policy records or broadening catalog grants.
pub(super) fn provider_config_home(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<std::path::PathBuf, axum::http::StatusCode> {
    access::authorize_provider_config(path, headers, tenant, target).map(|host| host.home)
}

/// Resolve discovery's local execution environment and task owner from the same grant.
#[cfg(target_os = "macos")]
pub(super) fn provider_discovery_host(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<
    (vw_agent_apps::apps::discovery::read::LocalReadEnvironment, String),
    axum::http::StatusCode,
> {
    use vw_agent_apps::apps::discovery::read::LocalReadEnvironment;
    let (host, principal) = access::authorize_provider_discovery(path, headers, tenant, target)?;
    Ok((
        LocalReadEnvironment {
            host_id: host.id,
            home: host.home,
            search_paths: host.search_paths,
            applications: host.applications,
        },
        principal,
    ))
}

/// Runs before the gateway accepts requests, without depending on a desktop page.
#[cfg(target_os = "macos")]
pub(crate) fn recover_providers_on_startup() -> Result<(), String> {
    if let Some(path) = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE") {
        return startup_provider_policy(std::path::Path::new(&path));
    }
    // The configured runtime root locates existing tenant policies only. Live
    // resource paths always come from previously persisted executor registration.
    let directory =
        vw_config_types::paths::home_config_dir(&crate::global::paths().home).join("local-apps");
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("provider recovery policy directory unavailable".into()),
    };
    for entry in entries {
        let entry = entry.map_err(|_| "provider recovery policy directory unavailable")?;
        if !entry
            .file_type()
            .map_err(|_| "provider recovery policy directory unavailable")?
            .is_dir()
        {
            continue;
        }
        let path = entry.path().join("access.json");
        if path.try_exists().map_err(|_| "provider recovery policy unavailable")? {
            startup_provider_policy(&path)?;
        }
    }
    Ok(())
}

#[cfg(all(test, target_os = "macos"))]
mod recovery_tests;

/// Preview needs the same authorized execution identity and existing private state root.
pub(super) fn provider_preview_context(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<(std::path::PathBuf, Option<std::path::PathBuf>), axum::http::StatusCode> {
    access::authorize_provider_config(path, headers, tenant, target)
        .map(|host| (host.home, host.credential_root))
}

pub(super) fn provider_editor_context(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    write: bool,
) -> Result<(std::path::PathBuf, std::path::PathBuf), axum::http::StatusCode> {
    provider_editor_authorization(path, headers, tenant, target, write)
        .map(|(home, root, _)| (home, root))
}

/// Resolves the principal alongside the execution paths under a single policy decision.
pub(super) fn provider_editor_authorization(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    write: bool,
) -> Result<(std::path::PathBuf, std::path::PathBuf, String), axum::http::StatusCode> {
    let authorization = access::authorize_provider_editor(path, headers, tenant, target, write)?;
    let host = authorization.host;
    let root = host.credential_root.ok_or(axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
    Ok((host.home, root, authorization.principal))
}

/// Native Claude checkout requires a separately authorized default login slot.
#[cfg(target_os = "macos")]
pub(super) fn claude_editor_authorization(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<(std::path::PathBuf, std::path::PathBuf, String, Option<bool>), axum::http::StatusCode>
{
    let authorization = access::authorize_provider_editor(path, headers, tenant, target, true)?;
    let owns_slot = authorization
        .native_login_owner(vw_api_types::agent_apps_auth::AuthService::ClaudeOfficial)
        .map(|owns| owns && path.is_some());
    let host = authorization.host;
    let root = host.credential_root.ok_or(axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
    Ok((host.home, root, authorization.principal, owns_slot))
}

/// Database-only provider operations require a host grant without opening its private file root.
pub(super) fn authorize_provider_config_access(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    write: bool,
) -> Result<(), axum::http::StatusCode> {
    if write {
        access::authorize_provider_write(path, headers, tenant, target).map(|_| ())
    } else {
        access::authorize_provider_config(path, headers, tenant, target).map(|_| ())
    }
}

pub(super) fn authorize_provider_login(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    account: &str,
) -> Result<(), axum::http::StatusCode> {
    access::authorize_provider_login(path, headers, tenant, target, account)
}

#[cfg(target_os = "macos")]
fn startup_provider_policy(path: &std::path::Path) -> Result<(), String> {
    access::recover_provider_policy(path).map_err(|_| "provider recovery policy unavailable")?;
    let database = crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home)
        .map_err(|_| "provider import storage unavailable")?;
    access::import_provider_policy(path, &database)
        .map_err(|_| "provider import policy unavailable".into())
}

#[cfg(all(test, target_os = "macos"))]
mod import_tests;

#[cfg(target_os = "macos")]
mod usage_refresh;
#[cfg(target_os = "macos")]
pub(crate) use access::{
    RoutingControlScope, authorize_routing_control, authorize_routing_mode, routing_setup,
};
#[cfg(target_os = "macos")]
pub(crate) use usage_refresh::{init as init_usage_refresh, run as run_usage_refresh};

mod usage_read;

#[cfg(all(test, target_os = "macos"))]
mod usage_refresh_tests;

#[cfg(all(test, target_os = "macos"))]
mod usage_read_tests;

#[cfg(target_os = "macos")]
mod apps_usage_refresh;
#[cfg(all(test, target_os = "macos"))]
mod apps_usage_refresh_tests;
#[cfg(target_os = "macos")]
mod usage_codex;
#[cfg(target_os = "macos")]
mod usage_native;

#[cfg(target_os = "macos")]
mod apps_usage_script;
#[cfg(all(test, target_os = "macos"))]
mod apps_usage_script_tests;

/// Read the MCP catalog under the existing tenant and host membership grant.
pub(super) fn authorize_mcp_catalog_read(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<(), axum::http::StatusCode> {
    access::authorize(path, headers, tenant, target).map(|_| ())
}

/// Write to the explicitly granted catalog scope on the requested host.
pub(super) use access::authorize_mcp_catalog_write;

/// Native preview requires the host's configuration read grant.
#[cfg(target_os = "macos")]
pub(super) fn authorize_mcp_native_read(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<vw_agent_apps::apps::discovery::read::LocalReadEnvironment, axum::http::StatusCode> {
    access::authorize_provider_config(path, headers, tenant, target).map(|host| {
        vw_agent_apps::apps::discovery::read::LocalReadEnvironment {
            host_id: host.id,
            home: host.home,
            search_paths: host.search_paths,
            applications: host.applications,
        }
    })
}

#[cfg(target_os = "macos")]
pub(super) use access::{
    authorize_aggregation_read, authorize_aggregation_write, authorize_pi_directory,
};

/// Native cleanup uses the same administrator-owned roots and write grant as provider edits.
#[cfg(target_os = "macos")]
pub(super) fn authorize_mcp_native_write(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<
    (vw_agent_apps::apps::discovery::read::LocalReadEnvironment, std::path::PathBuf),
    axum::http::StatusCode,
> {
    let host = access::authorize_provider_write(path, headers, tenant, target)?;
    let root = host.credential_root.ok_or(axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
    Ok((
        vw_agent_apps::apps::discovery::read::LocalReadEnvironment {
            host_id: host.id,
            home: host.home,
            search_paths: host.search_paths,
            applications: host.applications,
        },
        root,
    ))
}

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

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
    Router::new()
        .merge(read::router())
        .merge(lifecycle::router())
        .merge(preferences::router())
        .merge(launch::router())
        .route(
            "/agent-apps/custom-icons",
            get(read_icons).post(add_icon).layer(DefaultBodyLimit::max(128 * 1024)),
        )
        .route(
            "/agent-apps/custom-icons/{id}",
            patch(rename_icon).delete(delete_icon).layer(DefaultBodyLimit::max(128 * 1024)),
        )
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

mod lifecycle;
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
    let host = if write {
        access::authorize_provider_write(path, headers, tenant, target)?
    } else {
        access::authorize_provider_config(path, headers, tenant, target)?
    };
    Ok((host.home, host.credential_root.ok_or(axum::http::StatusCode::SERVICE_UNAVAILABLE)?))
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

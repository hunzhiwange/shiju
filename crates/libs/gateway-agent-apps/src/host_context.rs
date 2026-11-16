//! Authorized execution context projections and startup policy recovery.
use crate::access;
use vw_api_transport::TenantScope;

/// Share host authorization without exposing policy records or broadening catalog grants.
pub fn provider_config_home(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<std::path::PathBuf, axum::http::StatusCode> {
    access::authorize_provider_config(path, headers, tenant, target).map(|host| host.home)
}

/// Resolve discovery's local execution environment and task owner from the same grant.
#[cfg(target_os = "macos")]
pub fn provider_discovery_host(
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
pub fn recover_providers_on_startup() -> Result<(), String> {
    if let Some(path) = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE") {
        return startup_provider_policy(std::path::Path::new(&path));
    }
    // The configured runtime root locates existing tenant policies only. Live
    // resource paths always come from previously persisted executor registration.
    let directory =
        config_runtime_paths::home_config_dir(&agent_paths::paths().home).join("local-apps");
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


/// Preview needs the same authorized execution identity and existing private state root.
pub fn provider_preview_context(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<(std::path::PathBuf, Option<std::path::PathBuf>), axum::http::StatusCode> {
    access::authorize_provider_config(path, headers, tenant, target)
        .map(|host| (host.home, host.credential_root))
}

pub fn provider_editor_context(
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
pub fn provider_editor_authorization(
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
pub fn claude_editor_authorization(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<(std::path::PathBuf, std::path::PathBuf, String, Option<bool>), axum::http::StatusCode>
{
    let authorization = access::authorize_provider_editor(path, headers, tenant, target, true)?;
    let owns_slot = authorization
        .native_login_owner(vw_api_app_auth::AuthService::ClaudeOfficial)
        .map(|owns| owns && path.is_some());
    let host = authorization.host;
    let root = host.credential_root.ok_or(axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
    Ok((host.home, root, authorization.principal, owns_slot))
}

/// Database-only provider operations require a host grant without opening its private file root.
pub fn authorize_provider_config_access(
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

pub fn authorize_provider_login(
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
    let database = agent_database::GlobalDatabase::for_home(&agent_paths::paths().home)
        .map_err(|_| "provider import storage unavailable")?;
    access::import_provider_policy(path, &database)
        .map_err(|_| "provider import policy unavailable".into())
}

/// Read the MCP catalog under the existing tenant and host membership grant.
pub fn authorize_mcp_catalog_read(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<(), axum::http::StatusCode> {
    access::authorize(path, headers, tenant, target).map(|_| ())
}

/// Native preview requires the host's configuration read grant.
#[cfg(target_os = "macos")]
pub fn authorize_mcp_native_read(
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


/// Native cleanup uses the same administrator-owned roots and write grant as provider edits.
#[cfg(target_os = "macos")]
pub fn authorize_mcp_native_write(
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

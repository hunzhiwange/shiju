use app_access_service::access;
pub use app_access_service::access::*;
use vw_api_transport::TenantScope;
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

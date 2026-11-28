use app_access_service::access;
pub use app_access_service::access::*;
use vw_api_transport::TenantScope;
/// Read global MCP form options under the existing tenant and host membership grant.
pub fn authorize_global_mcp_options(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<(), axum::http::StatusCode> {
    let host = access::authorize(path, headers, tenant, target)?;
    if host.id == target { Ok(()) } else { Err(axum::http::StatusCode::NOT_FOUND) }
}

/// Native synchronization uses the same administrator-owned roots and write grant as provider edits.
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

//! Shape projections of the shared authorization owner; no policy is duplicated.
use vw_api_transport::id::TenantScope;
use app_access_service::access;
#[cfg(target_os = "macos")]
pub(crate) use access::authorize_provider_login;
pub(crate) fn provider_config_home(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<std::path::PathBuf, axum::http::StatusCode> {
    access::authorize_provider_config(path, headers, tenant, target).map(|host| host.home)
}


pub(crate) fn provider_editor_context(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    write: bool,
) -> Result<(std::path::PathBuf, std::path::PathBuf), axum::http::StatusCode> {
    provider_editor_authorization(path, headers, tenant, target, write)
        .map(|(home, root, _)| (home, root))
}


pub(crate) fn provider_editor_authorization(
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


#[cfg(target_os = "macos")]
pub(crate) fn claude_editor_authorization(
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

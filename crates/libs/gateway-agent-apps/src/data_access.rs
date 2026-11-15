//! Authorization used by data routes; policy and grants retain their service owner.
pub(crate) fn authorize_provider_config_access(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &vw_api_transport::id::TenantScope,
    target: &str,
    write: bool,
) -> Result<(), axum::http::StatusCode> {
    if write {
        crate::access::authorize_provider_write(path, headers, tenant, target).map(|_| ())
    } else {
        crate::access::authorize_provider_config(path, headers, tenant, target).map(|_| ())
    }
}

use crate::access;
pub(crate) use app_access_service::access::AccountTermination;
use vw_api_transport::id::TenantScope;
pub(crate) use app_access_service::account_host::AuthAccountHost;

pub(crate) fn authorize_auth_account_read(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: vw_api_app_auth::AuthService,
    bindings: bool,
) -> Result<AuthAccountHost, axum::http::StatusCode> {
    use axum::http::StatusCode;
    use vw_api_app_auth::AuthService;
    let authorized =
        access::authorize_auth_account_read(path, headers, tenant, target, service, bindings)?;
    let owns_native_slot = path.is_some() && authorized.native_login_owner(service) == Some(true);
    if matches!(service, AuthService::AgyOauth | AuthService::DshAccount) && !owns_native_slot {
        return Err(StatusCode::FORBIDDEN);
    }
    account_host(&authorized, owns_native_slot)
}

pub(crate) fn authorize_auth_binding(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: vw_api_app_auth::AuthService,
) -> Result<AuthAccountHost, axum::http::StatusCode> {
    use axum::http::StatusCode;
    use vw_api_app_auth::AuthService;

    let authorized = access::authorize_auth_binding(path, headers, tenant, target, service)?;
    let owns_native_slot = path.is_some() && authorized.native_login_owner(service) == Some(true);
    if matches!(service, AuthService::AgyOauth | AuthService::KimiOauth) && !owns_native_slot {
        return Err(StatusCode::FORBIDDEN);
    }
    account_host(&authorized, owns_native_slot)
}

pub(crate) fn authorize_auth_default(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: vw_api_app_auth::AuthService,
) -> Result<AuthAccountHost, axum::http::StatusCode> {
    let authorized = access::authorize_auth_default(path, headers, tenant, target, service)?;
    let owns_native_slot = path.is_some() && authorized.native_login_owner(service) == Some(true);
    account_host(&authorized, owns_native_slot)
}

pub(crate) fn authorize_auth_account_delete(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: vw_api_app_auth::AuthService,
) -> Result<AuthAccountHost, axum::http::StatusCode> {
    use axum::http::StatusCode;
    use vw_api_app_auth::AuthService;

    let authorized = access::authorize_auth_account_delete(path, headers, tenant, target, service)?;
    let owns_native_slot = path.is_some() && authorized.native_login_owner(service) == Some(true);
    if matches!(service, AuthService::AgyOauth | AuthService::DshAccount) && !owns_native_slot {
        return Err(StatusCode::FORBIDDEN);
    }
    account_host(&authorized, owns_native_slot)
}

pub(crate) fn authorize_auth_account_termination(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: vw_api_app_auth::AuthService,
    action: AccountTermination,
) -> Result<AuthAccountHost, axum::http::StatusCode> {
    use axum::http::StatusCode;
    use vw_api_app_auth::AuthService;

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

use app_access_service::account_host::account_host;

pub(crate) struct AuthStartHost {
    pub id: String,
    pub principal: String,
    pub credential_root: std::path::PathBuf,
    pub browser_local: bool,
    pub github_enterprise_authorities: Vec<String>,
    #[cfg(target_os = "macos")]
    pub environment: vw_agent_apps::apps::discovery::read::LocalReadEnvironment,
}

pub(crate) fn authorize_auth_flow_start(
    path: Option<&std::path::Path>,
    headers: &axum::http::HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: vw_api_app_auth::AuthService,
) -> Result<AuthStartHost, axum::http::StatusCode> {
    use axum::http::StatusCode;
    use vw_api_app_auth::AuthService;

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

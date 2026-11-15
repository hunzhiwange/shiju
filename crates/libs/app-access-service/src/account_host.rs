use crate::access;
pub use agent_app_services::auth_host::AuthAccountHost;
pub fn account_host(
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

//! Resolve the session user against gateway-controlled credential metadata.

use axum::{
    extract::Request,
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use config_gateway::GatewaySkey;
use gateway_pairing::pairing::{PairingGuard, constant_time_eq, hash_skey};
use vw_api_transport::{DEFAULT_USER_ID, TenantScope, USER_ID_HEADER};

use super::ApiError;

pub(super) async fn session_user(mut request: Request, next: Next) -> Response {
    if request.method() == Method::OPTIONS {
        return next.run(request).await;
    }
    let Some(tenant) = request.extensions().get::<TenantScope>().cloned() else {
        return ApiError::internal("missing gateway tenant context").into_response();
    };
    let user = match gateway_types::tenant_scope::user_id(request.headers()) {
        Ok(user) => user,
        Err(error) => return error.into_response(),
    };
    let token = gateway_types::auth::extract_auth_skey(request.headers());
    if let Some(token) = token {
        let skeys = match crate::config::load_gateway_skeys_for_tenant(&tenant).await {
            Ok(skeys) => skeys,
            Err(_) => {
                return ApiError::internal("gateway credential store unavailable").into_response();
            }
        };
        if !allows_user(skeys, token, user.as_ref()) {
            return unauthorized();
        }
    } else if user.as_ref() != DEFAULT_USER_ID {
        return unauthorized();
    }

    request.extensions_mut().insert(tenant.with_user_id(user));
    next.run(request).await
}

fn allows_user(skeys: Vec<GatewaySkey>, token: &str, user_id: &str) -> bool {
    let digest = hash_skey(token);
    let key = skeys.into_iter().find(|key| constant_time_eq(&key.skey_hash, &digest));
    if let Some(key) = key {
        return key.user_id.as_deref().unwrap_or(DEFAULT_USER_ID) == user_id
            && (user_id == DEFAULT_USER_ID
                || PairingGuard::from_skeys(true, &[key]).is_authenticated(token));
    }
    user_id == DEFAULT_USER_ID
}

fn unauthorized() -> Response {
    ApiError {
        status: StatusCode::UNAUTHORIZED,
        message: format!("{USER_ID_HEADER} must match the gateway credential"),
    }
    .into_response()
}

#[cfg(test)]
#[path = "user_scope_tests.rs"]
mod tests;

//! Shared tenant-header validation for the gateway request boundary.
use crate::ApiError;
use axum::{http::{HeaderMap, Method, Request}, middleware::Next, response::{IntoResponse, Response}};
use vw_api_transport::id::{TenantScope, PLATFORM_ID_HEADER, COMPANY_ID_HEADER};
pub async fn tenant_scope_middleware(
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    if req.method() == Method::OPTIONS {
        return next.run(req).await;
    }

    let platform_id = match required_tenant_header(req.headers(), PLATFORM_ID_HEADER) {
        Ok(value) => value,
        Err(error) => return error.into_response(),
    };
    let company_id = match required_tenant_header(req.headers(), COMPANY_ID_HEADER) {
        Ok(value) => value,
        Err(error) => return error.into_response(),
    };
    let scope = match TenantScope::new(platform_id, company_id) {
        Ok(scope) => scope,
        Err(error) => return ApiError::bad_request(error.to_string()).into_response(),
    };

    req.extensions_mut().insert(scope);
    next.run(req).await
}

fn required_tenant_header<'a>(
    headers: &'a HeaderMap,
    name: &'static str,
) -> Result<&'a str, ApiError> {
    let mut values = headers.get_all(name).iter();
    let Some(value) = values.next() else {
        return Err(ApiError::bad_request(format!("missing {name} header")));
    };
    if values.next().is_some() {
        return Err(ApiError::bad_request(format!("{name} header must have a single value")));
    }

    let value = value
        .to_str()
        .map_err(|_| ApiError::bad_request(format!("{name} header must be valid UTF-8")))?;
    if value.trim().is_empty() {
        return Err(ApiError::bad_request(format!("{name} header must not be empty")));
    }
    Ok(value)
}

#[cfg(test)]
#[path = "tenant_scope_tests.rs"]
mod tests;

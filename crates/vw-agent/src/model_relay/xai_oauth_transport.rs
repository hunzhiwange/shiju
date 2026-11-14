//! xAI managed-token wire boundary; refresh and account selection stay with the auth owner.
use axum::http::{HeaderMap, StatusCode};

/// Managed xAI credentials must never follow an editable third-party base URL.
pub(crate) fn validate_url(url: &reqwest::Url) -> Result<(), StatusCode> {
    if url.scheme() != "https"
        || url.host_str() != Some("api.x.ai")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "/v1/chat/completions" | "/v1/responses")
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(())
}

/// Apply after all client/provider overrides, using only the selected account's token.
pub(crate) fn apply(request: &mut reqwest::Request, fresh: &HeaderMap) -> Result<(), StatusCode> {
    validate_url(request.url())?;
    let authorization = fresh.get("authorization").ok_or(StatusCode::UNAUTHORIZED)?;
    if !authorization.to_str().ok().and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|token| !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_graphic()))
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let headers = request.headers_mut();
    for name in ["authorization", "cookie", "proxy-authorization", "x-api-key", "x-goog-api-key", "chatgpt-account-id"] {
        headers.remove(name);
    }
    let mut authorization = authorization.clone();
    authorization.set_sensitive(true);
    headers.insert("authorization", authorization);
    Ok(())
}

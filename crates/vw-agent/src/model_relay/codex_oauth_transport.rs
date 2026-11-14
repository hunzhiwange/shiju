//! Official Codex OAuth wire boundary. Account lifecycle stays with the auth owner.
use axum::http::{HeaderMap, HeaderValue, StatusCode};

/// Never send an official account token to a provider-configured third party.
pub(crate) fn validate_url(url: &reqwest::Url) -> Result<(), StatusCode> {
    if url.scheme() != "https"
        || url.host_str() != Some("chatgpt.com")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(),
            "/backend-api/codex/responses" | "/backend-api/codex/responses/compact")
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(())
}

/// Use only the freshly resolved account bundle, after client/provider overrides.
pub(crate) fn apply(request: &mut reqwest::Request, fresh: &HeaderMap) -> Result<(), StatusCode> {
    validate_url(request.url())?;
    apply_headers(request, fresh)
}

/// Search has a separate official endpoint; generation URL policy stays strict.
pub(crate) fn apply_alpha_search(request: &mut reqwest::Request, fresh: &HeaderMap) -> Result<(), StatusCode> {
    let mut generation = request.url().clone();
    if generation.path() != "/backend-api/codex/alpha/search" {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    generation.set_path("/backend-api/codex/responses");
    generation.set_query(None);
    validate_url(&generation)?;
    apply_headers(request, fresh)
}

fn apply_headers(request: &mut reqwest::Request, fresh: &HeaderMap) -> Result<(), StatusCode> {
    let account = fresh.get("chatgpt-account-id").ok_or(StatusCode::UNAUTHORIZED)?;
    let account_text = account.to_str().map_err(|_| StatusCode::UNAUTHORIZED)?;
    if account_text.is_empty() || !account_text.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let authorization = fresh.get("authorization").ok_or(StatusCode::UNAUTHORIZED)?;
    if !authorization.to_str().ok().and_then(|s| s.strip_prefix("Bearer "))
        .is_some_and(|token| !token.is_empty() && token.bytes().all(|b| b.is_ascii_graphic()))
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let headers = request.headers_mut();
    for name in ["authorization", "chatgpt-account-id", "x-api-key", "x-goog-api-key"] {
        headers.remove(name);
    }
    let mut authorization = authorization.clone();
    authorization.set_sensitive(true);
    let mut account = account.clone();
    account.set_sensitive(true);
    headers.insert("authorization", authorization);
    headers.insert("chatgpt-account-id", account);
    headers.insert("originator", HeaderValue::from_static("codex_cli_rs"));
    headers.insert("version", HeaderValue::from_static("0.159.0"));
    Ok(())
}

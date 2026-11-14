//! Google authentication wire differences. Refresh stays with AuthAccountStore.
use axum::http::{HeaderValue, StatusCode, header};

/// Check before resolving a managed grant and again at the final wire boundary.
pub(crate) fn validate_url(url: &reqwest::Url) -> Result<(), StatusCode> {
    let resource = url.path().strip_prefix("/v1beta/models/")
        .or_else(|| url.path().strip_prefix("/v1/models/"));
    let valid_resource = resource.and_then(|resource| resource.rsplit_once(':'))
        .is_some_and(|(model, action)| {
            !model.is_empty()
                && model.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                && matches!(action, "generateContent" | "streamGenerateContent")
        });
    if url.scheme() != "https"
        || url.host_str() != Some("generativelanguage.googleapis.com")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.query().is_some_and(|query| {
            query != "alt=sse" || !url.path().ends_with(":streamGenerateContent")
        })
        || !valid_resource
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(())
}

pub(crate) fn apply_oauth(request: &mut reqwest::Request) -> Result<(), StatusCode> {
    validate_url(request.url())?;
    let mut authorization = request.headers().get(header::AUTHORIZATION)
        .cloned().ok_or(StatusCode::UNAUTHORIZED)?;
    if !authorization.to_str().ok().and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|token| !token.is_empty() && token.bytes().all(|b| b.is_ascii_graphic()))
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    clear_auth(request);
    authorization.set_sensitive(true);
    request.headers_mut().insert(header::AUTHORIZATION, authorization);
    request.headers_mut().insert("x-goog-api-client", HeaderValue::from_static("GeminiCLI/1.0"));
    Ok(())
}

pub(crate) fn apply_api_key(
    request: &mut reqwest::Request,
    key: &HeaderValue,
) -> Result<(), StatusCode> {
    // Static Google keys are selected by publication, never inferred as OAuth.
    if key.as_bytes().is_empty() || !key.as_bytes().iter().all(u8::is_ascii_graphic) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    clear_auth(request);
    let mut key = key.clone();
    key.set_sensitive(true);
    request.headers_mut().insert("x-goog-api-key", key);
    Ok(())
}

fn clear_auth(request: &mut reqwest::Request) {
    for name in ["authorization", "x-api-key", "x-goog-api-key", "x-goog-api-client", "cookie", "proxy-authorization"] {
        request.headers_mut().remove(name);
    }
}

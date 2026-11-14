//! Codex Images endpoint policy; no generation schema or tool execution.
use axum::http::{HeaderValue, StatusCode, Uri};

pub(crate) fn is_path(path: &str) -> bool {
    matches!(path, "/images/generations" | "/v1/images/generations"
        | "/v1/v1/images/generations" | "/codex/v1/images/generations")
}

/// Preserve the selected provider's prefix and query, including complete Images
/// URLs. Reject opaque targets rather than guessing an API root.
pub(crate) fn url(target: &reqwest::Url, incoming: &Uri) -> Result<reqwest::Url, StatusCode> {
    let path = target.path().trim_end_matches('/');
    let lower = path.to_ascii_lowercase();
    let suffix = ["/images/generations", "/responses/compact", "/responses", "/chat/completions"]
        .into_iter().find(|suffix| lower.ends_with(suffix))
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    let mut url = target.clone();
    url.set_path(&format!("{}/images/generations", &path[..path.len() - suffix.len()]));
    let query = match (target.query().filter(|q| !q.is_empty()), incoming.query().filter(|q| !q.is_empty())) {
        (Some(base), Some(client)) => Some(format!("{base}&{client}")),
        (base, client) => base.or(client).map(str::to_owned),
    };
    url.set_query(query.as_deref());
    url.set_fragment(None);
    Ok(url)
}

pub(crate) fn is_result_type(value: &HeaderValue) -> bool {
    let Ok(value) = value.to_str() else { return false; };
    let mime = value.split(';').next().unwrap_or_default().trim();
    mime.eq_ignore_ascii_case("application/json")
        || mime.eq_ignore_ascii_case("application/octet-stream")
        || matches!(mime.to_ascii_lowercase().as_str(), "image/png" | "image/jpeg" | "image/webp" | "image/gif" | "image/avif")
}

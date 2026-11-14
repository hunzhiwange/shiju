//! Native compact endpoint differences for the resident application listener.
use axum::http::StatusCode;

pub(crate) fn is_path(path: &str) -> bool {
    matches!(path,
        "/responses/compact" | "/v1/responses/compact" | "/v1/v1/responses/compact"
        | "/codex/v1/responses/compact" | "/grokbuild/v1/responses/compact")
}

/// Extend only the selected native Responses target; never infer a different provider.
pub(crate) fn url(target: &reqwest::Url) -> Result<reqwest::Url, StatusCode> {
    let path = target.path().trim_end_matches('/');
    if !path.ends_with("/responses") && !path.ends_with("/responses/compact") {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let mut url = target.clone();
    if !path.ends_with("/responses/compact") {
        url.set_path(&format!("{path}/compact"));
    }
    Ok(url)
}

//! Codex standalone search: endpoint selection only, without generation bridges.
use axum::http::{StatusCode, Uri};

pub(crate) fn is_path(path: &str) -> bool {
    matches!(path, "/alpha/search" | "/v1/alpha/search" | "/v1/v1/alpha/search" | "/codex/v1/alpha/search")
}

/// Only Responses publications have an unambiguous search sibling. Preserve
/// the original prefix (including escaped segments) and both query strings.
pub(crate) fn url(target: &reqwest::Url, incoming: &Uri) -> Result<reqwest::Url, StatusCode> {
    let path = target.path().trim_end_matches('/');
    let lower = path.to_ascii_lowercase();
    let suffix = ["/responses/compact", "/responses"].into_iter()
        .find(|suffix| lower.ends_with(suffix)).ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    let mut url = target.clone();
    url.set_path(&format!("{}/alpha/search", &path[..path.len() - suffix.len()]));
    let query = match (target.query().filter(|q| !q.is_empty()), incoming.query().filter(|q| !q.is_empty())) {
        (Some(base), Some(client)) => Some(format!("{base}&{client}")),
        (base, client) => base.or(client).map(str::to_owned),
    };
    url.set_query(query.as_deref());
    url.set_fragment(None);
    Ok(url)
}

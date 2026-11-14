//! Images edits policy. Multipart bytes remain opaque; filenames are never opened.
use axum::http::{header, HeaderMap, HeaderValue, StatusCode, Uri};

pub(crate) fn is_path(path: &str) -> bool {
    matches!(path, "/images/edits" | "/v1/images/edits"
        | "/v1/v1/images/edits" | "/codex/v1/images/edits")
}

pub(crate) fn url(target: &reqwest::Url, incoming: &Uri) -> Result<reqwest::Url, StatusCode> {
    let mut target = target.clone();
    let path = target.path().trim_end_matches('/');
    if path.to_ascii_lowercase().ends_with("/images/edits") {
        let path = format!("{}/images/generations", &path[..path.len() - "/images/edits".len()]);
        target.set_path(&path);
    }
    let mut url = super::image_generation::url(&target, incoming)?;
    let path = url.path();
    let path = format!("{}/images/edits", &path[..path.len() - "/images/generations".len()]);
    url.set_path(&path);
    Ok(url)
}

/// Require a single unambiguous media type and a valid MIME boundary. Preserve
/// the complete original header, including quoting, to match the untouched body.
pub(crate) fn content_type(headers: &HeaderMap) -> Result<HeaderValue, StatusCode> {
    let mut values = headers.get_all(header::CONTENT_TYPE).iter();
    let value = values.next().ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;
    if values.next().is_some() { return Err(StatusCode::BAD_REQUEST); }
    let text = value.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut parts = text.split(';');
    let media = parts.next().unwrap_or_default().trim();
    if media.eq_ignore_ascii_case("application/json") { return Ok(value.clone()); }
    if !media.eq_ignore_ascii_case("multipart/form-data") {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    let mut boundary = None;
    for part in parts {
        let (name, raw) = part.trim().split_once('=').ok_or(StatusCode::BAD_REQUEST)?;
        if !name.trim().eq_ignore_ascii_case("boundary") { continue; }
        if boundary.is_some() { return Err(StatusCode::BAD_REQUEST); }
        let raw = raw.trim();
        let decoded = if raw.starts_with('"') {
            raw.strip_prefix('"').and_then(|s| s.strip_suffix('"')).ok_or(StatusCode::BAD_REQUEST)?
        } else {
            // Unquoted MIME parameters must be tokens.
            if !raw.bytes().all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)) {
                return Err(StatusCode::BAD_REQUEST);
            }
            raw
        };
        if decoded.is_empty() || decoded.len() > 70 || decoded.ends_with(' ')
            || !decoded.bytes().all(|b| b.is_ascii_alphanumeric() || b"'()+_,-./:=? ".contains(&b))
        {
            return Err(StatusCode::BAD_REQUEST);
        }
        boundary = Some(decoded);
    }
    boundary.ok_or(StatusCode::BAD_REQUEST)?;
    Ok(value.clone())
}

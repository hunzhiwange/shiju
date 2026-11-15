//! Native WebDAV URLs. Only the configured base is parsed as a URL.

use reqwest::{Client, Url, redirect::Policy};

use super::{manifest, webdav_config::{decode_path_segment, normalize_url}};

/// Build a fixed native artifact URL beneath the configured collection.
/// `data_compat_version` must come from the shared snapshot owner, never a default.
/// Root permits multiple segments; profile must identify exactly one segment.
pub fn build_artifact_url(
    base_url: &str,
    remote_root: &str,
    profile: &str,
    data_compat_version: u32,
    artifact: &str,
) -> Result<Url, &'static str> {
    if data_compat_version == 0 {
        return Err("Missing shared snapshot compatibility version");
    }
    if !matches!(artifact, manifest::CONFIG_ARTIFACT | manifest::SKILLS_ARTIFACT | "manifest.json") {
        return Err("Unsupported native synchronization artifact");
    }
    let base = normalize_url(base_url.trim())?;
    let mut url = Url::parse(&base).map_err(|_| "Invalid WebDAV address")?;
    let root = remote_root.trim().trim_matches('/');
    let root = if root.is_empty() { "shiju-sync" } else { root };
    let profile = profile.trim();
    let profile = if profile.is_empty() { "default" } else { profile };
    // Decode once before pushing: Url then encodes each segment exactly once.
    // Encoded slashes and nested escapes are rejected by the shared validator.
    let profile = decode_path_segment(profile)?;
    let mut path = url.path_segments_mut().map_err(|_| "Invalid WebDAV collection")?;
    path.pop_if_empty();
    for segment in root.split('/') {
        path.push(&decode_path_segment(segment)?);
    }
    path.push(&profile)
        .push(manifest::FORMAT)
        .push(&format!("v{}", manifest::PROTOCOL_VERSION))
        .push(&format!("data-v{data_compat_version}"))
        .push(artifact);
    drop(path);
    Ok(url)
}

/// Transport callers must use this client and attach credentials per request.
/// All redirects remain responses, including same-origin redirects; callers must
/// treat them as failures rather than replaying Authorization to Location.
/// Creating this client does not authorize network access.
pub fn build_client() -> Result<Client, reqwest::Error> {
    Client::builder().redirect(Policy::none()).build()
}

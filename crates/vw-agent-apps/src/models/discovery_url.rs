//! Ordered model discovery candidates for an authorized provider deployment.

use crate::providers::validation;
use reqwest::Url;
use vw_api_types::app_providers::{ProviderProtocol, endpoints::AppProviderEndpointKind};

// Check longer suffixes before the ones they contain.
const COMPAT_SUFFIXES: &[&str] = &[
    "/api/claudecode",
    "/api/anthropic",
    "/apps/anthropic",
    "/api/coding",
    "/claudecode",
    "/anthropic",
    "/step_plan",
    "/coding",
    "/claude",
];

/// The caller resolves the base and any explicitly authorized override endpoints for the
/// same provider under the current scope. Address-book entries alone do not grant access.
pub fn build_models_url_candidates(
    authorized_base_url: &str,
    kind: AppProviderEndpointKind,
    protocol: ProviderProtocol,
    models_url: Option<&str>,
    authorized_overrides: &[Url],
) -> Result<Vec<Url>, String> {
    let base = parse_endpoint(authorized_base_url.trim())?;
    if let Some(raw) = models_url.map(str::trim).filter(|url| !url.is_empty()) {
        let explicit = parse_endpoint(raw)?;
        if explicit.origin() != base.origin() && !authorized_overrides.contains(&explicit) {
            return Err("model discovery override is not authorized".into());
        }
        return Ok(vec![explicit]);
    }

    let mut candidates = Vec::new();
    let mut push_path = |path: String| {
        let mut url = base.clone();
        url.set_path(&path);
        if !candidates.contains(&url) {
            candidates.push(url);
        }
    };
    let path = base.path().trim_end_matches('/');
    if kind == AppProviderEndpointKind::FullUrl {
        let root = path
            .split_once("/v1/")
            .or_else(|| path.rsplit_once('/'))
            .map(|(root, _)| root)
            .ok_or("cannot derive model discovery endpoint from full URL")?;
        push_path(format!("{root}/v1/models"));
    } else if protocol == ProviderProtocol::Gemini {
        let root =
            path.strip_suffix("/v1beta").or_else(|| path.strip_suffix("/v1")).unwrap_or(path);
        push_path(format!("{root}/v1beta/models"));
    } else {
        let segment = path.rsplit('/').next().unwrap_or_default();
        if segment
            .strip_prefix('v')
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
        {
            push_path(format!("{path}/models"));
        }
        if segment != "v1" {
            push_path(format!("{path}/v1/models"));
        }

        if let Some(root) = COMPAT_SUFFIXES.iter().find_map(|&suffix| path.strip_suffix(suffix)) {
            let root = root.trim_end_matches('/');
            push_path(format!("{root}/v1/models"));
            push_path(format!("{root}/models"));
            if path.ends_with("/apps/anthropic") {
                push_path(format!("{root}/compatible-mode/v1/models"));
            }
        }
    }
    Ok(candidates)
}

fn parse_endpoint(raw: &str) -> Result<Url, String> {
    validation::endpoint(raw)?;
    Url::parse(raw).map_err(|_| "invalid application provider endpoint".into())
}

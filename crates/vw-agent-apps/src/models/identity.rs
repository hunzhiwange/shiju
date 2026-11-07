//! Exact model identities use the shared DTOs' field-by-field equality. The
//! normalization here applies only when selecting metadata source candidates.

use crate::providers::validation;
use reqwest::Url;

/// Metadata source endpoint with the origin and deployment path preserved.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct MetadataEndpointKey {
    origin: String,
    path: String,
}

impl MetadataEndpointKey {
    /// Accept a provider endpoint or a scheme-less metadata source address.
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }
        let with_scheme;
        let endpoint = if raw.contains("://") {
            raw
        } else {
            with_scheme = format!("https://{raw}");
            &with_scheme
        };
        validation::endpoint(endpoint).ok()?;
        let url = Url::parse(endpoint).ok()?;
        let origin = url.origin().ascii_serialization();
        let path = metadata_base_path(url.path());
        let path = match (origin.as_str(), path) {
            ("https://opencode.ai", "/zen") => "/inference/openai",
            ("https://opencode.ai", "/zen/go") => "/inference/go/openai",
            _ => path,
        };
        Some(Self { origin, path: path.to_owned() })
    }
}

fn metadata_base_path(mut path: &str) -> &str {
    loop {
        path = path.trim_end_matches('/');
        if let Some(base) = path.strip_suffix("/chat/completions") {
            path = base;
            continue;
        }
        match path.rsplit_once('/') {
            Some((base, segment))
                if matches!(segment, "responses" | "messages") || is_api_version(segment) =>
            {
                path = base;
            }
            _ => return path,
        }
    }
}

fn is_api_version(segment: &str) -> bool {
    let Some(version) = segment.strip_prefix('v') else {
        return false;
    };
    let suffix = version.trim_start_matches(|c: char| c.is_ascii_digit());
    if suffix.len() == version.len() {
        return false;
    }
    suffix.is_empty()
        || suffix
            .strip_prefix("alpha")
            .or_else(|| suffix.strip_prefix("beta"))
            .is_some_and(|number| number.bytes().all(|byte| byte.is_ascii_digit()))
}

/// Normalize a model name for metadata lookup. Keep the original ID for storage and execution.
pub fn metadata_model_key(model_id: &str) -> String {
    let tail = model_id.rsplit('/').next().unwrap_or(model_id);
    let name = tail.split(':').next().unwrap_or(tail);
    let folded = name.trim().replace('@', "-").to_lowercase();
    match folded.strip_suffix("[1m]") {
        Some(base) => base.trim_end().to_owned(),
        None => folded,
    }
}

/// Search rows from a selected metadata source, preferring exact IDs, then
/// case-insensitive IDs, then normalized variants. Ties retain source order.
pub fn find_metadata_model<'a, T: 'a>(
    model_id: &str,
    models: impl IntoIterator<Item = (&'a str, &'a T)>,
) -> Option<&'a T> {
    let folded = model_id.to_lowercase();
    let normalized = metadata_model_key(model_id);
    let mut case_match = None;
    let mut variant_match = None;
    for (id, value) in models {
        if id == model_id {
            return Some(value);
        }
        if case_match.is_none() && id.to_lowercase() == folded {
            case_match = Some(value);
        }
        if case_match.is_none()
            && variant_match.is_none()
            && !normalized.is_empty()
            && metadata_model_key(id) == normalized
        {
            variant_match = Some(value);
        }
    }
    case_match.or(variant_match)
}

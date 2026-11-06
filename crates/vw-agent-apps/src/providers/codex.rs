//! Pure native Codex TOML projection. No files, login policy or transaction publication.
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionError {
    InvalidConfig,
    MissingCredential,
    ProfileOverride,
    Unsupported,
}

/// Contains credentials. Only executor-side login/config transaction owners may consume it.
/// Deliberately has no Serialize implementation.
pub struct NativeProjection(String);
impl NativeProjection {
    pub fn config(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Debug for NativeProjection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeProjection([redacted])")
    }
}

/// `login_on_disk` is supplied by the login owner after planning; this function does not
/// decide whether login material should be preserved. The result is never a client response.
pub fn project(
    live: &str,
    settings: &Value,
    previous: Option<&Value>,
    login_on_disk: bool,
) -> Result<NativeProjection, ProjectionError> {
    // Unlike editor drafts, native inputs must have the actual config/auth shape.
    let config =
        settings.get("config").and_then(Value::as_str).ok_or(ProjectionError::InvalidConfig)?;
    let auth =
        settings.get("auth").and_then(Value::as_object).ok_or(ProjectionError::InvalidConfig)?;
    if live.len() > 1024 * 1024 || config.len() > 128 * 1024 {
        return Err(ProjectionError::InvalidConfig);
    }
    if let Some(key) = auth.get("OPENAI_API_KEY").filter(|v| !v.is_null()) {
        validate_key(key.as_str().ok_or(ProjectionError::InvalidConfig)?)?;
    }
    let live = super::projection::toml(live, "live.config").map_err(map_error)?;
    let result = super::projection::codex_native(live, settings, previous, login_on_disk)
        .map_err(map_error)?;
    // Also cover an embedded table/top-level token selected by the same routing owner.
    if let Some(key) = result
        .get("model_provider")
        .and_then(toml_edit::Item::as_str)
        .and_then(|id| result.get("model_providers")?.get(id))
        .and_then(|table| table.get("experimental_bearer_token"))
    {
        validate_key(key.as_str().ok_or(ProjectionError::InvalidConfig)?)?;
    }
    Ok(NativeProjection(result.to_string()))
}
fn validate_key(key: &str) -> Result<(), ProjectionError> {
    let marker = key.trim();
    if matches!(marker, "CC_SWITCH_PENDING_KEY" | "[redacted]" | "<redacted>")
        || !marker.is_empty() && marker.chars().all(|c| matches!(c, '*' | '•'))
    {
        return Err(ProjectionError::MissingCredential);
    }
    Ok(())
}

fn map_error(error: super::projection::Error) -> ProjectionError {
    use super::projection::Error;
    use vw_api_types::app_providers::preview::PreviewUnavailable;
    match error {
        Error::Keyless => ProjectionError::MissingCredential,
        Error::Unavailable(PreviewUnavailable::Ownership) => ProjectionError::ProfileOverride,
        Error::Unavailable(_) => ProjectionError::Unsupported,
        _ => ProjectionError::InvalidConfig,
    }
}

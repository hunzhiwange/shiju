//! Catalog-only update; native re-projection is owned by application adapters.
use super::{validate_identity, validate_profile};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderWrite, create::ProviderSecretWrite, update::AppProviderUpdate},
};

/// Read only the established profile fields; retain opaque stored metadata on update.
pub fn profile(stored: &serde_json::Value) -> Result<AppProviderWrite, String> {
    let mut fields = serde_json::Map::new();
    for key in ["name", "protocol", "apiKey", "baseUrl", "model"] {
        fields.insert(
            key.into(),
            stored.get(key).cloned().ok_or("invalid stored application provider profile")?,
        );
    }
    if let Some(options) = stored.get("requestOptions") {
        fields.insert("requestOptions".into(), options.clone());
    }
    serde_json::from_value(fields.into())
        .map_err(|_| "invalid stored application provider profile".into())
}

pub fn update(
    app: AppId,
    id: &str,
    stored: &serde_json::Value,
    request: &AppProviderUpdate,
) -> Result<serde_json::Value, String> {
    validate_identity(id)?;
    let mut value = profile(stored)?;
    if let Some(name) = &request.name {
        value.name = name.clone();
    }
    if let Some(protocol) = request.protocol {
        value.protocol = protocol;
    }
    if let Some(base_url) = &request.base_url {
        value.base_url = base_url.clone();
    }
    if let Some(model) = &request.model {
        value.model = model.clone();
    }
    match &request.api_key {
        ProviderSecretWrite::Keep => {}
        ProviderSecretWrite::Replace { value: secret } => {
            let marker = secret.trim();
            if matches!(marker, "[redacted]" | "<redacted>")
                || (!marker.is_empty() && marker.chars().all(|c| c == '*' || c == '•'))
            {
                return Err("credential placeholder cannot replace a secret; use keep".into());
            }
            value.api_key = secret.clone();
        }
        ProviderSecretWrite::Clear => {
            return Err("application provider requires a credential".into());
        }
    }
    if let Some(options) = &request.request_options {
        value.request_options =
            Some(super::validation::merge(app, value.request_options.as_ref(), options)?);
    }
    super::validation::metadata(app, stored.get("meta"))?;
    validate_profile(app, &value)?;
    let fields = serde_json::to_value(value).map_err(|_| "invalid application provider profile")?;
    let mut updated = stored.clone();
    let object = updated.as_object_mut().ok_or("invalid stored application provider profile")?;
    for (key, value) in fields.as_object().ok_or("invalid application provider profile")? {
        object.insert(key.clone(), value.clone());
    }
    Ok(updated)
}

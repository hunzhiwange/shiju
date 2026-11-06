//! Universal catalog rules. Never writes native application configuration.
use super::{validate_identity, validate_profile};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        AppProviderWrite, ProviderProtocol,
        create::ProviderSecretWrite,
        universal::{UniversalConfig, UniversalProviderSummary, UniversalProviderWrite},
    },
};
const CORRUPT: &str = "invalid stored universal provider";
fn validate(config: &UniversalConfig, secret: &str) -> Result<(), String> {
    validate_identity(&config.provider_type)?;
    validate_profile(
        AppId::Claude,
        &AppProviderWrite {
            name: config.name.clone(),
            protocol: ProviderProtocol::Anthropic,
            base_url: config.base_url.clone(),
            api_key: secret.into(),
            model: "universal-record".into(),
        },
    )?;
    let models = &config.models;
    let mut fields: Vec<(&str, usize)> = Vec::new();
    if let Some(claude) = &models.claude {
        for value in [&claude.model, &claude.haiku_model, &claude.sonnet_model, &claude.opus_model]
            .into_iter()
            .flatten()
        {
            fields.push((value, 256));
        }
    }
    if let Some(codex) = &models.codex {
        for value in [&codex.model, &codex.reasoning_effort].into_iter().flatten() {
            fields.push((value, 256));
        }
    }
    if let Some(gemini) = &models.gemini
        && let Some(value) = &gemini.model
    {
        fields.push((value, 256));
    }
    if let Some(value) = &config.website_url {
        let url = reqwest::Url::parse(value).map_err(|_| "invalid universal website URL")?;
        if !matches!(url.scheme(), "https" | "http")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err("invalid universal website URL".into());
        }
        fields.push((value, 2048));
    }
    if let Some(value) = &config.notes {
        fields.push((value, 4096));
    }
    if let Some(value) = &config.icon_color {
        fields.push((value, 64));
    }
    for (value, max) in fields {
        if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
            return Err("invalid universal provider field".into());
        }
    }
    if let Some(icon) = &config.icon
        && !crate::icons::validation::is_valid_custom_icon(icon)
    {
        // Built-in icon names are inert identifiers, never paths, SVG or remote URLs.
        validate_identity(icon)?;
    }
    Ok(())
}
fn secret(stored: &serde_json::Value) -> Result<&str, String> {
    stored
        .get("apiKey")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| CORRUPT.into())
}
pub fn save(
    id: &str,
    stored: Option<&serde_json::Value>,
    request: &UniversalProviderWrite,
) -> Result<serde_json::Value, String> {
    validate_identity(id)?;
    if let Some(stored) = stored {
        summary(id, stored)?;
    }
    let key = match &request.api_key {
        ProviderSecretWrite::Keep => {
            secret(stored.ok_or("new universal provider requires a credential")?)?
        }
        ProviderSecretWrite::Replace { value } => {
            let marker = value.trim();
            if matches!(marker, "[redacted]" | "<redacted>")
                || (!marker.is_empty() && marker.chars().all(|c| c == '*' || c == '•'))
            {
                return Err("credential placeholder cannot replace a secret; use keep".into());
            }
            value
        }
        ProviderSecretWrite::Clear => return Err("universal provider requires a credential".into()),
    };
    validate(&request.config, key)?;
    let mut updated = stored.cloned().unwrap_or_else(|| serde_json::json!({}));
    let object = updated.as_object_mut().ok_or(CORRUPT)?;
    object.insert(
        "config".into(),
        serde_json::to_value(&request.config).map_err(|_| "invalid universal provider field")?,
    );
    object.insert("apiKey".into(), serde_json::Value::String(key.into()));
    Ok(updated)
}
pub fn summary(id: &str, stored: &serde_json::Value) -> Result<UniversalProviderSummary, String> {
    validate_identity(id).map_err(|_| CORRUPT)?;
    let config: UniversalConfig =
        serde_json::from_value(stored.get("config").cloned().ok_or(CORRUPT)?)
            .map_err(|_| CORRUPT)?;
    validate(&config, secret(stored)?).map_err(|_| CORRUPT)?;
    Ok(UniversalProviderSummary { id: id.into(), config, configured: true })
}

//! One catalog projection dispatcher. No host access or native activation.
use super::{universal, update, validate_identity, validate_profile};
use serde_json::{Value, json};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        AppProviderWrite, ProviderProtocol,
        universal_sync::{UniversalSyncError, UniversalSyncRequest},
    },
};
pub fn validate_request(request: &UniversalSyncRequest) -> Result<(), String> {
    if request.apps.is_empty() || request.apps.len() > 14 {
        return Err("invalid universal sync selection".into());
    }
    for (i, app) in request.apps.iter().enumerate() {
        if request.apps[..i].contains(app) {
            return Err("invalid universal sync selection".into());
        }
    }
    Ok(())
}
pub fn child_id(app: AppId, id: &str) -> Result<String, UniversalSyncError> {
    let app = match app {
        AppId::Claude => "claude",
        AppId::Codex => "codex",
        AppId::Gemini => "gemini",
        _ => return Err(UniversalSyncError::UnsupportedApp),
    };
    let child = format!("universal-{app}-{id}");
    validate_identity(&child).map_err(|_| UniversalSyncError::InvalidTargetIdentity)?;
    Ok(child)
}
/// Only generated children of this parent can be replaced. Preserve application-owned
/// metadata, order, unknown configuration keys and nested env/auth entries.
pub fn project(
    id: &str,
    parent: &Value,
    app: AppId,
    existing: Option<&Value>,
) -> Result<Value, UniversalSyncError> {
    use UniversalSyncError::*;
    let config = universal::summary(id, parent).map_err(|_| InvalidProfile)?.config;
    child_id(app, id)?;
    let enabled = match app {
        AppId::Claude => config.apps.claude,
        AppId::Codex => config.apps.codex,
        AppId::Gemini => config.apps.gemini,
        _ => return Err(UnsupportedApp),
    };
    if !enabled {
        return Err(AppDisabled);
    }
    if let Some(existing) = existing {
        if existing.get("universalProviderId").and_then(Value::as_str) != Some(id) {
            return Err(TargetConflict);
        }
        let profile = update::profile(existing).map_err(|_| InvalidProfile)?;
        validate_profile(app, &profile).map_err(|_| InvalidProfile)?;
    }
    let key = parent.get("apiKey").and_then(Value::as_str).ok_or(InvalidProfile)?;
    let mut settings =
        existing.and_then(|v| v.get("settingsConfig")).cloned().unwrap_or_else(|| json!({}));
    if !settings.is_object() {
        return Err(InvalidProfile);
    }
    // These defaults are frozen source conversion behavior, not model discovery.
    let (protocol, model, base_url) = match app {
        AppId::Claude => {
            let models = config.models.claude.as_ref();
            let model =
                models.and_then(|m| m.model.as_deref()).unwrap_or("claude-sonnet-4-20250514");
            let env = object_field(&mut settings, "env")?;
            for (name, value) in [
                ("ANTHROPIC_BASE_URL", config.base_url.as_str()),
                ("ANTHROPIC_AUTH_TOKEN", key),
                ("ANTHROPIC_MODEL", model),
                (
                    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
                    models.and_then(|m| m.haiku_model.as_deref()).unwrap_or(model),
                ),
                (
                    "ANTHROPIC_DEFAULT_SONNET_MODEL",
                    models.and_then(|m| m.sonnet_model.as_deref()).unwrap_or(model),
                ),
                (
                    "ANTHROPIC_DEFAULT_OPUS_MODEL",
                    models.and_then(|m| m.opus_model.as_deref()).unwrap_or(model),
                ),
            ] {
                env.insert(name.into(), value.into());
            }
            (ProviderProtocol::Anthropic, model.to_owned(), config.base_url.clone())
        }
        AppId::Codex => {
            let models = config.models.codex.as_ref();
            let model = models.and_then(|m| m.model.as_deref()).unwrap_or("gpt-4o");
            let effort = models.and_then(|m| m.reasoning_effort.as_deref()).unwrap_or("high");
            let base = config.base_url.trim_end_matches('/');
            let origin = reqwest::Url::parse(base).map_err(|_| InvalidProfile)?.path() == "/";
            let base_url = if origin { format!("{base}/v1") } else { base.into() };
            object_field(&mut settings, "auth")?.insert("OPENAI_API_KEY".into(), key.into());
            let mut document: toml::Table = match settings.get("config") {
                None => toml::Table::new(),
                Some(Value::String(raw)) => raw.parse().map_err(|_| InvalidProfile)?,
                _ => return Err(InvalidProfile),
            };
            document.insert("model_provider".into(), "custom".into());
            document.insert("model".into(), model.into());
            document.insert("model_reasoning_effort".into(), effort.into());
            document.insert("disable_response_storage".into(), true.into());
            let providers = document
                .entry("model_providers")
                .or_insert_with(|| toml::Table::new().into())
                .as_table_mut()
                .ok_or(InvalidProfile)?;
            let custom = providers
                .entry("custom")
                .or_insert_with(|| toml::Table::new().into())
                .as_table_mut()
                .ok_or(InvalidProfile)?;
            for (name, value) in
                [("name", "NewAPI"), ("base_url", base_url.as_str()), ("wire_api", "responses")]
            {
                custom.insert(name.into(), value.into());
            }
            custom.insert("requires_openai_auth".into(), true.into());
            settings["config"] = toml::to_string(&document).map_err(|_| InvalidProfile)?.into();
            (ProviderProtocol::OpenaiResponses, model.to_owned(), base_url)
        }
        AppId::Gemini => {
            let model = config
                .models
                .gemini
                .as_ref()
                .and_then(|m| m.model.as_deref())
                .unwrap_or("gemini-2.5-pro");
            let env = object_field(&mut settings, "env")?;
            for (name, value) in [
                ("GOOGLE_GEMINI_BASE_URL", config.base_url.as_str()),
                ("GEMINI_API_KEY", key),
                ("GEMINI_MODEL", model),
            ] {
                env.insert(name.into(), value.into());
            }
            (ProviderProtocol::Gemini, model.into(), config.base_url.clone())
        }
        _ => return Err(UnsupportedApp),
    };
    let profile = AppProviderWrite {
        request_options: None,
        name: config.name.clone(),
        protocol,
        base_url,
        api_key: key.into(),
        model,
    };
    validate_profile(app, &profile).map_err(|_| InvalidProfile)?;
    let mut updated = existing.cloned().unwrap_or_else(|| json!({}));
    let object = updated.as_object_mut().ok_or(InvalidProfile)?;
    let fields = serde_json::to_value(profile).map_err(|_| InvalidProfile)?;
    for (name, value) in fields.as_object().ok_or(InvalidProfile)? {
        object.insert(name.clone(), value.clone());
    }
    object.insert("settingsConfig".into(), settings);
    object.insert("universalProviderId".into(), id.into());
    for (name, value) in [
        ("websiteUrl", config.website_url),
        ("notes", config.notes),
        ("icon", config.icon),
        ("iconColor", config.icon_color),
    ] {
        object.insert(name.into(), value.map(Value::String).unwrap_or(Value::Null));
    }
    if existing.is_none() {
        object.insert("category".into(), "aggregator".into());
        object.insert("sortIndex".into(), json!(config.sort_index));
    }
    Ok(updated)
}
fn object_field<'a>(
    value: &'a mut Value,
    name: &str,
) -> Result<&'a mut serde_json::Map<String, Value>, UniversalSyncError> {
    value
        .as_object_mut()
        .ok_or(UniversalSyncError::InvalidProfile)?
        .entry(name)
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or(UniversalSyncError::InvalidProfile)
}

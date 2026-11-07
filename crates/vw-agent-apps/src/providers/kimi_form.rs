//! Desktop projection of a single Kimi declaration. Opaque values stay on the executor.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::Value;
use sha2::{Digest, Sha256};
use vw_api_types::app_providers::{create::ProviderSecretWrite, kimi::*};

use super::{WriteError, official, validate};
use crate::providers::basic;

const KEEP: &str = "[保留执行端值]";

pub fn managed(row: &Value) -> bool {
    row.pointer("/meta/authBinding").is_some()
        || row.pointer("/meta/liveConfigManaged").is_some_and(|v| v == true)
}

fn catalog_view(row: &Value) -> Result<KimiFormView, WriteError> {
    let fields = basic::read(row).map_err(|_| WriteError::InvalidResource)?;
    let config = if managed(row) || row.pointer("/kimi/declaration/provider").is_some_and(official)
    {
        None
    } else {
        row.get("kimi")
            .map(|value| {
                let profile: KimiProfile = serde_json::from_value(value.clone())
                    .map_err(|_| WriteError::InvalidResource)?;
                validate(&profile)?;
                let mut public = project(&profile.declaration, &[]);
                let has_saved_key = profile.declaration.pointer("/provider/api_key").is_some();
                if let Some(options) = public.get_mut("provider").and_then(Value::as_object_mut) {
                    options.remove("api_key");
                }
                let declaration =
                    toml::to_string(&public).map_err(|_| WriteError::InvalidResource)?;
                // Compare only the public view. Never fingerprint secret values for the client.
                let baseline = serde_json::to_vec(&(&fields, &public, has_saved_key))
                    .map_err(|_| WriteError::InvalidResource)?;
                let revision = URL_SAFE_NO_PAD.encode(Sha256::digest(baseline));
                Ok(KimiFormConfig { declaration, has_saved_key, revision })
            })
            .transpose()?
    };
    Ok(KimiFormView { fields, config })
}

fn catalog_prepare(
    body: &KimiFormWrite,
    before: Option<&Value>,
) -> Result<(KimiProfile, Value), WriteError> {
    let invalid = WriteError::InvalidResource;
    let mut declaration = toml::from_str::<Value>(&body.declaration).map_err(|_| invalid)?;
    let prior = if let Some(row) = before {
        let config = catalog_view(row)?.config.ok_or(invalid)?;
        if body.revision.as_deref() != Some(&config.revision) {
            return Err(WriteError::Conflict);
        }
        row.pointer("/kimi/declaration")
    } else {
        None
    };
    let mut kept = restore(&mut declaration, prior, &[])?;
    {
        if declaration.pointer("/provider/api_key").is_some() {
            return Err(invalid);
        }
        let old_key = prior.and_then(|node| node.pointer("/provider/api_key"));
        let key = match &body.api_key {
            ProviderSecretWrite::Keep => {
                kept |= old_key.is_some();
                old_key.cloned()
            }
            ProviderSecretWrite::Clear => None,
            ProviderSecretWrite::Replace { value } => {
                let trimmed = value.trim();
                if trimmed.is_empty()
                    || matches!(trimmed, KEEP | "[redacted]" | "<redacted>")
                    || trimmed.chars().all(|c| matches!(c, '*' | '•'))
                    || value.chars().any(char::is_control)
                {
                    return Err(invalid);
                }
                Some(Value::String(value.clone()))
            }
        };
        if let Some(key) = key {
            let options = declaration
                .as_object_mut()
                .ok_or(invalid)?
                .entry("provider")
                .or_insert_with(|| serde_json::json!({}));
            options.as_object_mut().ok_or(invalid)?.insert("api_key".into(), key);
        }
    }
    if kept && prior.is_some_and(|previous| routing_changed(previous, &declaration)) {
        return Err(invalid);
    }
    let profile = KimiProfile { name: body.fields.name.clone(), declaration };
    validate(&profile)?;
    if official(&profile.declaration["provider"]) {
        return Err(invalid);
    }
    let metadata = basic::apply(
        vw_api_types::agent_apps::AppId::Kimi,
        before.unwrap_or(&serde_json::json!({})),
        &body.fields,
        &ProviderSecretWrite::Keep,
    )
    .map_err(|_| invalid)?;
    Ok((profile, metadata))
}

fn routing_changed(before: &Value, after: &Value) -> bool {
    [
        "/provider/type",
        "/provider/base_url",
        "/provider/api_key_env",
        "/provider/env",
        "/provider/oauth",
    ]
    .into_iter()
    .any(|path| before.pointer(path) != after.pointer(path))
}

fn public(path: &[&str], value: &Value) -> bool {
    match path {
        [] | ["provider" | "models"] | ["models", _] => value.is_object(),
        ["provider", "type" | "api_key_env"]
        | ["models", _, "model" | "provider" | "display_name"] => value.is_string(),
        ["models", _, "max_context_size"] => value.is_number(),
        ["models", _, "capabilities"] => {
            value.as_array().is_some_and(|a| a.iter().all(Value::is_string))
        }
        ["provider", "base_url"] => {
            value.as_str().and_then(|text| reqwest::Url::parse(text).ok()).is_some_and(|url| {
                matches!(url.scheme(), "http" | "https")
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none()
            })
        }
        _ => false,
    }
}

fn project(value: &Value, path: &[&str]) -> Value {
    if !public(path, value) {
        return Value::String(KEEP.into());
    }
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let mut child = path.to_vec();
                    child.push(key);
                    (key.clone(), project(value, &child))
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn restore(value: &mut Value, before: Option<&Value>, path: &[&str]) -> Result<bool, WriteError> {
    if value.as_str() == Some(KEEP) {
        let original =
            before.filter(|original| !public(path, original)).ok_or(WriteError::InvalidResource)?;
        *value = original.clone();
        return Ok(true);
    }
    let mut kept = false;
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                let mut child = path.to_vec();
                child.push(key);
                kept |= restore(value, before.and_then(|node| node.get(key)), &child)?;
            }
        }
        // Opaque arrays are projected as a unit and cannot contain moved markers.
        Value::Array(values) => {
            for value in values {
                restore(value, None, path)?;
            }
        }
        _ => {}
    }
    Ok(kept)
}

pub fn view(row: Option<&Value>, member: Option<&KimiProfile>) -> Result<KimiFormView, WriteError> {
    let fallback = member.map(|profile| serde_json::json!({"name":profile.name,"kimi":profile}));
    let mut view = catalog_view(row.or(fallback.as_ref()).ok_or(WriteError::InvalidResource)?)?;
    if let Some(config) = view.config.as_mut() {
        let native = fallback.as_ref().map(catalog_view).transpose()?;
        let baseline = serde_json::to_vec(&(&config.revision, native, row.is_some()))
            .map_err(|_| WriteError::InvalidResource)?;
        config.revision = URL_SAFE_NO_PAD.encode(Sha256::digest(baseline));
    }
    Ok(view)
}
pub fn prepare(
    body: &KimiFormWrite,
    row: Option<&Value>,
    member: Option<&KimiProfile>,
) -> Result<(KimiProfile, Value), WriteError> {
    let fallback = member.map(|profile| serde_json::json!({"name":profile.name,"kimi":profile}));
    let mut body = body.clone();
    if row.is_some() || member.is_some() {
        let config = view(row, member)?.config.ok_or(WriteError::InvalidResource)?;
        if body.revision.as_deref() != Some(config.revision.as_str()) {
            return Err(WriteError::Conflict);
        }
    } else if body.revision.is_some() {
        return Err(WriteError::Conflict);
    }
    let mut previous = row.or(fallback.as_ref()).cloned();
    if body.activation == KimiActivation::IfMember
        && let (Some(previous), Some(member)) = (previous.as_mut(), member)
    {
        previous["kimi"] = serde_json::to_value(member).map_err(|_| WriteError::InvalidResource)?;
    }
    body.revision = previous
        .as_ref()
        .map(catalog_view)
        .transpose()?
        .and_then(|view| view.config.map(|config| config.revision));
    catalog_prepare(&body, previous.as_ref())
}

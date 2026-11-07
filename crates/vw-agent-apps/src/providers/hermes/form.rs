//! Public projection and write preparation for a single Hermes provider.
use super::{WriteError, invalid, normalize_declaration, validate};
use crate::providers::basic;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{create::ProviderSecretWrite, hermes::*},
};

const FIELDS: &[&str] = &["base_url", "api_mode", "model", "models", "rate_limit_delay"];
const MODEL_FIELDS: &[&str] = &["name", "context_length", "supports_vision"];

pub fn managed(row: &Value) -> bool {
    row.pointer("/meta/authBinding").is_some()
        || row.pointer("/meta/liveConfigManaged").is_some_and(|value| value == true)
        || row
            .pointer("/hermes/declaration/_cc_source")
            .is_some_and(|value| value == "providers_dict")
}

pub fn view(row: &Value) -> Result<HermesFormView, WriteError> {
    let invalid = WriteError::InvalidResource;
    let fields = basic::read(row).map_err(|_| invalid)?;
    let config = if managed(row) {
        None
    } else {
        row.pointer("/hermes/declaration")
            .map(|stored| {
                let normalized = normalize_declaration(stored)?;
                let mut public = select(normalized.as_object().ok_or(invalid)?, FIELDS);
                public.retain(|key, value| match key.as_str() {
                    "rate_limit_delay" => value.is_number(),
                    _ => true,
                });
                if let Some(models) = public.get_mut("models").and_then(Value::as_object_mut) {
                    for model in models.values_mut() {
                        let mut fields = select(model.as_object().ok_or(invalid)?, MODEL_FIELDS);
                        fields.retain(|key, value| match key.as_str() {
                            "name" => value.is_string(),
                            "context_length" => value.as_u64().is_some_and(|v| v > 0),
                            "supports_vision" => value.is_boolean(),
                            _ => false,
                        });
                        *model = Value::Object(fields);
                    }
                }
                // Legacy URLs with embedded authentication must never enter a desktop draft.
                if public.get("base_url").is_some_and(|value| !public_url(value)) {
                    public.remove("base_url");
                }
                let has_saved_key = has_api_key(&normalized);
                let declaration = serde_json::to_string_pretty(&public).map_err(|_| invalid)?;
                let baseline =
                    serde_json::to_vec(&(&fields, &public, has_saved_key)).map_err(|_| invalid)?;
                Ok(HermesFormConfig {
                    declaration,
                    has_saved_key,
                    revision: URL_SAFE_NO_PAD.encode(Sha256::digest(baseline)),
                })
            })
            .transpose()?
    };
    Ok(HermesFormView { fields, config })
}

fn select(fields: &Map<String, Value>, keys: &[&str]) -> Map<String, Value> {
    keys.iter()
        .filter_map(|&key| fields.get(key).map(|value| (key.to_owned(), value.clone())))
        .collect()
}

fn public_url(value: &Value) -> bool {
    value.as_str().and_then(|s| reqwest::Url::parse(s).ok()).is_some_and(|url| {
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
    })
}

fn has_api_key(value: &Value) -> bool {
    value.get("api_key").and_then(Value::as_str).is_some_and(|key| !key.is_empty())
}

pub fn check_live_credentials(
    body: &HermesFormWrite,
    previous: &Value,
    next: &Value,
) -> Result<(), WriteError> {
    if (previous.get("key_env").is_some()
        || (matches!(body.api_key, ProviderSecretWrite::Keep)
            && (has_api_key(previous) || has_api_key(next))))
        && ["base_url", "api_mode"].iter().any(|key| previous.get(key) != next.get(key))
    {
        return invalid();
    }
    Ok(())
}

pub fn prepare(
    body: &HermesFormWrite,
    before: Option<&Value>,
) -> Result<(HermesProfile, Value), WriteError> {
    let invalid = WriteError::InvalidResource;
    let declaration: Value = serde_json::from_str(&body.declaration).map_err(|_| invalid)?;
    let fields = declaration.as_object().ok_or(invalid)?;
    if fields.keys().any(|key| !FIELDS.contains(&key.as_str()))
        || fields.get("base_url").is_some_and(|value| !public_url(value))
        || fields.get("api_mode").is_some_and(|value| {
            !value
                .as_str()
                .is_some_and(|mode| !mode.trim().is_empty() && !mode.chars().any(char::is_control))
        })
        || fields
            .get("rate_limit_delay")
            .is_some_and(|value| !value.as_f64().is_some_and(|v| v >= 0.0))
    {
        return Err(invalid);
    }
    let prior = if let Some(row) = before {
        let config = view(row)?.config.ok_or(invalid)?;
        if body.revision.as_deref() != Some(&config.revision) {
            return Err(WriteError::Conflict);
        }
        normalize_declaration(row.pointer("/hermes/declaration").ok_or(invalid)?)?
    } else {
        json!({})
    };
    let mut submitted = normalize_declaration(&declaration)?;
    if let Some(models) = submitted.get_mut("models").and_then(Value::as_object_mut) {
        for (id, model) in models {
            let values = model.as_object().ok_or(invalid)?;
            if values.keys().any(|key| !MODEL_FIELDS.contains(&key.as_str()))
                || values.get("name").is_some_and(|v| !v.is_string())
                || values.get("context_length").is_some_and(|v| !v.as_u64().is_some_and(|n| n > 0))
                || values.get("supports_vision").is_some_and(|v| !v.is_boolean())
            {
                return Err(invalid);
            }
            let mut stored = prior
                .get("models")
                .and_then(|v| v.get(id))
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            stored.retain(|key, _| !MODEL_FIELDS.contains(&key.as_str()));
            stored.extend(values.clone());
            *model = Value::Object(stored);
        }
    }
    let mut next = prior.as_object().cloned().ok_or(invalid)?;
    next.extend(submitted.as_object().cloned().ok_or(invalid)?);
    match &body.api_key {
        ProviderSecretWrite::Keep => {}
        ProviderSecretWrite::Clear => {
            // The native patch treats omission as keep.
            next.insert("api_key".into(), json!(""));
        }
        ProviderSecretWrite::Replace { value } => {
            let marker = value.trim();
            if marker.is_empty()
                || marker.eq_ignore_ascii_case("[redacted]")
                || marker.eq_ignore_ascii_case("<redacted>")
                || marker.chars().all(|c| matches!(c, '*' | '•'))
                || value.chars().any(char::is_control)
            {
                return Err(invalid);
            }
            next.insert("api_key".into(), json!(value));
        }
    }
    let next = normalize_declaration(&Value::Object(next))?;
    check_live_credentials(body, &prior, &next)?;
    let profile = HermesProfile { name: body.fields.name.clone(), declaration: next };
    validate(&profile)?;
    let metadata = basic::apply(
        AppId::Hermes,
        before.unwrap_or(&json!({})),
        &body.fields,
        &ProviderSecretWrite::Keep,
    )
    .map_err(|_| invalid)?;
    Ok((profile, metadata))
}

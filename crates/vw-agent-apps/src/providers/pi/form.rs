//! Public draft and secret-preserving edits for one Pi provider node.
use super::{WriteError, invalid, validate};
use crate::providers::basic;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{create::ProviderSecretWrite, pi::*},
};

const ROOT: &[&str] = &["name", "baseUrl", "api", "models"];
const MODEL: &[&str] =
    &["id", "name", "reasoning", "input", "contextWindow", "maxTokens", "cost", "thinkingLevelMap"];
const EMPTY: Value = Value::Null;

pub fn managed(row: &Value) -> bool {
    row.pointer("/meta/authBinding").is_some()
        || row.pointer("/meta/liveConfigManaged").is_some_and(|v| v == true)
}

fn selected(node: &Map<String, Value>, fields: &[&str]) -> Map<String, Value> {
    node.iter()
        .filter(|(key, _)| fields.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn transport_url(value: &Value) -> bool {
    value
        .as_str()
        .filter(|v| !v.chars().any(char::is_control))
        .and_then(|v| reqwest::Url::parse(v).ok())
        .is_some_and(|url| {
            matches!(url.scheme(), "http" | "https")
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none()
        })
}

fn text(value: &Value) -> bool {
    value.as_str().is_some_and(|v| !v.trim().is_empty() && !v.chars().any(char::is_control))
}

fn model_field(key: &str, value: &Value) -> bool {
    match key {
        "id" | "name" => text(value),
        "reasoning" => value.is_boolean(),
        "input" => value.as_array().is_some_and(|items| {
            !items.is_empty() && items.iter().all(|v| v == "text" || v == "image")
        }),
        "contextWindow" | "maxTokens" => value.as_u64().is_some_and(|v| v > 0),
        "cost" => value.as_object().is_some_and(|cost| {
            cost.iter().all(|(key, v)| {
                ["input", "output", "cacheRead", "cacheWrite"].contains(&key.as_str())
                    && v.as_f64().is_some_and(|v| v >= 0.0)
            })
        }),
        "thinkingLevelMap" => value.as_object().is_some_and(|levels| {
            levels.iter().all(|(key, v)| {
                ["off", "minimal", "low", "medium", "high", "xhigh", "max"].contains(&key.as_str())
                    && (v.is_null() || text(v))
            })
        }),
        _ => false,
    }
}

fn root_field(key: &str, value: &Value) -> bool {
    match key {
        "name" | "api" => text(value),
        "baseUrl" => transport_url(value),
        "models" => value.as_array().is_some_and(|models| {
            let mut ids = BTreeSet::new();
            models.iter().all(|model| {
                model.as_object().is_some_and(|model| {
                    model
                        .get("id")
                        .filter(|id| text(id))
                        .and_then(Value::as_str)
                        .is_some_and(|id| ids.insert(id))
                        && model.iter().all(|(key, value)| model_field(key, value))
                })
            })
        }),
        _ => false,
    }
}

fn public(node: &Value) -> Option<Value> {
    let mut fields = selected(node.as_object()?, ROOT);
    if let Some(models) = fields.get_mut("models") {
        for model in models.as_array_mut()? {
            *model = Value::Object(selected(model.as_object()?, MODEL));
        }
    }
    fields.iter().all(|(key, value)| root_field(key, value)).then_some(Value::Object(fields))
}

fn has_key(node: &Value) -> bool {
    node.get("apiKey").is_some_and(|value| !value.is_null() && value != "")
}

pub fn view(row: Option<&Value>, member: Option<&PiProfile>) -> Result<PiFormView, WriteError> {
    let native = member.map(|profile| &profile.declaration);
    let catalog = row.and_then(|row| row.pointer("/pi/declaration"));
    let native_metadata = member.map(|profile| json!({"name":profile.name}));
    let stored = row.or(native_metadata.as_ref()).ok_or(WriteError::InvalidResource)?;
    let fields = basic::read(stored).map_err(|_| WriteError::InvalidResource)?;
    let config = if row.is_some_and(managed) {
        None
    } else {
        (|| {
            let source = catalog.or(native)?;
            let node = public(source)?;
            // A malformed or credential-bearing legacy transport URL cannot be safely edited.
            let live = native.map(public);
            if matches!(live, Some(None)) {
                return None;
            }
            let has_saved_key = has_key(source) || native.is_some_and(has_key);
            let baseline =
                serde_json::to_vec(&(&fields, &node, live, catalog.is_some(), has_saved_key))
                    .ok()?;
            Some(PiFormConfig {
                declaration: serde_json::to_string_pretty(&node).ok()?,
                has_saved_key,
                revision: URL_SAFE_NO_PAD.encode(Sha256::digest(baseline)),
            })
        })()
    };
    Ok(PiFormView { fields, config })
}

pub fn prepare(
    body: &PiFormWrite,
    row: Option<&Value>,
    member: Option<&PiProfile>,
) -> Result<(PiProfile, Value), WriteError> {
    let bad = WriteError::InvalidResource;
    let submitted: Map<String, Value> = serde_json::from_str(&body.declaration).map_err(|_| bad)?;
    if !submitted.iter().all(|(key, value)| root_field(key, value)) {
        return invalid();
    }
    if body.revision.is_none() && !complete_new_node(&submitted) {
        return invalid();
    }
    if row.is_some() || member.is_some() {
        let config = view(row, member)?.config.ok_or(bad)?;
        if body.revision.as_deref() != Some(config.revision.as_str()) {
            return Err(WriteError::Conflict);
        }
    } else if body.revision.is_some() {
        return Err(WriteError::Conflict);
    }

    let catalog = row.and_then(|row| row.pointer("/pi/declaration"));
    let native = member.map(|profile| &profile.declaration);
    let previous = if body.activation == PiActivation::IfMember {
        native.or(catalog)
    } else {
        catalog.or(native)
    }
    .unwrap_or(&EMPTY);
    let mut next = merge_public_fields(submitted, previous)?;
    match &body.api_key {
        ProviderSecretWrite::Keep => {}
        ProviderSecretWrite::Clear => {
            next.remove("apiKey");
        }
        ProviderSecretWrite::Replace { value } => {
            let marker = value.trim();
            // Pi can execute keys prefixed with '!'. The editor only accepts credentials.
            if marker.is_empty()
                || marker.starts_with('!')
                || marker.eq_ignore_ascii_case("[redacted]")
                || marker.eq_ignore_ascii_case("<redacted>")
                || marker.chars().all(|ch| matches!(ch, '*' | '•'))
                || value.chars().any(char::is_control)
            {
                return invalid();
            }
            next.insert("apiKey".into(), json!(value));
        }
    }
    check_transport(previous, &next, matches!(body.api_key, ProviderSecretWrite::Keep))?;
    let profile = PiProfile { name: body.fields.name.clone(), declaration: Value::Object(next) };
    validate(&profile)?;
    let metadata = basic::apply(
        AppId::Pi,
        row.unwrap_or(&json!({})),
        &body.fields,
        &ProviderSecretWrite::Keep,
    )
    .map_err(|_| bad)?;
    Ok((profile, metadata))
}

/// Directory edits can change public model fields. Native or catalog credentials
/// remain under their existing owner.
pub(super) fn directory_declaration(
    profile: &PiProfile,
    previous: Option<&Value>,
) -> Result<Value, WriteError> {
    validate(profile)?;
    let submitted = profile.declaration.as_object().ok_or(WriteError::InvalidResource)?;
    if !submitted.iter().all(|(key, value)| root_field(key, value))
        || (previous.is_none() && !complete_new_node(submitted))
    {
        return invalid();
    }
    let previous = previous.unwrap_or(&EMPTY);
    let next = merge_public_fields(submitted.clone(), previous)?;
    check_transport(previous, &next, true)?;
    Ok(Value::Object(next))
}

fn merge_public_fields(
    submitted: Map<String, Value>,
    previous: &Value,
) -> Result<Map<String, Value>, WriteError> {
    let bad = WriteError::InvalidResource;
    let mut next = previous.as_object().cloned().unwrap_or_default();
    next.retain(|key, _| !ROOT.contains(&key.as_str()));
    next.extend(submitted);
    if let Some(models) = next.get_mut("models").and_then(Value::as_array_mut) {
        let previous_models: BTreeMap<_, _> = previous
            .get("models")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|model| Some((model.get("id")?.as_str()?, model)))
            .collect();
        for model in models {
            let id = model["id"].as_str().ok_or(bad)?;
            if let Some(previous_model) = previous_models.get(id) {
                let fields = model.as_object().ok_or(bad)?;
                let mut merged = previous_model.as_object().cloned().ok_or(bad)?;
                merged.retain(|key, _| !MODEL.contains(&key.as_str()));
                merged.extend(fields.clone());
                *model = Value::Object(merged);
            }
        }
    }
    Ok(next)
}

fn check_transport(
    previous: &Value,
    next: &Map<String, Value>,
    keeping_key: bool,
) -> Result<(), WriteError> {
    if ["baseUrl", "api"].iter().any(|key| previous.get(key) != next.get(*key))
        && (next.contains_key("headers")
            || (keeping_key && has_key(previous))
            || next
                .get("models")
                .and_then(Value::as_array)
                .is_some_and(|models| models.iter().any(protected)))
    {
        return invalid();
    }
    Ok(())
}

/// Explicit legacy overrides may inherit missing fields. New nodes need a usable transport
/// and model definitions; the form does not guess model limits or capabilities.
fn complete_new_node(node: &Map<String, Value>) -> bool {
    node.contains_key("baseUrl")
        && node.contains_key("api")
        && node.get("models").and_then(Value::as_array).is_some_and(|models| {
            !models.is_empty()
                && models.iter().all(|model| {
                    ["name", "contextWindow", "maxTokens"]
                        .iter()
                        .all(|key| model.get(*key).is_some())
                })
        })
}

fn protected(node: &Value) -> bool {
    has_key(node) || node.get("headers").is_some()
}

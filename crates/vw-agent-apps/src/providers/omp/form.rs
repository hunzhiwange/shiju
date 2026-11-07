//! Public YAML and credential-preserving edits for one OMP provider node.
use super::{WriteError, invalid, validate};
use crate::providers::basic;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{create::ProviderSecretWrite, omp::*},
};

const ROOT: &[&str] = &["baseUrl", "api", "auth", "models", "discovery"];
const MODEL: &[&str] =
    &["id", "name", "baseUrl", "api", "reasoning", "input", "contextWindow", "maxTokens", "cost"];
const DISCOVERY: &[&str] = &["type", "injectV1", "timeoutMs"];
const TRANSPORT: &[&str] = &["baseUrl", "api"];
const BAD: WriteError = WriteError::InvalidResource;

pub fn managed(row: &Value) -> bool {
    row.pointer("/meta/authBinding").is_some()
        || row.pointer("/meta/liveConfigManaged").is_some_and(|value| value == true)
}

fn select(node: &Map<String, Value>, fields: &[&str]) -> Map<String, Value> {
    node.iter()
        .filter(|(key, _)| fields.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn text(value: &Value) -> bool {
    value.as_str().is_some_and(|s| !s.trim().is_empty() && !s.chars().any(char::is_control))
}

fn safe_url(value: &Value) -> bool {
    text(value)
        && value.as_str().and_then(|s| reqwest::Url::parse(s).ok()).is_some_and(|url| {
            matches!(url.scheme(), "http" | "https")
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none()
        })
}

fn public_fields(node: &Map<String, Value>) -> bool {
    node.get("baseUrl").is_none_or(safe_url) && node.get("api").is_none_or(text)
}

fn editable(node: &Map<String, Value>) -> bool {
    if !node.keys().all(|key| ROOT.contains(&key.as_str()))
        || !public_fields(node)
        || node.get("auth").is_some_and(|value| !matches!(value.as_str(), Some("none" | "apiKey")))
        || node.get("discovery").is_some_and(|value| {
            !value.as_object().is_some_and(|discovery| {
                discovery.iter().all(|(key, value)| match key.as_str() {
                    "type" => text(value),
                    "injectV1" => value.is_boolean(),
                    "timeoutMs" => value.is_number(),
                    _ => false,
                })
            })
        })
    {
        return false;
    }
    let mut ids = BTreeSet::new();
    node.get("models").is_none_or(|value| {
        value.as_array().is_some_and(|models| {
            models.iter().all(|value| {
                value.as_object().is_some_and(|model| {
                    public_fields(model)
                        && model
                            .get("id")
                            .filter(|id| text(id))
                            .and_then(Value::as_str)
                            .is_some_and(|id| ids.insert(id))
                        && model.iter().all(|(key, value)| match key.as_str() {
                            "id" | "baseUrl" | "api" => true,
                            "name" => text(value),
                            "reasoning" => value.is_boolean(),
                            "input" => value.as_array().is_some_and(|v| v.iter().all(text)),
                            "contextWindow" | "maxTokens" => value.is_number(),
                            "cost" => value.as_object().is_some_and(|v| {
                                v.iter().all(|(k, v)| {
                                    ["input", "output", "cacheRead", "cacheWrite"]
                                        .contains(&k.as_str())
                                        && v.is_number()
                                })
                            }),
                            _ => false,
                        })
                })
            })
        })
    })
}

fn public(node: &Value) -> Option<Map<String, Value>> {
    let mut fields = select(node.as_object()?, ROOT);
    if let Some(models) = fields.get_mut("models") {
        for model in models.as_array_mut()? {
            *model = Value::Object(select(model.as_object()?, MODEL));
        }
    }
    if let Some(discovery) = fields.get_mut("discovery") {
        *discovery = Value::Object(select(discovery.as_object()?, DISCOVERY));
    }
    editable(&fields).then_some(fields)
}

fn catalog_node(row: Option<&Value>) -> Option<&Value> {
    row.and_then(|row| row.pointer("/omp/declaration"))
}

fn has_key(node: &Value) -> bool {
    node.get("apiKey").is_some()
}

pub fn view(row: Option<&Value>, member: Option<&OmpProfile>) -> Result<OmpFormView, WriteError> {
    let native = member.map(|profile| &profile.declaration);
    let catalog = catalog_node(row);
    let native_metadata = member.map(|profile| json!({"name":profile.name}));
    let stored = row.or(native_metadata.as_ref()).ok_or(BAD)?;
    let fields = basic::read(stored).map_err(|_| BAD)?;
    let config = if row.is_some_and(managed) {
        None
    } else {
        (|| {
            let source = catalog.or(native)?;
            let node = public(source)?;
            let live = native.map(public);
            if matches!(live, Some(None)) {
                return None;
            }
            let has_saved_key = has_key(source) || native.is_some_and(has_key);
            let baseline =
                serde_json::to_vec(&(&fields, &node, live, catalog.is_some(), has_saved_key))
                    .ok()?;
            Some(OmpFormConfig {
                declaration: serde_yaml::to_string(&node).ok()?,
                has_saved_key,
                revision: URL_SAFE_NO_PAD.encode(Sha256::digest(baseline)),
            })
        })()
    };
    Ok(OmpFormView { fields, config })
}

fn merge(
    previous: Option<&Value>,
    submitted: Map<String, Value>,
    fields: &[&str],
) -> Map<String, Value> {
    let mut next = previous.and_then(Value::as_object).cloned().unwrap_or_default();
    next.retain(|key, _| !fields.contains(&key.as_str()));
    next.extend(submitted);
    next
}

fn model_map(node: Option<&Value>) -> BTreeMap<&str, &Value> {
    node.and_then(|node| node.get("models"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|model| Some((model.get("id")?.as_str()?, model)))
        .collect()
}

fn transport_changed(previous: Option<&Value>, next: &Value, keys: &[&str]) -> bool {
    keys.iter().any(|key| previous.and_then(|node| node.get(*key)) != next.get(*key))
}

pub fn prepare(
    body: &OmpFormWrite,
    row: Option<&Value>,
    member: Option<&OmpProfile>,
) -> Result<(OmpProfile, Value), WriteError> {
    let submitted: Map<String, Value> = serde_yaml::from_str(&body.declaration).map_err(|_| BAD)?;
    if !editable(&submitted) {
        return invalid();
    }
    if row.is_some() || member.is_some() {
        let config = view(row, member)?.config.ok_or(BAD)?;
        if body.revision.as_deref() != Some(config.revision.as_str()) {
            return Err(WriteError::Conflict);
        }
    } else if body.revision.is_some() {
        return Err(WriteError::Conflict);
    }

    let catalog = catalog_node(row);
    let native = member.map(|profile| &profile.declaration);
    let previous = if body.activation == OmpActivation::IfMember {
        native.or(catalog)
    } else {
        catalog.or(native)
    };
    let previous_models = model_map(previous);
    let mut next = merge(previous, submitted, ROOT);
    if let Some(models) = next.get_mut("models").and_then(Value::as_array_mut) {
        for model in models {
            let id = model["id"].as_str().ok_or(BAD)?;
            let fields = model.as_object().ok_or(BAD)?;
            *model = Value::Object(merge(previous_models.get(id).copied(), fields.clone(), MODEL));
        }
    }
    if let Some(old) = previous.and_then(|node| node.get("discovery"))
        && let Some(new) = next.get_mut("discovery")
    {
        let fields = new.as_object().ok_or(BAD)?;
        if old["type"] == new["type"] {
            *new = Value::Object(merge(Some(old), fields.clone(), DISCOVERY));
        } else if old
            .as_object()
            .is_some_and(|v| v.keys().any(|k| !DISCOVERY.contains(&k.as_str())))
        {
            return invalid();
        }
    }
    match &body.api_key {
        ProviderSecretWrite::Keep => {}
        ProviderSecretWrite::Clear => {
            next.remove("apiKey");
        }
        ProviderSecretWrite::Replace { value } => {
            let marker = value.trim();
            if marker.is_empty()
                || value.chars().any(char::is_control)
                || marker.starts_with('!')
                || marker.eq_ignore_ascii_case("[redacted]")
                || marker.eq_ignore_ascii_case("<redacted>")
                || marker.chars().all(|ch| matches!(ch, '*' | '•'))
            {
                return invalid();
            }
            next.insert("apiKey".into(), json!(value));
        }
    }

    let next = Value::Object(next);
    let route_changed =
        transport_changed(previous, &next, &["baseUrl", "api", "auth", "discovery"]);
    let retained_root_auth = next.get("headers").is_some()
        || (matches!(body.api_key, ProviderSecretWrite::Keep) && next.get("apiKey").is_some());
    if route_changed && retained_root_auth {
        return invalid();
    }
    for (id, model) in model_map(Some(&next)) {
        if (route_changed || transport_changed(previous_models.get(id).copied(), model, TRANSPORT))
            && (retained_root_auth || has_key(model) || model.get("headers").is_some())
        {
            return invalid();
        }
    }
    let profile = OmpProfile { name: body.fields.name.clone(), declaration: next };
    validate(&profile)?;
    let metadata = basic::apply(
        AppId::Omp,
        row.unwrap_or(&json!({})),
        &body.fields,
        &ProviderSecretWrite::Keep,
    )
    .map_err(|_| BAD)?;
    Ok((profile, metadata))
}

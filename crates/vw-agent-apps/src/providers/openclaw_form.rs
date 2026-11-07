//! Public editor projection for a single OpenClaw provider declaration.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{create::ProviderSecretWrite, openclaw::*},
};

use super::{WriteError, object_value, validate};
use crate::providers::basic;

const KEEP: &str = "[保留执行端值]";
const BROWSER_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:148.0) Gecko/20100101 Firefox/148.0";

pub fn managed(row: &Value) -> bool {
    row.pointer("/meta/authBinding").is_some()
        || row.pointer("/meta/liveConfigManaged").is_some_and(|value| value == true)
}

pub fn view(row: &Value) -> Result<OpenClawFormView, WriteError> {
    let invalid = WriteError::InvalidResource;
    let fields = basic::read(row).map_err(|_| invalid)?;
    let config = if managed(row) {
        None
    } else {
        row.get("openclaw")
            .map(|value| {
                let profile: OpenClawProfile =
                    serde_json::from_value(value.clone()).map_err(|_| invalid)?;
                validate(&profile)?;
                let mut public = project(&profile.declaration, &[]);
                public.as_object_mut().ok_or(invalid)?.remove("apiKey");
                public.as_object_mut().ok_or(invalid)?.remove("headers");
                let has_saved_key = profile.declaration.get("apiKey").is_some();
                let user_agent = headers(&profile.declaration)
                    .any(|(name, _)| name.eq_ignore_ascii_case("user-agent"));
                let declaration = serde_json::to_string_pretty(&public).map_err(|_| invalid)?;
                let baseline = serde_json::to_vec(&(&fields, &public, has_saved_key, user_agent))
                    .map_err(|_| invalid)?;
                Ok(OpenClawFormConfig {
                    declaration,
                    has_saved_key,
                    user_agent,
                    revision: URL_SAFE_NO_PAD.encode(Sha256::digest(baseline)),
                })
            })
            .transpose()?
    };
    Ok(OpenClawFormView { fields, config })
}

pub fn prepare(
    body: &OpenClawFormWrite,
    before: Option<&Value>,
) -> Result<(OpenClawProfile, Value), WriteError> {
    let invalid = WriteError::InvalidResource;
    let mut declaration = object_value(body.declaration.as_bytes()).map_err(|_| invalid)?;
    if declaration.get("apiKey").is_some() || declaration.get("headers").is_some() {
        return Err(invalid);
    }
    let prior = if let Some(row) = before {
        let config = view(row)?.config.ok_or(invalid)?;
        if body.revision.as_deref() != Some(&config.revision) {
            return Err(WriteError::Conflict);
        }
        row.pointer("/openclaw/declaration")
    } else {
        None
    };
    let mut kept = restore(&mut declaration, prior, &[])?;
    let key = match &body.api_key {
        ProviderSecretWrite::Keep => {
            let key = prior.and_then(|value| value.get("apiKey"));
            kept |= key.is_some();
            key.cloned()
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
    let mut headers = headers(prior.unwrap_or(&Value::Null))
        .filter(|(name, _)| body.user_agent || !name.eq_ignore_ascii_case("user-agent"))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<serde_json::Map<_, _>>();
    kept |= headers.keys().any(|name| !name.eq_ignore_ascii_case("user-agent"));
    if kept
        && prior.is_some_and(|previous| {
            ["baseUrl", "api"].iter().any(|key| previous.get(key) != declaration.get(key))
        })
    {
        return Err(invalid);
    }
    if body.user_agent && !headers.keys().any(|name| name.eq_ignore_ascii_case("user-agent")) {
        headers.insert("User-Agent".into(), BROWSER_USER_AGENT.into());
    }
    let node = declaration.as_object_mut().ok_or(invalid)?;
    if let Some(key) = key {
        node.insert("apiKey".into(), key);
    }
    if !headers.is_empty() {
        node.insert("headers".into(), Value::Object(headers));
    }
    let profile = OpenClawProfile { name: body.fields.name.clone(), declaration };
    validate(&profile)?;
    let metadata = basic::apply(
        AppId::OpenClaw,
        before.unwrap_or(&json!({})),
        &body.fields,
        &ProviderSecretWrite::Keep,
    )
    .map_err(|_| invalid)?;
    Ok((profile, metadata))
}

fn headers(node: &Value) -> impl Iterator<Item = (&String, &Value)> {
    node.get("headers").and_then(Value::as_object).into_iter().flatten()
}

fn public(path: &[&str], value: &Value) -> bool {
    match path {
        [] | ["models", "*"] | ["models", "*", "cost"] => value.is_object(),
        ["baseUrl"] => {
            value.as_str().and_then(|text| reqwest::Url::parse(text).ok()).is_some_and(|url| {
                matches!(url.scheme(), "http" | "https")
                    && url.host_str().is_some()
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none()
            })
        }
        ["api"] | ["models", "*", "id" | "name"] => value.is_string(),
        ["models"] => value.is_array(),
        ["models", "*", "input"] => {
            value.as_array().is_some_and(|types| types.iter().all(Value::is_string))
        }
        ["models", "*", "reasoning"] => value.is_boolean(),
        ["models", "*", "contextWindow" | "maxTokens"]
        | ["models", "*", "cost", "input" | "output" | "cacheRead" | "cacheWrite"] => {
            value.is_number()
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
        Value::Array(models) if path == ["models"] => {
            Value::Array(models.iter().map(|model| project(model, &["models", "*"])).collect())
        }
        _ => value.clone(),
    }
}

fn restore(value: &mut Value, before: Option<&Value>, path: &[&str]) -> Result<bool, WriteError> {
    let invalid = WriteError::InvalidResource;
    if value.as_str() == Some(KEEP) {
        let original = before.filter(|original| !public(path, original)).ok_or(invalid)?;
        *value = original.clone();
        return Ok(true);
    }
    if !public(path, value) {
        return Err(invalid);
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
        Value::Array(models) if path == ["models"] => {
            // Match saved opaque model fields by ID, even when rows have been reordered.
            let previous = before
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|model| Some((model.get("id")?.as_str()?, model)))
                .collect::<std::collections::HashMap<_, _>>();
            for model in models {
                let old = model.get("id").and_then(Value::as_str).and_then(|id| previous.get(id));
                kept |= restore(model, old.copied(), &["models", "*"])?;
            }
        }
        _ => {}
    }
    Ok(kept)
}

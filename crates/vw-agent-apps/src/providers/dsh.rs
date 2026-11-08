//! Dsh member projection. Shared account grants and device records stay native-owned.
use super::live::write_guard::WriteError;
use serde_json::{Value, json};
use serde_yaml::Value as Yaml;
use std::collections::BTreeMap;
use vw_api_types::app_providers::dsh::*;
const MAX_YAML_BYTES: usize = 1024 * 1024;

pub type Snapshot = BTreeMap<String, Option<Vec<u8>>>;
fn invalid<T>() -> Result<T, WriteError> {
    Err(WriteError::InvalidResource)
}
pub fn validate(id: &str, p: &DshProfile) -> Result<(), WriteError> {
    if id.is_empty()
        || id.len() > 128
        || id == "deepseek-official"
        || id.starts_with('-')
        || id.ends_with('-')
        || id.contains("--")
        || id.chars().any(|c| !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-')
        || p.name.trim().is_empty()
        || p.name.len() > 128
        || p.name.chars().any(char::is_control)
    {
        return invalid();
    }
    let node = p.declaration.as_object().ok_or(WriteError::InvalidResource)?;
    if serde_json::to_vec(node).map_err(|_| WriteError::InvalidResource)?.len() > 1024 * 1024
        || !matches!(
            node.get("api").and_then(Value::as_str),
            Some("openai-completions" | "openai-responses" | "anthropic-messages")
        )
    {
        return invalid();
    }
    let options =
        node.get("options").and_then(Value::as_object).ok_or(WriteError::InvalidResource)?;
    let url = options.get("baseURL").and_then(Value::as_str).ok_or(WriteError::InvalidResource)?;
    super::validation::endpoint(url).map_err(|_| WriteError::InvalidResource)?;
    if options.keys().any(|k| !matches!(k.as_str(), "baseURL" | "apiKey" | "headers"))
        || node.contains_key("authBinding")
        || node.contains_key("meta")
    {
        return invalid();
    }
    let key = options.get("apiKey").and_then(Value::as_str).unwrap_or_default();
    let reference = node.get("apiKeyEnv").and_then(Value::as_str).unwrap_or_default();
    if key.trim().is_empty() && reference.is_empty()
        || matches!(key, "[redacted]" | "<redacted>")
        || !key.is_empty() && key.chars().all(|c| matches!(c, '*' | '•'))
        || key.len() > 16384
        || key.chars().any(char::is_control)
        || !reference.is_empty()
            && (!reference.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                || reference.starts_with(|c: char| c.is_ascii_digit()))
    {
        return invalid();
    }
    if let Some(h) = options.get("headers")
        && !h.as_object().is_some_and(|m| m.values().all(Value::is_string))
    {
        return invalid();
    }
    let models = node
        .get("models")
        .and_then(Value::as_object)
        .filter(|m| !m.is_empty())
        .ok_or(WriteError::InvalidResource)?;
    for (id, m) in models {
        if id.trim().is_empty() || !m.is_object() {
            return invalid();
        }
        for field in ["/limit/context", "/limit/output", "/contextWindow", "/maxTokens"] {
            if m.pointer(field).is_some_and(|v| !v.is_null() && v.as_u64().is_none_or(|n| n == 0)) {
                return invalid();
            }
        }
    }
    Ok(())
}
/// Managed login references are not resolvable by the provider catalog; never turn one into an API-key row.
pub fn valid_binding(record: Option<&Value>) -> bool {
    !record.is_some_and(|v| {
        v.pointer("/meta/authBinding").is_some()
            || v.pointer("/meta/liveConfigManaged").is_some_and(|v| v == true)
    })
}
fn yaml(bytes: Option<&[u8]>) -> Result<Yaml, WriteError> {
    let bytes = bytes.unwrap_or_default();
    if bytes.len() > MAX_YAML_BYTES {
        return invalid();
    }
    serde_yaml::from_slice(bytes).map_err(|_| WriteError::InvalidResource)
}
fn entries(bytes: Option<&[u8]>) -> Result<Vec<Yaml>, WriteError> {
    match yaml(bytes)? {
        Yaml::Null => Ok(vec![]),
        Yaml::Sequence(v) => Ok(v),
        _ => invalid(),
    }
}
fn providers(bytes: Option<&[u8]>) -> Result<Yaml, WriteError> {
    let nodes = entries(bytes)?;
    let found: Vec<_> = nodes.iter().filter(|v| v["id"].as_str() == Some("llm-pi-ai")).collect();
    if found.len() > 1 {
        return invalid();
    }
    if found.first().and_then(|v| v.get("config")).is_some_and(|v| !v.is_mapping()) {
        return invalid();
    }
    match found.first().and_then(|v| v.get("config")).and_then(|v| v.get("providers")) {
        None | Some(Yaml::Null) => Ok(Yaml::Mapping(Default::default())),
        Some(v @ Yaml::Mapping(_))
            if v.as_mapping().unwrap().keys().all(|k| k.as_str().is_some()) =>
        {
            Ok(v.clone())
        }
        _ => invalid(),
    }
}
pub fn state(pre: &Snapshot) -> Result<DshState, WriteError> {
    let mut ids = std::collections::BTreeSet::new();
    for (key, bytes) in pre {
        if !matches!(key.as_str(), "credentials" | "home") {
            for key in providers(bytes.as_deref())?.as_mapping().unwrap().keys() {
                ids.insert(key.as_str().unwrap().to_owned());
            }
        }
    }
    Ok(DshState { members: ids.into_iter().collect() })
}
pub(crate) fn credential_document(bytes: Option<&[u8]>) -> Result<Yaml, WriteError> {
    let v = yaml(bytes)?;
    if v.is_null() {
        return Ok(serde_yaml::from_str("version: 1\nrefs: {}\n").unwrap());
    }
    if !v.is_mapping()
        || v["version"].as_u64() != Some(1)
        || v.get("refs").is_some_and(|v| !v.is_null() && !v.is_mapping())
        || v.get("records").is_some_and(|v| !v.is_null() && !v.is_mapping())
    {
        return invalid();
    }
    Ok(v)
}
pub fn credential_binding_valid(pre: &Snapshot, p: &DshProfile) -> bool {
    if p.declaration
        .pointer("/options/apiKey")
        .and_then(Value::as_str)
        .is_some_and(|k| !k.trim().is_empty())
    {
        return true;
    }
    let reference = p.declaration.get("apiKeyEnv").and_then(Value::as_str).unwrap_or_default();
    credential_document(pre.get("credentials").and_then(|v| v.as_deref()))
        .is_ok_and(|v| v["refs"][reference].as_str().is_some_and(|k| !k.trim().is_empty()))
}
fn native(id: &str, p: &DshProfile) -> Value {
    let mut node = p.declaration.clone();
    let obj = node.as_object_mut().unwrap();
    for k in ["name", "options", "kind", "enabled"] {
        obj.remove(k);
    }
    obj.insert("displayName".into(), json!(p.name));
    obj.insert("baseURL".into(), p.declaration["options"]["baseURL"].clone());
    if let Some(h) = p.declaration.pointer("/options/headers") {
        obj.insert("headers".into(), h.clone());
    }
    if p.declaration
        .pointer("/options/apiKey")
        .and_then(Value::as_str)
        .is_some_and(|k| !k.trim().is_empty())
    {
        obj.insert(
            "apiKeyEnv".into(),
            json!(format!("SHIJU_DSH_{}", id.replace('-', "_").to_ascii_uppercase())),
        );
    }
    let mut models = vec![];
    for (id, m) in p.declaration["models"].as_object().unwrap() {
        let mut m = m.clone();
        m["id"] = json!(id);
        for (a, b) in [("context", "contextWindow"), ("output", "maxTokens")] {
            if let Some(v) = m.pointer(&format!("/limit/{a}")).filter(|v| !v.is_null()).cloned() {
                m[b] = v;
            }
        }
        let input: Vec<Value> = m
            .pointer("/modalities/input")
            .or_else(|| m.get("input"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|v| matches!(v.as_str(), Some("text" | "image")))
            .cloned()
            .collect();
        let object = m.as_object_mut().unwrap();
        for k in ["limit", "modalities", "input"] {
            object.remove(k);
        }
        if !input.is_empty() {
            object.insert("input".into(), json!(input));
        }
        if let Some(reasoning) = object.remove("reasoning") {
            if reasoning == false {
                object.insert("reasoningEfforts".into(), json!(false));
            } else if reasoning == true && !object.contains_key("reasoningEfforts") {
                object.insert(
                    "reasoningEfforts".into(),
                    json!({"off":null,"low":"low","medium":"medium","high":"high"}),
                );
            }
        }
        models.push(m);
    }
    obj.insert("models".into(), json!(models));
    node
}
/// Splice only the llm-pi-ai config's provider mapping. Other plugin bytes survive, including !!js.
fn patch_profile(
    bytes: Option<&[u8]>,
    id: &str,
    node: Option<&Value>,
) -> Result<Vec<u8>, WriteError> {
    let text =
        std::str::from_utf8(bytes.unwrap_or_default()).map_err(|_| WriteError::InvalidResource)?;
    let parsed = entries(bytes)?;
    let matching = parsed.iter().position(|v| v["id"].as_str() == Some("llm-pi-ai"));
    let replacement = if let Some(index) = matching {
        let lines: Vec<_> = text.split_inclusive('\n').collect();
        let starts: Vec<_> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.starts_with("- ") || l.trim_end() == "-")
            .map(|(i, _)| i)
            .collect();
        if starts.len() != parsed.len() {
            return invalid();
        }
        let start = starts[index];
        let end = starts.get(index + 1).copied().unwrap_or(lines.len());
        let config = (start..end)
            .find(|i| matches!(lines[*i].trim_end(), "  config:" | "- config:"))
            .ok_or(WriteError::InvalidResource)?;
        let config_end = (config + 1..end)
            .find(|i| {
                !lines[*i].trim().is_empty()
                    && !lines[*i].trim_start().starts_with('#')
                    && !lines[*i].starts_with("    ")
            })
            .unwrap_or(end);
        let mut body = String::new();
        for line in &lines[config + 1..config_end] {
            body.push_str(line.strip_prefix("    ").unwrap_or(line));
        }
        let next = super::live::patch_yaml::patch_provider(
            Some(body.as_bytes()),
            super::live::patch_yaml::ProviderSection::Providers,
            id,
            node,
        )
        .map_err(|_| WriteError::InvalidResource)?;
        let mut result = lines[..config + 1].concat();
        for line in std::str::from_utf8(&next)
            .map_err(|_| WriteError::InvalidResource)?
            .split_inclusive('\n')
        {
            result.push_str("    ");
            result.push_str(line);
        }
        result.push_str(&lines[config_end..].concat());
        result
    } else {
        let Some(node) = node else {
            return Ok(bytes.unwrap_or_default().to_vec());
        };
        let entry =
            serde_yaml::to_string(&json!([{"id":"llm-pi-ai","config":{"providers":{id:node}}}]))
                .map_err(|_| WriteError::InvalidResource)?;
        if parsed.is_empty() {
            let mut result: String = text
                .lines()
                .filter(|l| l.trim_start().starts_with('#') || l.trim().is_empty())
                .map(|l| format!("{l}\n"))
                .collect();
            result.push_str(&entry);
            result
        } else {
            format!("{text}{}{entry}", if text.ends_with('\n') { "" } else { "\n" })
        }
    };
    let mut expected = providers(bytes)?;
    if let Some(node) = node {
        expected.as_mapping_mut().unwrap().insert(
            id.into(),
            serde_yaml::to_value(node).map_err(|_| WriteError::InvalidResource)?,
        );
    } else {
        expected.as_mapping_mut().unwrap().remove(Yaml::from(id));
    }
    if providers(Some(replacement.as_bytes()))? != expected {
        return invalid();
    }
    Ok(replacement.into_bytes())
}
pub fn plan(
    pre: &Snapshot,
    id: &str,
    p: &DshProfile,
    activation: DshActivation,
) -> Result<Snapshot, WriteError> {
    validate(id, p)?;
    if !credential_binding_valid(pre, p) {
        return invalid();
    }
    if entries(pre.get("home").and_then(|v| v.as_deref()))?
        .iter()
        .any(|v| v["id"].as_str() == Some("llm-pi-ai") && !v["config"].is_null())
    {
        return invalid();
    }
    let mut existing = None;
    for (key, bytes) in pre {
        if matches!(key.as_str(), "credentials" | "home") {
            continue;
        }
        let nodes = providers(bytes.as_deref())?;
        if let Some(node) = nodes.get(id) {
            if activation == DshActivation::Add {
                return Err(WriteError::Conflict);
            }
            if existing.as_ref().is_some_and(|v| v != node) {
                return Err(WriteError::Conflict);
            }
            existing = Some(node.clone());
        }
    }
    if activation == DshActivation::CatalogOnly {
        return invalid();
    }
    if activation == DshActivation::IfMember && existing.is_none() {
        return Ok(pre.clone());
    }
    let mut next = pre.clone();
    let node = native(id, p);
    for (key, bytes) in pre {
        if !matches!(key.as_str(), "credentials" | "home") {
            next.insert(key.clone(), Some(patch_profile(bytes.as_deref(), id, Some(&node))?));
        }
    }
    if let Some(key) = p
        .declaration
        .pointer("/options/apiKey")
        .and_then(Value::as_str)
        .filter(|k| !k.trim().is_empty())
    {
        let mut creds = credential_document(pre.get("credentials").and_then(|v| v.as_deref()))?;
        if creds["refs"].is_null() {
            creds["refs"] = Yaml::Mapping(Default::default());
        }
        let reference = node["apiKeyEnv"].as_str().unwrap();
        // Never overwrite a reference used by a different native provider.
        if creds["refs"].get(reference).is_some()
            && existing.as_ref().is_none_or(|n| n["apiKeyEnv"].as_str() != Some(reference))
        {
            return Err(WriteError::Conflict);
        }
        for (name, bytes) in pre {
            if matches!(name.as_str(), "credentials" | "home") {
                continue;
            }
            let nodes = providers(bytes.as_deref())?;
            if nodes.as_mapping().unwrap().iter().any(|(other, node)| {
                other.as_str() != Some(id) && node["apiKeyEnv"].as_str() == Some(reference)
            }) {
                return Err(WriteError::Conflict);
            }
        }
        creds["refs"][reference] = key.into();
        next.insert(
            "credentials".into(),
            Some(
                serde_yaml::to_string(&creds)
                    .map_err(|_| WriteError::InvalidResource)?
                    .into_bytes(),
            ),
        );
    }
    Ok(next)
}

#[path = "dsh_files.rs"]
mod files;
#[cfg(target_os = "macos")]
pub(crate) use files::DshCredentials;
pub use files::{DshFiles, read};

#[path = "dsh_edit.rs"]
mod edit;
pub use edit::{member, remove};
#[path = "dsh_form.rs"]
pub mod form;

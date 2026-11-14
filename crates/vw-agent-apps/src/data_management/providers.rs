//! CC provider migration preparation. Sensitive rows never implement Debug/Serialize.
use std::collections::{BTreeMap, BTreeSet};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use vw_api_types::{agent_apps::AppId, app_providers::{claude::*, gemini::*}};
use super::{preview::{self, Decision, ImportPreview, PreviewCandidate, PreviewError}, source::*};

pub struct ProviderRow {
    pub app: AppId,
    pub id: String,
    pub value: Value,
}
pub struct ProviderPreview {
    preview: ImportPreview,
    rows: Vec<ProviderRow>,
}
impl ProviderPreview {
    pub fn entries(&self) -> &[preview::PreviewEntry] { self.preview.entries() }
    /// Re-resolve under the owner's transaction. The retained rows and handles
    /// were produced together, so execution cannot substitute another payload.
    pub fn resolve(&self, scope: &SourceScope, source: &SourceSnapshot,
        existing: &[ProviderRow], decisions: &[(Uuid, Decision)],
    ) -> Result<Vec<&ProviderRow>, PreviewError> {
        let resolved = self.preview.resolve(scope, source, &revision(existing)?, decisions)?;
        Ok(self.preview.entries().iter().zip(&self.rows)
            .filter(|(entry, _)| resolved[&entry.id] == Decision::Replace)
            .map(|(_, row)| row).collect())
    }
}

pub fn revision(rows: &[ProviderRow]) -> Result<String, PreviewError> {
    let mut values = BTreeMap::new();
    for row in rows {
        let key = key(row.app, &row.id)?;
        if values.insert(key, &row.value).is_some() { return Err(PreviewError::InvalidCandidate); }
    }
    let bytes = serde_json::to_vec(&values).map_err(|_| PreviewError::InvalidCandidate)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
pub fn key(app: AppId, id: &str) -> Result<String, PreviewError> {
    crate::providers::validate_identity(id).map_err(|_| PreviewError::InvalidCandidate)?;
    Ok(format!("{}:{id}", serde_json::to_string(&app).map_err(|_| PreviewError::InvalidCandidate)?))
}

/// Only catalog metadata/configuration is imported. Runtime current/queue flags
/// never become activation. Missing credential ownership is an explicit error.
pub fn prepare(scope: &SourceScope, source: &SourceSnapshot, existing: &[ProviderRow])
    -> Result<ProviderPreview, PreviewError> {
    let source_records = source.records(scope).map_err(|_| PreviewError::Forbidden)?;
    let mut expanded = Vec::new();
    for record in source_records.iter().filter(|r| r.category == SourceCategory::Providers) {
        if record.origin.starts_with('/') {
            let app = record.origin.trim_start_matches('/');
            let providers = record.fields.get("providers").and_then(Value::as_object).ok_or(PreviewError::InvalidCandidate)?;
            for (id, fields) in providers {
                let mut fields = fields.clone();
                let object = fields.as_object_mut().ok_or(PreviewError::InvalidCandidate)?;
                if object.get("id").is_some_and(|v| v.as_str() != Some(id.as_str())) { return Err(PreviewError::InvalidCandidate); }
                object.insert("id".into(), json!(id));
                object.insert("app_type".into(), json!(app));
                for (old, new) in [("settingsConfig", "settings_config"), ("sortIndex", "sort_index")] {
                    if let Some(value) = object.remove(old) { object.insert(new.into(), value); }
                }
                expanded.push(SourceRecord { category: SourceCategory::Providers, origin: "providers".into(), fields });
            }
        } else {
            expanded.push(SourceRecord { category: record.category, origin: record.origin.clone(), fields: record.fields.clone() });
        }
    }
    let records = &expanded;
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    for record in records.iter().filter(|r| r.category == SourceCategory::Providers) {
        if record.origin != "providers" { continue; }
        let f = &record.fields;
        let id = string(f, "id")?;
        let app: AppId = serde_json::from_value(f.get("app_type").cloned().ok_or(PreviewError::InvalidCandidate)?)
            .map_err(|_| PreviewError::UnsupportedCategory)?;
        if !seen.insert(key(app, id)?) { return Err(PreviewError::InvalidCandidate); }
        let before = existing.iter().find(|r| r.app == app && r.id == id).map(|r| &r.value);
        let settings = match f.get("settings_config") {
            Some(Value::String(s)) => serde_json::from_str(s).map_err(|_| PreviewError::InvalidCandidate)?,
            Some(v) if v.is_object() => v.clone(),
            _ => return Err(PreviewError::InvalidCandidate),
        };
        let name = string(f, "name")?;
        let mut value = normalize(app, name, &settings, before)?;
        for (from, to) in [("sort_index", "sortIndex"), ("category", "category"), ("notes", "notes"), ("icon", "icon")] {
            if let Some(v) = f.get(from).filter(|v| !v.is_null()) { value[to] = v.clone(); }
        }
        crate::providers::icons::read(&value).map_err(|_| PreviewError::InvalidCandidate)?;
        let presentation = crate::providers::presentation::read(&value).map_err(|_| PreviewError::InvalidCandidate)?;
        value = crate::providers::presentation::update(&value, &presentation).map_err(|_| PreviewError::InvalidCandidate)?;
        rows.push(ProviderRow { app, id: id.into(), value });
    }
    // Endpoint references must resolve within this same selected category.
    for record in records.iter().filter(|r| r.category == SourceCategory::Providers) {
        if record.origin == "providers" { continue; }
        if record.origin != "provider_endpoints" { return Err(PreviewError::UnsupportedCategory); }
        let f = &record.fields;
        let app: AppId = serde_json::from_value(f.get("app_type").cloned().ok_or(PreviewError::InvalidCandidate)?)
            .map_err(|_| PreviewError::InvalidCandidate)?;
        let id = string(f, "provider_id")?;
        let row = rows.iter_mut().find(|r| r.app == app && r.id == id).ok_or(PreviewError::InvalidCandidate)?;
        use vw_api_types::app_providers::endpoints::*;
        let url = string(f, "url")?.to_owned();
        let timestamp = f.get("added_at").and_then(Value::as_u64).ok_or(PreviewError::InvalidCandidate)?;
        row.value = crate::providers::endpoints::update(&row.value,
            &AppProviderEndpointChange::Add { url, kind: AppProviderEndpointKind::BaseUrl }, timestamp)
            .map_err(|_| PreviewError::InvalidCandidate)?;
    }
    let preview = preview::build_preview(scope, source, &[SourceCategory::Providers], revision(existing)?, |_, _| {
        Ok(rows.iter().map(|row| PreviewCandidate {
            incoming: fingerprint(&row.value),
            existing: existing.iter().find(|r| r.app == row.app && r.id == row.id).map(|r| fingerprint(&r.value)),
            references_valid: true,
        }).collect())
    })?;
    Ok(ProviderPreview { preview, rows })
}
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str, PreviewError> {
    v.get(key).and_then(Value::as_str).filter(|s| !s.trim().is_empty()).ok_or(PreviewError::InvalidCandidate)
}
fn normalize(app: AppId, name: &str, settings: &Value, before: Option<&Value>) -> Result<Value, PreviewError> {
    // Filter before domain decoding; never copy source credentials, host paths,
    // permissions, or opaque config. Existing credentials stay with their owner.
    if !settings.is_object() { return Err(PreviewError::InvalidCandidate); }
    if name.len() > 128 || name.chars().any(char::is_control) { return Err(PreviewError::InvalidCandidate); }
    let filtered = super::portable_policy::filter_portable_item(settings, super::portable_policy::MigrationLocation::CrossDevice);
    let mut safe = filtered.value;
    if app == AppId::Codex {
        // Parse TOML, then retain only routing/model fields; never copy commands,
        // permissions, auth headers, host paths or source login material.
        let raw = settings.get("config").and_then(Value::as_str).ok_or(PreviewError::InvalidCandidate)?;
        let document: toml::Value = toml::from_str(raw).map_err(|_| PreviewError::InvalidCandidate)?;
        let mut clean = toml::map::Map::new();
        for key in ["model", "model_provider", "model_reasoning_effort"] {
            if let Some(value) = document.get(key) {
                if !value.is_str() { return Err(PreviewError::InvalidCandidate); }
                clean.insert(key.into(), value.clone());
            }
        }
        if let Some(providers) = document.get("model_providers") {
            let mut result = toml::map::Map::new();
            for (id, value) in providers.as_table().ok_or(PreviewError::InvalidCandidate)? {
                let mut provider = toml::map::Map::new();
                for key in ["name", "base_url", "wire_api"] {
                    if let Some(v) = value.get(key) {
                        if !v.is_str() { return Err(PreviewError::InvalidCandidate); }
                        if key == "base_url" { crate::providers::validation::endpoint(v.as_str().unwrap()).map_err(|_| PreviewError::InvalidCandidate)?; }
                        provider.insert(key.into(), v.clone());
                    }
                }
                result.insert(id.clone(), toml::Value::Table(provider));
            }
            clean.insert("model_providers".into(), toml::Value::Table(result));
        }
        let config = toml::to_string(&toml::Value::Table(clean)).map_err(|_| PreviewError::InvalidCandidate)?;
        let mut profile = crate::providers::codex_catalog::import(Some(config.as_bytes()), None).map_err(|_| PreviewError::InvalidCandidate)?;
        profile.name = name.into();
        if let Some(existing) = before.and_then(|v| v.get("codex")) {
            let existing: vw_api_types::app_providers::codex_catalog::CodexProfile = serde_json::from_value(existing.clone()).map_err(|_| PreviewError::InvalidCandidate)?;
            profile.api_key = existing.api_key;
        }
        return Ok(json!({"name":name,"codex":profile,"settingsConfig":crate::providers::codex_catalog::settings(&profile)}));
    }
    for key in ["ANTHROPIC_BASE_URL", "GEMINI_BASE_URL"] {
        if settings.get("env").and_then(|v| v.get(key)).is_some()
            && safe.get("env").and_then(|v| v.get(key)).is_none() {
            return Err(PreviewError::InvalidCandidate);
        }
    }
    if let Some(env) = settings.get("env").and_then(Value::as_object) {
        if (app == AppId::Claude && !env.contains_key("ANTHROPIC_BASE_URL")
            && (env.contains_key("ANTHROPIC_API_KEY") || env.contains_key("ANTHROPIC_AUTH_TOKEN")))
            || (app == AppId::Gemini && !env.contains_key("GEMINI_BASE_URL") && env.contains_key("GEMINI_API_KEY")) {
            return Err(PreviewError::InvalidCandidate);
        }
    }
    let env = safe.get_mut("env").and_then(Value::as_object_mut).ok_or(PreviewError::InvalidCandidate)?;
    match app {
        AppId::Claude => {
            // Bedrock/Vertex need separately authorized host/account material.
            if settings.get("env").and_then(Value::as_object).is_some_and(|e|
                e.contains_key("CLAUDE_CODE_USE_BEDROCK") || e.contains_key("CLAUDE_CODE_USE_VERTEX")) {
                return Err(PreviewError::UnsupportedCategory);
            }
            if env.contains_key("ANTHROPIC_BASE_URL") {
                let existing: ClaudeProfile = serde_json::from_value(before.and_then(|v| v.get("claude")).cloned()
                    .ok_or(PreviewError::InvalidCandidate)?).map_err(|_| PreviewError::InvalidCandidate)?;
                let ClaudeConfig::ThirdParty { api_key, api_key_header, .. } = existing.config else { return Err(PreviewError::InvalidCandidate) };
                env.insert(if api_key_header { "ANTHROPIC_API_KEY" } else { "ANTHROPIC_AUTH_TOKEN" }.into(), json!(api_key));
            }
            let profile = crate::providers::claude::import(name, safe.to_string().as_bytes()).map_err(|_| PreviewError::InvalidCandidate)?;
            Ok(json!({"name":name, "claude":profile, "settingsConfig":crate::providers::claude::settings(&profile).map_err(|_| PreviewError::InvalidCandidate)?}))
        }
        AppId::Gemini => {
            let config = if let Some(url) = env.get("GEMINI_BASE_URL").and_then(Value::as_str) {
                let existing: GeminiProfile = serde_json::from_value(before.and_then(|v| v.get("gemini")).cloned()
                    .ok_or(PreviewError::InvalidCandidate)?).map_err(|_| PreviewError::InvalidCandidate)?;
                let GeminiConfig::ThirdParty { api_key, .. } = existing.config else { return Err(PreviewError::InvalidCandidate) };
                GeminiConfig::ThirdParty { base_url: url.into(), api_key }
            } else { GeminiConfig::Official {} };
            let profile = GeminiProfile { name: name.into(), config, model: settings.get("env").and_then(|v|v.get("GEMINI_MODEL")).and_then(Value::as_str).map(str::to_owned) };
            Ok(json!({"name":name, "gemini":profile, "settingsConfig":crate::providers::gemini::settings(&profile).map_err(|_| PreviewError::InvalidCandidate)?}))
        }
        // Opaque TOML and application-specific account references must not be
        // treated as a generic JSON provider. Their owners must decode them.
        _ => Err(PreviewError::UnsupportedCategory),
    }
}

fn fingerprint(value: &Value) -> Value {
    json!({"content":format!("{:x}", Sha256::digest(value.to_string().as_bytes()))})
}

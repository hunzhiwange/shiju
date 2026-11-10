//! Claude Code model discovery from saved provider records.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;
use serde_json::Value;
use vw_api_types::agent_apps::AppId;

use super::model_ids::{ONE_M_MARKER, encode, strip_one_m};

pub const CLAUDE_DEFAULT_WINDOW: u64 = 200_000;

const MODEL_MAPPINGS: [(&str, Option<&str>); 5] = [
    ("ANTHROPIC_MODEL", None),
    ("ANTHROPIC_DEFAULT_OPUS_MODEL", Some("ANTHROPIC_DEFAULT_OPUS_MODEL_NAME")),
    ("ANTHROPIC_DEFAULT_SONNET_MODEL", Some("ANTHROPIC_DEFAULT_SONNET_MODEL_NAME")),
    ("ANTHROPIC_DEFAULT_HAIKU_MODEL", Some("ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME")),
    ("ANTHROPIC_DEFAULT_FABLE_MODEL", Some("ANTHROPIC_DEFAULT_FABLE_MODEL_NAME")),
];

/// A saved Claude Code provider and its registered key, resolved by the authorized caller.
pub struct ClaudeCatalogMember<'a> {
    pub provider_id: &'a str,
    pub key: &'a str,
    pub record: &'a Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaudeCatalogError {
    InvalidMember,
    InvalidConfiguration,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ClaudeCatalogModel {
    pub id: String,
    pub provider_id: String,
    /// Configured upstream name, which can carry the client-side context marker.
    pub upstream_model: String,
    pub name: String,
    pub label: String,
    pub description: String,
    pub supports_1m: bool,
    /// Provider window for models without the 1M capability.
    pub window: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ClaudeModelCatalog {
    models: Vec<ClaudeCatalogModel>,
    has_default: bool,
}

impl ClaudeModelCatalog {
    /// Project eligible members in saved order, with the default model first when it exists.
    pub fn new(
        members: &[ClaudeCatalogMember<'_>],
        default_provider_id: Option<&str>,
    ) -> Result<Self, ClaudeCatalogError> {
        let mut models = Vec::new();
        let mut provider_ids = HashSet::new();
        let mut keys = HashSet::new();
        let mut default_index = None;
        for member in members {
            if !provider_ids.insert(member.provider_id) || !keys.insert(member.key) {
                return Err(ClaudeCatalogError::InvalidMember);
            }
            let start = models.len();
            models.extend(claude_models(member)?);
            if Some(member.provider_id) == default_provider_id && models.len() > start {
                default_index = Some(start);
            }
        }
        if default_provider_id.is_some_and(|id| !provider_ids.contains(id)) {
            return Err(ClaudeCatalogError::InvalidMember);
        }
        if let Some(index) = default_index {
            models[..=index].rotate_right(1);
        }
        Ok(Self { models, has_default: default_index.is_some() })
    }

    pub fn models(&self) -> &[ClaudeCatalogModel] {
        &self.models
    }

    pub fn default_model(&self) -> Option<&ClaudeCatalogModel> {
        self.has_default.then(|| &self.models[0])
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListedModel<'a> {
    model: &'a str,
    display_name: Option<&'a str>,
    #[serde(default)]
    one_m: bool,
}

struct FoundModel<'a> {
    upstream_model: String,
    name: Option<&'a str>,
    supports_1m: bool,
}

pub fn claude_models(
    member: &ClaudeCatalogMember<'_>,
) -> Result<Vec<ClaudeCatalogModel>, ClaudeCatalogError> {
    let provider = member
        .record
        .get("name")
        .and_then(Value::as_str)
        .ok_or(ClaudeCatalogError::InvalidConfiguration)?;
    let env = member.record.pointer("/settingsConfig/env").unwrap_or(&Value::Null);
    let window = env
        .get("CLAUDE_CODE_MAX_CONTEXT_TOKENS")
        .and_then(|value| match value {
            Value::String(text) => text.trim().parse().ok(),
            _ => value.as_u64(),
        })
        .filter(|&window| window > 0)
        .unwrap_or(CLAUDE_DEFAULT_WINDOW);

    let candidates: Vec<_> = match member.record.pointer("/meta/stackModels") {
        Some(list) if !list.is_null() => Vec::<ListedModel<'_>>::deserialize(list)
            .map_err(|_| ClaudeCatalogError::InvalidConfiguration)?
            .into_iter()
            .map(|row| (row.model, row.display_name, Some(row.one_m)))
            .collect(),
        _ => MODEL_MAPPINGS
            .into_iter()
            .filter_map(|(model_key, name_key)| {
                let model = env.get(model_key)?.as_str()?;
                let name = name_key.and_then(|key| env.get(key)?.as_str());
                Some((model, name, None))
            })
            .collect(),
    };

    let mut ordered: Vec<(&str, FoundModel<'_>)> = Vec::new();
    let mut indices = HashMap::new();
    for (raw, name, explicit_1m) in candidates {
        let raw = raw.trim();
        let marker = strip_one_m(raw);
        let model = marker.unwrap_or(raw);
        if model.is_empty() {
            continue;
        }
        let supports_1m = marker.is_some() || explicit_1m == Some(true);
        // Mapping entries preserve the configured marker spelling. Explicit rows normalize it.
        let upstream_model = if explicit_1m.is_some() && supports_1m {
            format!("{model}{ONE_M_MARKER}")
        } else {
            raw.into()
        };
        let name = name.map(str::trim).filter(|name| !name.is_empty());
        let index = *indices.entry(model).or_insert(ordered.len());
        if let Some((_, previous)) = ordered.get_mut(index) {
            if supports_1m && !previous.supports_1m {
                previous.upstream_model = upstream_model;
                previous.supports_1m = true;
            }
            previous.name = previous.name.or(name);
        } else {
            ordered.push((model, FoundModel { upstream_model, name, supports_1m }));
        }
    }

    ordered
        .into_iter()
        .map(|(model, found)| {
            let id = encode(AppId::Claude, member.key, model, found.supports_1m)
                .map_err(|_| ClaudeCatalogError::InvalidConfiguration)?;
            let name = found.name.unwrap_or(model);
            let shown_window = if found.supports_1m { 1_000_000 } else { window };
            let context = match shown_window {
                tokens if tokens.is_multiple_of(1_000_000) => format!("{}M", tokens / 1_000_000),
                tokens if tokens >= 1_000 => {
                    format!("{}K", tokens / 1_000 + u64::from(tokens % 1_000 >= 500))
                }
                tokens => tokens.to_string(),
            };
            let description = match provider.trim() {
                "" => format!("{model} · {context}"),
                provider => format!("{provider} · {model} · {context}"),
            };
            Ok(ClaudeCatalogModel {
                id,
                provider_id: member.provider_id.into(),
                upstream_model: found.upstream_model,
                name: name.into(),
                label: format!("{name}（{provider}）"),
                description,
                supports_1m: found.supports_1m,
                window,
            })
        })
        .collect()
}

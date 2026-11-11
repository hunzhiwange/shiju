//! Read-only assembly of the managed Codex aggregation catalog.

use std::collections::HashSet;

use super::model_ids::{self, Decoded};
use serde_json::{Map, Value, json};
use toml_edit::DocumentMut;
use vw_api_types::{
    agent_apps::AppId,
    model_catalog::{CodexNativeModel, SavedCodexCatalog},
};

#[derive(Debug, PartialEq, Eq)]
pub enum CodexCatalogError {
    NativeCatalogUnavailable,
    MissingDefaultModel,
    InvalidCatalog,
}

#[derive(Clone, Copy)]
pub struct CodexCatalogRow<'a> {
    pub catalog: Option<&'a SavedCodexCatalog>,
    pub config: &'a DocumentMut,
}

pub struct CodexCatalogMember<'a> {
    pub key: &'a str,
    pub provider_name: &'a str,
    pub row: CodexCatalogRow<'a>,
}

pub enum CodexCatalogDefault<'a> {
    ThirdParty(CodexCatalogRow<'a>),
    /// Native rows obtained and validated by the executor for the official default.
    Official {
        native: &'a [Map<String, Value>],
        config: &'a DocumentMut,
    },
}

/// Assemble from one authorized snapshot. The caller owns login, catalog-pointer
/// policy, client settings, revision checks, and transactional publication.
/// Members are in publication order and exclude the default provider.
pub fn plan_codex_stack_catalog(
    default: CodexCatalogDefault<'_>,
    members: &[CodexCatalogMember<'_>],
) -> Result<Value, CodexCatalogError> {
    let mut rows = match default {
        CodexCatalogDefault::ThirdParty(row) => {
            let rows = third_party_rows(row)?;
            if rows.is_empty() {
                return Err(CodexCatalogError::MissingDefaultModel);
            }
            rows
        }
        CodexCatalogDefault::Official { native, config } => {
            if native.is_empty() {
                return Err(CodexCatalogError::NativeCatalogUnavailable);
            }
            let windows = RowWindows::read(config)?;
            let mut rows = native.to_vec();
            // Stable sort retains the native order of rows with equal priority.
            rows.sort_by_key(|row| row.get("priority").and_then(Value::as_i64).unwrap_or(i64::MAX));
            for row in &mut rows {
                windows.apply(row);
            }
            rows
        }
    };

    for row in &rows {
        if !matches!(model_ids::decode(AppId::Codex, slug(row)?), Ok(Decoded::Plain)) {
            return Err(CodexCatalogError::InvalidCatalog);
        }
    }

    let mut keys = HashSet::new();
    for member in members {
        if !keys.insert(member.key) {
            return Err(CodexCatalogError::InvalidCatalog);
        }
        rows.extend(member_rows(member)?);
    }

    let mut slugs = HashSet::new();
    for (index, row) in rows.iter_mut().enumerate() {
        row.insert("priority".into(), json!(index + 1));
        if !slugs.insert(slug(row)?) {
            return Err(CodexCatalogError::InvalidCatalog);
        }
    }
    Ok(json!({ "models": rows }))
}

/// Project a non-default member for publication or read-only collision reporting.
pub fn member_rows(
    member: &CodexCatalogMember<'_>,
) -> Result<Vec<Map<String, Value>>, CodexCatalogError> {
    third_party_rows(member.row)?
        .into_iter()
        .map(|mut row| {
            let model = slug(&row)?;
            let id = model_ids::encode(AppId::Codex, member.key, model, false)
                .map_err(|_| CodexCatalogError::InvalidCatalog)?;
            let label = row.get("display_name").and_then(Value::as_str).unwrap_or(model);
            let display = format!("{label}（{}）", member.provider_name);
            let window = row.get("context_window").and_then(Value::as_u64).unwrap_or_default();
            let description = match window {
                0 => model.to_owned(),
                w if w.is_multiple_of(1_000_000) => format!("{model} · {}M", w / 1_000_000),
                w if w >= 1_000 => {
                    format!("{model} · {}K", w / 1_000 + u64::from(w % 1_000 >= 500))
                }
                w => format!("{model} · {w}"),
            };
            row.insert("slug".into(), json!(id));
            row.insert("display_name".into(), json!(display));
            row.insert("description".into(), json!(description));
            Ok(row)
        })
        .collect()
}

fn slug(row: &Map<String, Value>) -> Result<&str, CodexCatalogError> {
    row.get("slug")
        .and_then(Value::as_str)
        .filter(|slug| !slug.trim().is_empty() && !slug.chars().any(char::is_control))
        .ok_or(CodexCatalogError::InvalidCatalog)
}

fn third_party_rows(
    row: CodexCatalogRow<'_>,
) -> Result<Vec<Map<String, Value>>, CodexCatalogError> {
    let windows = RowWindows::read(row.config)?;
    if let Some(catalog) = row.catalog.filter(|catalog| !catalog.models.is_empty()) {
        return catalog.models.iter().map(|model| third_party_row(model, &windows)).collect();
    }

    let model = match row.config.get("model") {
        Some(item) => item.as_str().ok_or(CodexCatalogError::InvalidCatalog)?.trim(),
        None => "",
    };
    if model.is_empty() {
        return Ok(Vec::new());
    }
    let fallback = CodexNativeModel {
        slug: model.to_owned(),
        display_name: model.to_owned(),
        description: model.to_owned(),
        base_instructions: "You are Codex, a coding agent.".to_owned(),
        supports_reasoning_summaries: false,
        supports_parallel_tool_calls: false,
        context_window: None,
        supported_reasoning_levels: vec![],
    };
    Ok(vec![third_party_row(&fallback, &windows)?])
}

fn third_party_row(
    model: &CodexNativeModel,
    windows: &RowWindows,
) -> Result<Map<String, Value>, CodexCatalogError> {
    // Use a neutral tool contract for saved and fallback models. Never copy
    // official capabilities based on a third-party model's name.
    let Value::Object(mut row) = json!({
        "slug": model.slug,
        "display_name": if model.display_name.trim().is_empty() { &model.slug } else { &model.display_name },
        "description": model.description,
        "base_instructions": model.base_instructions,
        "supports_reasoning_summaries": model.supports_reasoning_summaries,
        "supports_parallel_tool_calls": model.supports_parallel_tool_calls,
        "shell_type": "shell_command",
        "visibility": "list",
        "supported_in_api": true,
        "supported_reasoning_levels": model.supported_reasoning_levels,
        "default_reasoning_summary": "none",
        "support_verbosity": false,
        "supports_image_detail_original": false,
        "effective_context_window_percent": 95,
        "input_modalities": ["text", "image"],
        "supports_search_tool": false,
        "truncation_policy": { "mode": "bytes", "limit": 10000 },
        "experimental_supported_tools": [],
        "use_responses_lite": false,
        // Share history only among these rows with the same direct tool contract.
        "comp_hash": "cc-switch"
    }) else {
        unreachable!("Codex catalog row is an object");
    };

    let window = windows.context.or(model.context_window).unwrap_or(128_000);
    if window == 0 {
        return Err(CodexCatalogError::InvalidCatalog);
    }
    row.insert("context_window".into(), json!(window));
    row.insert("max_context_window".into(), json!(window));
    let limit = windows.compact.unwrap_or(window - window.div_ceil(10));
    row.insert("auto_compact_token_limit".into(), json!(limit));
    Ok(row)
}

struct RowWindows {
    context: Option<u64>,
    compact: Option<u64>,
}

impl RowWindows {
    fn read(config: &DocumentMut) -> Result<Self, CodexCatalogError> {
        let read = |key| {
            config
                .get(key)
                .map(|value| {
                    value
                        .as_integer()
                        .and_then(|n| u64::try_from(n).ok())
                        .filter(|n| *n > 0)
                        .ok_or(CodexCatalogError::InvalidCatalog)
                })
                .transpose()
        };
        Ok(Self {
            context: read("model_context_window")?,
            compact: read("model_auto_compact_token_limit")?,
        })
    }

    fn apply(&self, row: &mut Map<String, Value>) {
        if let Some(window) = self.context {
            row.insert("context_window".into(), json!(window));
            row.insert("max_context_window".into(), json!(window));
        }
        if let Some(limit) = self.compact {
            row.insert("auto_compact_token_limit".into(), json!(limit));
        }
    }
}

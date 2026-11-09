//! Editable Codex supplier fields. Opaque TOML and credentials stay on the executor.
use super::{identity, settings};
use crate::providers::{basic, projection, validate_identity, validation};
use serde_json::Value;
use toml_edit::{DocumentMut, Item, Table, TableLike, value};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::AuthErrorCode,
    app_providers::{codex_catalog::*, create::ProviderSecretWrite},
    model_catalog::{CodexNativeModel, SavedCodexCatalog},
};

type Result<T> = std::result::Result<T, &'static str>;
const INVALID: &str = "invalid or unsupported Codex editor configuration";
const ROUTE_FIELDS: &[&str] = &["base_url", "name", "wire_api", "requires_openai_auth"];

fn parse(text: &str) -> Result<DocumentMut> {
    if text.len() > 128 * 1024 {
        return Err(INVALID);
    }
    text.parse().map_err(|_| INVALID)
}

fn copy_scalar(field: &str, item: &Item) -> Result<Item> {
    match field {
        "disable_response_storage"
        | "model_supports_reasoning_summaries"
        | "requires_openai_auth" => item.as_bool().map(value).ok_or(INVALID),
        "model_context_window" | "model_auto_compact_token_limit" => {
            item.as_integer().map(value).ok_or(INVALID)
        }
        _ => {
            let v = item
                .as_str()
                .filter(|v| v.len() <= 2048 && !v.chars().any(char::is_control))
                .ok_or(INVALID)?;
            match field {
                "base_url" | "openai_base_url" => validation::endpoint(v).map_err(|_| INVALID)?,
                "wire_api" if v != "responses" => return Err(INVALID),
                _ => {}
            }
            Ok(value(v))
        }
    }
}

fn table<'a>(doc: &'a mut DocumentMut, key: &str) -> Result<&'a mut dyn TableLike> {
    if !doc.contains_key(key) {
        doc.insert(key, Item::Table(Table::new()));
    }
    doc.get_mut(key).and_then(Item::as_table_like_mut).ok_or(INVALID)
}

// Extract only routing/model fields into a new document; re-encoding scalars also drops
// comments and formatting that could contain credentials. The remainder is never returned.
fn extract(doc: &mut DocumentMut) -> Result<DocumentMut> {
    use projection::{codex_route::ROW_TOP_FIELDS, fields};
    let mut visible = DocumentMut::new();
    let selector = doc.get("model_provider").and_then(Item::as_str).map(str::to_owned);
    if doc.contains_key("model_provider")
        && !selector.as_deref().is_some_and(|id| validate_identity(id).is_ok())
        || doc.contains_key("experimental_bearer_token")
    {
        return Err(INVALID);
    }
    for &key in ROW_TOP_FIELDS.iter().chain(fields::CODEX_EXCLUSIVE_TOP).chain(&[
        "model_provider",
        "openai_base_url",
        "base_url",
        "wire_api",
    ]) {
        if let Some(item) = doc.remove(key) {
            visible.insert(key, copy_scalar(key, &item)?);
        }
    }
    for &path in fields::CODEX_FLOOR_NESTED {
        let [parent, key] = path else { return Err(INVALID) };
        if let Some(source) = doc.get_mut(parent).and_then(Item::as_table_like_mut) {
            if let Some(item) = source.remove(key) {
                table(&mut visible, parent)?.insert(key, copy_scalar(key, &item)?);
            }
            if source.is_empty() {
                doc.remove(parent);
            }
        }
    }
    if let Some(providers) = doc.get_mut("model_providers") {
        let providers = providers.as_table_like_mut().ok_or(INVALID)?;
        if let Some(id) = selector
            && let Some(item) = providers.remove(&id)
        {
            let source = item.as_table_like().ok_or(INVALID)?;
            let mut route = Table::new();
            for (key, item) in source.iter() {
                if !ROUTE_FIELDS.contains(&key) {
                    return Err(INVALID);
                }
                route.insert(key, copy_scalar(key, item)?);
            }
            table(&mut visible, "model_providers")?.insert(&id, Item::Table(route));
        }
        if providers.is_empty() {
            doc.remove("model_providers");
        }
    }
    Ok(visible)
}

fn merge(doc: &mut DocumentMut, visible: DocumentMut) -> Result<()> {
    for (key, item) in visible.into_table() {
        if let Item::Table(source) = item {
            let target = table(doc, &key)?;
            for (member, item) in source {
                if target.contains_key(&member) {
                    // Never take over an inactive provider table with opaque settings.
                    return Err(INVALID);
                }
                target.insert(&member, item);
            }
        } else {
            doc.insert(&key, item);
        }
    }
    Ok(())
}

pub fn editable_profile(id: &str, row: &Value) -> Option<CodexProfile> {
    if crate::providers::preview::editor_category(AppId::Codex, id, row, None).is_some() {
        return None;
    }
    serde_json::from_value(row.get("codex")?.clone()).ok()
}

pub fn external_catalog(profile: &CodexProfile) -> bool {
    parse(&profile.config).is_ok_and(|doc| doc.contains_key("model_catalog_json"))
}

pub fn view(
    row: &Value,
    saved: Option<&CodexProfile>,
    catalog: Option<&SavedCodexCatalog>,
) -> Result<CodexFormView> {
    let fields = basic::read(row).map_err(|_| INVALID)?;
    let config = saved.and_then(|saved| {
        let mut full = parse(&saved.config).ok()?;
        let visible = extract(&mut full).ok()?;
        let inactive =
            projection::codex(DocumentMut::new(), &settings(saved), None, None, false, false)
                .ok()?
                .inactive;
        Some(CodexFormConfig {
            toml: visible.to_string(),
            has_saved_key: saved.api_key.is_some(),
            inactive,
            models: (!external_catalog(saved)).then(|| {
                catalog
                    .into_iter()
                    .flat_map(|catalog| &catalog.models)
                    .map(|row| CodexModelMapping {
                        slug: row.slug.clone(),
                        display_name: row.display_name.clone(),
                    })
                    .collect()
            }),
        })
    });
    let can_bind = official_binding_model(row).is_ok();
    Ok(CodexFormView { fields, config, can_bind })
}

pub fn prepare(id: &str, request: &CodexFormWrite, before: Option<&Value>) -> Result<CodexProfile> {
    if request.fields.category.as_deref().is_some_and(|c| c.trim().eq_ignore_ascii_case("official"))
    {
        return Err(INVALID);
    }
    let old = before.map(|row| editable_profile(id, row).ok_or(INVALID)).transpose()?;
    let mut draft = parse(&request.toml)?;
    let visible = extract(&mut draft)?;
    if !draft.as_table().is_empty() {
        return Err(INVALID);
    }
    let mut merged = if let Some(old) = &old {
        let mut doc = parse(&old.config)?;
        extract(&mut doc)?;
        doc
    } else {
        DocumentMut::new()
    };
    if visible
        .get("model_provider")
        .and_then(Item::as_str)
        .is_some_and(|id| merged.get("model_providers").and_then(|p| p.get(id)).is_some())
    {
        return Err(INVALID);
    }
    merge(&mut merged, visible)?;
    let api_key = match &request.api_key {
        ProviderSecretWrite::Keep => old.as_ref().and_then(|old| old.api_key.clone()),
        ProviderSecretWrite::Replace { value } => {
            let key = value.trim();
            if key.is_empty() || key.len() > 8192 || key.chars().any(char::is_control) {
                return Err(INVALID);
            }
            Some(key.to_owned())
        }
        ProviderSecretWrite::Clear => None,
    };
    let new = CodexProfile {
        name: request.fields.name.clone(),
        config: merged.to_string(),
        api_key,
        preserve_login: old.as_ref().is_none_or(|old| old.preserve_login),
    };
    let new_id = identity(&request.target_host_id, id, &new).map_err(|_| INVALID)?;
    validation::endpoint(&new_id.endpoint).map_err(|_| INVALID)?;
    if matches!(request.api_key, ProviderSecretWrite::Keep) && new.api_key.is_some() {
        let old_id = identity(&request.target_host_id, id, old.as_ref().ok_or(INVALID)?)
            .map_err(|_| INVALID)?;
        if old_id.endpoint != new_id.endpoint {
            return Err(INVALID);
        }
    }
    Ok(new)
}

pub fn models(
    mappings: &[CodexModelMapping],
    prior: Option<&SavedCodexCatalog>,
) -> Option<SavedCodexCatalog> {
    (!mappings.is_empty()).then(|| SavedCodexCatalog {
        models: mappings
            .iter()
            .map(|mapping| {
                let mut model = prior
                    .and_then(|p| p.models.iter().find(|r| r.slug == mapping.slug))
                    .cloned()
                    .unwrap_or_else(|| CodexNativeModel {
                        slug: mapping.slug.clone(),
                        display_name: String::new(),
                        description: String::new(),
                        base_instructions: String::new(),
                        supports_reasoning_summaries: false,
                        supports_parallel_tool_calls: false,
                        context_window: None,
                    });
                model.display_name.clone_from(&mapping.display_name);
                model
            })
            .collect(),
    })
}

/// Validate the saved card used by the managed account transaction.
pub fn official_binding_model(row: &Value) -> std::result::Result<Option<String>, AuthErrorCode> {
    let profile: CodexProfile =
        serde_json::from_value(row.get("codex").cloned().ok_or(AuthErrorCode::Unsupported)?)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
    let config: toml::Table = profile.config.parse().map_err(|_| AuthErrorCode::Unsupported)?;
    if profile.api_key.is_some()
        || config.get("model_provider").is_some_and(|value| value.as_str() != Some("openai"))
        || config
            .get("cli_auth_credentials_store")
            .is_some_and(|value| value.as_str() != Some("file"))
        || config.keys().any(|key| {
            !matches!(key.as_str(), "model" | "model_provider" | "cli_auth_credentials_store")
        })
        || row.get("modelCatalogRevision").is_some_and(|value| !value.is_null())
        || row.get("requestOptions").is_some_and(|value| !value.is_null())
        || row.pointer("/meta/authBinding").is_some()
        || row.pointer("/meta/codexAccountId").is_some()
        || row.pointer("/meta/liveConfigManaged").is_some_and(|value| value == true)
        || row.get("settingsConfig") != Some(&settings(&profile))
    {
        return Err(AuthErrorCode::Unsupported);
    }
    config
        .get("model")
        .map(|model| {
            model
                .as_str()
                .filter(|model| {
                    !model.is_empty()
                        && model.len() <= 256
                        && !model.starts_with('-')
                        && !model.chars().any(char::is_control)
                })
                .map(str::to_owned)
                .ok_or(AuthErrorCode::Unsupported)
        })
        .transpose()
}

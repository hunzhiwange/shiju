//! Prepare scoped catalog edits for the caller's provider-record transaction.
use crate::providers::{validate_identity, validation::endpoint};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use vw_api_types::{
    id::{ModelId, TenantScope},
    model_catalog::{
        ModelCatalogDefaultChange, ModelCatalogEdit, ModelCatalogEditRequest, ModelCatalogEntry,
        ModelCatalogIdentity, ModelCatalogReadResponse, ModelCatalogWriteResponse,
        ModelMetadataDto, ScopedModelCatalogIdentity,
    },
};

const CATALOGS: &str = "deploymentModelCatalogs";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SavedCatalog {
    pub(super) revision: String,
    default_model_id: Option<ModelId>,
    pub(super) models: Vec<SavedModel>,
}

#[derive(PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SavedModel {
    pub(super) id: ModelId,
    display_alias: Option<String>,
    pub(super) metadata: ModelMetadataDto,
}

pub(super) fn deployment_key(deployment: &ModelCatalogIdentity) -> Result<String, String> {
    validate_identity(&deployment.target_host_id)?;
    validate_identity(&deployment.provider_id)?;
    endpoint(&deployment.endpoint)?;
    // Tenant, application and provider are already part of the provider record key.
    serde_json::to_string(&(&deployment.target_host_id, &deployment.endpoint))
        .map_err(|_| "invalid model catalog deployment".into())
}

pub(super) fn revision(parts: &impl Serialize) -> Result<String, String> {
    let bytes = serde_json::to_vec(parts).map_err(|_| "invalid model catalog")?;
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(bytes)))
}

pub(super) fn load(
    tenant: &TenantScope,
    deployment: &ModelCatalogIdentity,
    stored: &Value,
    key: &str,
) -> Result<SavedCatalog, String> {
    if !stored.is_object() {
        return Err("invalid stored application provider profile".into());
    }
    if let Some(catalogs) = stored.get(CATALOGS) {
        let catalogs = catalogs.as_object().ok_or("invalid stored model catalogs")?;
        if let Some(saved) = catalogs.get(key) {
            return serde_json::from_value(saved.clone())
                .map_err(|_| "invalid stored model catalog".into());
        }
    }
    Ok(SavedCatalog {
        revision: revision(&(tenant, deployment))?,
        default_model_id: None,
        models: Vec::new(),
    })
}

/// Read the complete catalog for one authorized deployment from its provider record.
pub fn read(
    tenant: &TenantScope,
    deployment: &ModelCatalogIdentity,
    stored: &Value,
) -> Result<ModelCatalogReadResponse, String> {
    let saved = load(tenant, deployment, stored, &deployment_key(deployment)?)?;
    let items = saved
        .models
        .into_iter()
        .map(|row| ModelCatalogEntry {
            identity: ScopedModelCatalogIdentity {
                scope: tenant.clone(),
                deployment: deployment.clone(),
                model_id: row.id,
            },
            display_alias: row.display_alias,
            metadata: row.metadata,
        })
        .collect();
    Ok(ModelCatalogReadResponse {
        revision: saved.revision,
        default_model_id: saved.default_model_id,
        items,
        next_cursor: None,
    })
}

/// Commit once, after validating the whole batch. The callback must conditionally
/// replace the exact provider record read by the caller, returning false on conflict.
pub fn edit(
    tenant: &TenantScope,
    stored: &Value,
    request: &ModelCatalogEditRequest,
    commit: impl FnOnce(&Value) -> Result<bool, String>,
) -> Result<ModelCatalogWriteResponse, String> {
    let deployment = &request.deployment;
    let key = deployment_key(deployment)?;
    let mut saved = load(tenant, deployment, stored, &key)?;
    if request.expected_revision != saved.revision {
        return Err("model catalog changed".into());
    }
    let mut unique = HashMap::new();
    let mut changed = false;
    for edit in &request.edits {
        let id = match edit {
            ModelCatalogEdit::Upsert { model_id, .. } | ModelCatalogEdit::Remove { model_id } => {
                model_id
            }
        };
        validate_model_id(id)?;
        if let Some(previous) = unique.insert(id, edit) {
            if previous != edit {
                return Err("conflicting edits for one model".into());
            }
            continue;
        }
        match edit {
            ModelCatalogEdit::Upsert { display_alias, metadata, .. } => {
                if display_alias
                    .as_ref()
                    .is_some_and(|alias| alias.len() > 1024 || alias.chars().any(char::is_control))
                {
                    return Err("invalid display alias".into());
                }
                let alias = display_alias
                    .as_deref()
                    .map(str::trim)
                    .filter(|alias| !alias.is_empty())
                    .map(str::to_owned);
                validate_metadata(metadata)?;
                if let Some(row) = saved.models.iter_mut().find(|row| row.id == *id) {
                    if display_alias.is_some() && row.display_alias != alias {
                        row.display_alias = alias;
                        changed = true;
                    }
                    // A selection containing only the model ID leaves saved metadata alone.
                    if **metadata != ModelMetadataDto::default() && row.metadata != **metadata {
                        row.metadata = (**metadata).clone();
                        changed = true;
                    }
                } else {
                    saved.models.push(SavedModel {
                        id: id.clone(),
                        display_alias: alias,
                        metadata: (**metadata).clone(),
                    });
                    changed = true;
                }
            }
            ModelCatalogEdit::Remove { .. } => {
                if saved.default_model_id.as_ref() == Some(id) && request.default_change.is_none() {
                    return Err("cannot remove default model".into());
                }
                if let Some(index) = saved.models.iter().position(|row| row.id == *id) {
                    saved.models.remove(index);
                    changed = true;
                }
            }
        }
    }

    if let Some(default_change) = &request.default_change {
        let default_model_id = match default_change {
            ModelCatalogDefaultChange::Set { model_id } => {
                validate_model_id(model_id)?;
                if !saved.models.iter().any(|row| row.id == *model_id) {
                    return Err("default model not found in deployment".into());
                }
                Some(model_id.clone())
            }
            ModelCatalogDefaultChange::Clear => None,
        };
        if default_model_id != saved.default_model_id {
            saved.default_model_id = default_model_id;
            changed = true;
        }
    }

    if changed {
        saved.revision = revision(&(saved.revision, &saved.default_model_id, &saved.models))?;
        save(stored, &key, &saved, commit)?;
    }
    Ok(ModelCatalogWriteResponse { revision: saved.revision, changed })
}

pub(super) fn save(
    stored: &Value,
    key: &str,
    saved: &SavedCatalog,
    commit: impl FnOnce(&Value) -> Result<bool, String>,
) -> Result<(), String> {
    let mut updated = stored.clone();
    updated[CATALOGS][key] = serde_json::to_value(saved).map_err(|_| "invalid model catalog")?;
    if !commit(&updated)? {
        return Err("model catalog changed".into());
    }
    Ok(())
}

fn validate_model_id(id: &ModelId) -> Result<(), String> {
    let id = id.as_ref();
    if id.is_empty()
        || id.len() > 256
        || id != id.trim()
        || id.starts_with('-')
        || id.chars().any(char::is_control)
    {
        return Err("invalid model ID".into());
    }
    Ok(())
}

fn validate_metadata(metadata: &ModelMetadataDto) -> Result<(), String> {
    let context = metadata.context_window;
    let output = metadata.max_output_tokens;
    if !context.into_iter().chain(output).all(valid_limit)
        || context.zip(output).is_some_and(|(context, output)| output > context)
    {
        return Err("invalid model limits".into());
    }

    for modalities in
        [&metadata.input_modalities, &metadata.output_modalities].into_iter().flatten()
    {
        if !valid_modalities(modalities) {
            return Err("invalid model modalities".into());
        }
    }

    if let Some(cost) = &metadata.cost
        && [Some(cost.input), Some(cost.output), cost.cache_read, cost.cache_write]
            .into_iter()
            .flatten()
            .any(|value| !value.is_finite() || value < 0.0)
    {
        return Err("invalid model cost".into());
    }
    Ok(())
}

// Keep explicit limits representable in the native JSON and TOML projections.
pub(super) fn valid_limit(limit: u64) -> bool {
    (1..=i64::MAX as u64).contains(&limit)
}

pub(super) fn valid_modalities(modalities: &[String]) -> bool {
    modalities.iter().enumerate().all(|(index, modality)| {
        matches!(modality.as_str(), "text" | "image" | "audio" | "video" | "pdf")
            && !modalities[..index].contains(modality)
    })
}

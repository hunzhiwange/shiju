//! Preview and commit metadata for an authorized, saved catalog row.

use super::{
    edits::{self, SavedCatalog},
    metadata::resolve_model_metadata,
};
use serde_json::Value;
use vw_api_types::{
    id::TenantScope,
    model_catalog::{
        ModelCapabilityState::Unknown, ModelCatalogFillCommitRequest,
        ModelCatalogFillPreviewRequest, ModelCatalogFillPreviewResponse, ModelCatalogIdentity,
        ModelCatalogWriteResponse, ModelMetadataDto, ModelMetadataField,
        ScopedModelCatalogIdentity,
    },
};
use vw_provider_resolver::models::ModelsSnapshot;

/// Preview only the fields that would be filled. The caller must supply the
/// authorized current deployment and its provider record, independently of the request.
pub fn preview(
    tenant: &TenantScope,
    deployment: &ModelCatalogIdentity,
    stored: &Value,
    request: &ModelCatalogFillPreviewRequest,
    snapshot: Option<&ModelsSnapshot>,
) -> Result<ModelCatalogFillPreviewResponse, String> {
    let identity = &request.identity;
    let key = target_key(tenant, deployment, identity)?;
    let mut saved = edits::load(tenant, deployment, stored, &key)?;
    let proposed = propose(&mut saved, identity, snapshot)?;
    Ok(ModelCatalogFillPreviewResponse {
        identity: identity.clone(),
        revision: saved.revision,
        proposed,
    })
}

/// Recompute against the saved row and conditionally replace its provider record.
/// The write callback must compare against the exact record supplied in `stored`.
pub fn commit(
    tenant: &TenantScope,
    deployment: &ModelCatalogIdentity,
    stored: &Value,
    request: &ModelCatalogFillCommitRequest,
    snapshot: Option<&ModelsSnapshot>,
    write: impl FnOnce(&Value) -> Result<bool, String>,
) -> Result<ModelCatalogWriteResponse, String> {
    let identity = &request.identity;
    let key = target_key(tenant, deployment, identity)?;
    let mut saved = edits::load(tenant, deployment, stored, &key)?;
    let next_revision =
        edits::revision(&("model-catalog-fill", &request.expected_revision, identity))?;
    if request.expected_revision != saved.revision {
        // The digest identifies a retry of the last successful fill for this
        // exact row. A later edit or a fill for another row invalidates it.
        if saved.revision == next_revision {
            return Ok(ModelCatalogWriteResponse { revision: saved.revision, changed: false });
        }
        return Err("model catalog changed".into());
    }

    let changed = propose(&mut saved, identity, snapshot)? != ModelMetadataDto::default();
    if changed {
        saved.revision = next_revision;
        edits::save(stored, &key, &saved, write)?;
    }
    Ok(ModelCatalogWriteResponse { revision: saved.revision, changed })
}

fn target_key(
    tenant: &TenantScope,
    deployment: &ModelCatalogIdentity,
    identity: &ScopedModelCatalogIdentity,
) -> Result<String, String> {
    if &identity.scope != tenant || &identity.deployment != deployment {
        return Err("model fill target changed".into());
    }
    edits::deployment_key(deployment)
}

fn propose(
    saved: &mut SavedCatalog,
    identity: &ScopedModelCatalogIdentity,
    snapshot: Option<&ModelsSnapshot>,
) -> Result<ModelMetadataDto, String> {
    let row = saved
        .models
        .iter_mut()
        .find(|row| row.id == identity.model_id)
        .ok_or("model not found in deployment")?;
    let deployment = &identity.deployment;
    let Some(resolved) =
        resolve_model_metadata(deployment.app_id, &deployment.endpoint, row.id.as_ref(), snapshot)
    else {
        return Ok(ModelMetadataDto::default());
    };
    Ok(fill_missing(&mut row.metadata, resolved.metadata))
}

fn fill_missing(
    current: &mut ModelMetadataDto,
    mut incoming: ModelMetadataDto,
) -> ModelMetadataDto {
    use ModelMetadataField::*;

    // The stored row wins. Skip candidate limits that would conflict with its
    // other limit, and skip modalities the saved catalog cannot represent.
    incoming.context_window = incoming.context_window.filter(|&context| {
        edits::valid_limit(context)
            && current.max_output_tokens.is_none_or(|output| output <= context)
    });
    let context = current.context_window.or(incoming.context_window);
    incoming.max_output_tokens = incoming.max_output_tokens.filter(|&output| {
        edits::valid_limit(output) && context.is_none_or(|context| output <= context)
    });
    incoming.input_modalities = incoming.input_modalities.filter(|m| edits::valid_modalities(m));
    incoming.output_modalities = incoming.output_modalities.filter(|m| edits::valid_modalities(m));

    let mut proposed = ModelMetadataDto::default();
    macro_rules! fill {
        ($name:ident, $field:ident, $blank:expr) => {
            if current.$name == $blank && incoming.$name != $blank {
                proposed.$name.clone_from(&incoming.$name);
                current.$name = incoming.$name;
                if let Some(source) = incoming.provenance.remove(&$field) {
                    current.provenance.insert($field, source.clone());
                    proposed.provenance.insert($field, source);
                }
            }
        };
    }
    fill!(context_window, ContextWindow, None);
    fill!(max_output_tokens, MaxOutputTokens, None);
    fill!(reasoning, Reasoning, Unknown);
    fill!(tool_calls, ToolCalls, Unknown);
    fill!(reasoning_efforts, ReasoningEfforts, None);
    fill!(input_modalities, InputModalities, None);
    fill!(output_modalities, OutputModalities, None);
    fill!(cost, Cost, None);
    proposed
}

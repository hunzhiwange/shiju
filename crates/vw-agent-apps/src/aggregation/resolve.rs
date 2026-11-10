//! Exact routing for model IDs from the effective aggregation publication.

use vw_api_types::{agent_apps::AppId, id::TenantScope};

use crate::providers::{
    desktop_mapping::catalog::{DesktopModel, is_reserved as is_desktop_reserved},
    live::commit::AppState,
    mode_state::observe,
};

use super::model_ids::{Decoded, ModelIdError, decode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishedModel<'a> {
    pub id: &'a str,
    pub provider_id: &'a str,
    pub upstream_model: &'a str,
}

impl<'a> From<&'a DesktopModel> for PublishedModel<'a> {
    fn from(model: &'a DesktopModel) -> Self {
        Self {
            id: &model.id,
            provider_id: &model.provider_id,
            upstream_model: &model.upstream_model,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolved<'a> {
    /// Keep the application's ordinary routing path.
    Plain,
    Exact(PublishedModel<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveError {
    UnsupportedApplication,
    InvalidModelId,
    Forbidden,
    Inactive,
    UnknownMember,
    RemovedMember,
    UnpublishedModel,
    InvalidCatalog,
}

/// Resolve against the effective publication for the caller's authorized host,
/// project and application. Supply state and ordered rows from the same settled
/// revision. Only `Plain` may use ordinary routing. A failed exact target lookup
/// or upstream request must remain a failure.
pub fn resolve<'a>(
    app: AppId,
    tenant: &TenantScope,
    state: &AppState,
    requested: &str,
    published: impl IntoIterator<Item = PublishedModel<'a>>,
) -> Result<Resolved<'a>, ResolveError> {
    let requested = if app == AppId::ClaudeDesktop { requested.trim() } else { requested };
    let decoded = match decode(app, requested) {
        Ok(Decoded::Plain) if app == AppId::ClaudeDesktop && is_desktop_reserved(requested) => {
            return Err(ResolveError::InvalidModelId);
        }
        Ok(Decoded::Plain) => return Ok(Resolved::Plain),
        Ok(decoded) => decoded,
        Err(ModelIdError::UnsupportedApplication) => {
            return Err(ResolveError::UnsupportedApplication);
        }
        Err(ModelIdError::InvalidModelId) => return Err(ResolveError::InvalidModelId),
    };

    let mode = observe(state, tenant).map_err(|_| ResolveError::Forbidden)?;
    if !mode.stack.active || !mode.attached {
        return Err(ResolveError::Inactive);
    }

    let members = &state.stack.members;
    let expected_provider = if let Decoded::Stack { key, .. } = decoded {
        let provider = state.stack.keys.get(key).ok_or(ResolveError::UnknownMember)?;
        if !members.contains(provider) {
            return Err(ResolveError::RemovedMember);
        }
        Some(provider.as_str())
    } else {
        None
    };

    let target = published
        .into_iter()
        .find(|row| match (decoded, decode(app, row.id)) {
            (
                Decoded::Stack { key, model, .. },
                Ok(Decoded::Stack { key: row_key, model: row_model, .. }),
            ) => key == row_key && model == row_model,
            (Decoded::Code(code), Ok(Decoded::Code(row_code))) => code == row_code,
            _ => false,
        })
        .ok_or(ResolveError::UnpublishedModel)?;

    if expected_provider.is_some_and(|id| id != target.provider_id)
        || target.upstream_model.trim().is_empty()
    {
        return Err(ResolveError::InvalidCatalog);
    }
    if expected_provider.is_none() && !members.iter().any(|id| id == target.provider_id) {
        return Err(ResolveError::RemovedMember);
    }
    Ok(Resolved::Exact(target))
}

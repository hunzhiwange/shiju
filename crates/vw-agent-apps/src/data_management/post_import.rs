//! Host-side post-commit refresh. Callbacks must use the existing domain owner
//! under the authorized target lock; unavailable owners must return an error.
//! This module never imports data or rolls a committed transaction back.
use vw_api_types::agent_apps::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Projection {
    ProviderLive,
    Rules,
    Prices,
    Settings,
    Usage,
}
impl Projection {
    fn identity(self) -> (DataCategory, &'static str) {
        match self {
            Self::ProviderLive => (DataCategory::Providers, "provider_live"),
            Self::Rules => (DataCategory::Rules, "rules"),
            Self::Prices => (DataCategory::Usage, "prices"),
            Self::Settings => (DataCategory::Preferences, "settings"),
            Self::Usage => (DataCategory::Usage, "usage"),
        }
    }
    fn warning(self) -> DataManagementError {
        let (category, id) = self.identity();
        DataManagementError {
            code: DataManagementErrorCode::ProjectionFailed,
            category: Some(category),
            entry_id: Some(id.into()),
        }
    }
}
const PROJECTIONS: [Projection; 5] = [
    Projection::ProviderLive, Projection::Rules, Projection::Prices,
    Projection::Settings, Projection::Usage,
];

/// Attempt every owner even after a failure. Diagnostics are fixed safe codes,
/// never raw owner errors. The result keeps committed writes explicit through
/// the enclosing progress.committed flag and Partial result contract.
pub fn refresh_after_commit(
    target: &DataManagementTarget,
    mut refresh: impl FnMut(&DataManagementTarget, Projection) -> Result<(), ()>,
) -> DataOperationResult {
    let errors = PROJECTIONS.into_iter().filter_map(|projection| {
        refresh(target, projection).err().map(|()| projection.warning())
    }).collect::<Vec<_>>();
    if errors.is_empty() { DataOperationResult::Succeeded }
    else { DataOperationResult::Partial { errors } }
}

/// Only retry the exact failed projections retained by the server. Validate the
/// entire result before any IO; this entry point has no data-commit callback.
pub fn retry_failed_projections(
    target: &DataManagementTarget,
    previous: &DataOperationResult,
    mut refresh: impl FnMut(&DataManagementTarget, Projection) -> Result<(), ()>,
) -> Result<DataOperationResult, ()> {
    let DataOperationResult::Partial { errors } = previous else { return Err(()); };
    if errors.is_empty() { return Err(()); }
    let mut pending = Vec::new();
    for error in errors {
        let projection = PROJECTIONS.into_iter().find(|p| p.warning() == *error).ok_or(())?;
        if pending.contains(&projection) { return Err(()); }
        pending.push(projection);
    }
    let errors = pending.into_iter().filter_map(|projection| {
        refresh(target, projection).err().map(|()| projection.warning())
    }).collect::<Vec<_>>();
    Ok(if errors.is_empty() { DataOperationResult::Succeeded }
        else { DataOperationResult::Partial { errors } })
}

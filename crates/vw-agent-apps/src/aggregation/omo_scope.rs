//! Project-scoped OMO selection settles in the native file transaction.
use crate::providers::live::{
    commit::{AppState, OmoSelection, PendingTarget, ProviderTransactions},
    stage::FileChange,
    write_guard::{AppWriteGuard, WriteError},
};
use vw_api_types::id::{ProjectScope, TenantScope};
pub struct ProjectOmoEdit {
    pub scope: ProjectScope,
    pub expected: AppState,
    pub selection: OmoSelection,
}
#[allow(clippy::too_many_arguments)]
pub(super) fn commit(
    project: Option<&ProjectOmoEdit>,
    transactions: &ProviderTransactions,
    tenant: &TenantScope,
    guard: &mut AppWriteGuard,
    changes: &[FileChange<'_>],
    validate: impl Fn() -> Result<(), WriteError>,
) -> Result<(), WriteError> {
    match project {
        Some(project) => {
            if project.scope.tenant_scope() != tenant {
                return Err(WriteError::Conflict);
            }
            transactions.commit_project_checked(
                &project.scope,
                guard,
                &project.expected,
                false,
                changes,
                PendingTarget { omo: Some(project.selection.clone()), ..Default::default() },
                validate,
            )
        }
        None => transactions.commit_checked(tenant, guard, changes, Default::default(), validate),
    }
}

/// Clear only the saved standard selection when its native reader confirms absence.
/// The caller additionally rechecks its authorization, catalog and revision inputs.
pub fn disable_absent(
    home: &std::path::Path,
    transactions: &ProviderTransactions,
    project: &ProjectOmoEdit,
    unchanged: impl Fn() -> bool,
) -> Result<(), WriteError> {
    if !home.is_absolute() || project.selection != OmoSelection::Disabled {
        return Err(WriteError::InvalidResource);
    }
    let absent =
        || super::omo_read::configuration_present(home, None).is_ok_and(|present| !present);
    if !absent() || !unchanged() {
        return Err(WriteError::Conflict);
    }
    let mut guard = crate::providers::live::write_guard::lock_app(
        vw_api_types::agent_apps::AppId::OpenCode,
        home,
    )?;
    commit(Some(project), transactions, project.scope.tenant_scope(), &mut guard, &[], || {
        if absent() && unchanged() { Ok(()) } else { Err(WriteError::Conflict) }
    })
}

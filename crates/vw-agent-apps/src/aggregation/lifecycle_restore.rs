//! Native lifecycle rules. The authorized executor holds the physical guard and
//! supplies the saved direct profile and frozen publication, just as for exit.
//! Gateway hooks and listener ownership remain with the existing route executor.
use super::exit_restore::{self, Restore, RestoreError};
use crate::providers::{
    claude::ClaudeFile,
    codex_catalog::CodexFiles,
    codex_login::LoginInput,
    live::commit::{Mode, ProviderTransactions},
    mode_state,
};
use vw_api_types::{agent_apps::AppId, id::ProjectScope};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleError {
    Cancelled,
    Restore(RestoreError),
}
impl From<RestoreError> for LifecycleError {
    fn from(error: RestoreError) -> Self {
        Self::Restore(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupAction {
    StayDirect,
    /// Reuse the original publisher with fresh listener credentials and saved
    /// members/default/settings. An attached bit from the old process is not
    /// proof of a live listener after restart.
    Attach,
}

/// Read only: recovery is explicit and scoped, never an incidental state read.
/// Desktop retains its files during detach, but still needs fresh route
/// publication on startup, like the source lifecycle.
pub fn startup_action(
    tx: &ProviderTransactions,
    project: &ProjectScope,
    app: AppId,
) -> Result<StartupAction, RestoreError> {
    if !matches!(app, AppId::Claude | AppId::Codex | AppId::ClaudeDesktop) {
        return Err(RestoreError::UnsupportedTarget);
    }
    let state = tx.state(app)?;
    mode_state::observe(&state, project.tenant_scope()).map_err(|_| RestoreError::Forbidden)?;
    state.require_project(project).map_err(|_| RestoreError::Forbidden)?;
    if let Some(pending) = tx.pending(app)? {
        return Err(
            if pending.tenant != *project.tenant_scope()
                || pending.target.project_scope.as_ref() != Some(project)
            {
                RestoreError::Forbidden
            } else {
                RestoreError::RecoveryPending
            },
        );
    }
    match state.mode.as_ref() {
        Some(mode) if mode.mode == Mode::Proxy && state.stack.enabled => {
            if state.pointer.is_none()
                || mode.proxy_route.as_ref().is_none_or(|id| !state.stack.members.contains(id))
            {
                return Err(RestoreError::InvalidConfiguration);
            }
            Ok(StartupAction::Attach)
        }
        Some(mode) if mode.mode == Mode::Direct && !mode.attached && !state.stack.enabled => {
            Ok(StartupAction::StayDirect)
        }
        _ => Err(RestoreError::InvalidConfiguration),
    }
}

/// Restore files/login in the SAME project transaction while preserving proxy
/// mode and the enabled stack. No follow-up state write can lose restart intent.
/// Cancellation is checked again by the original commit's pre-publication check.
pub fn detach_claude(
    file: &mut ClaudeFile,
    tx: &ProviderTransactions,
    input: Restore<'_>,
    unchanged: impl Fn() -> bool,
    cancelled: impl Fn() -> bool,
) -> Result<(), LifecycleError> {
    if cancelled() {
        return Err(LifecycleError::Cancelled);
    }
    let result = exit_restore::restore_claude_with_intent(file, tx, input, true, || {
        !cancelled() && unchanged()
    });
    result.map_err(|error| {
        if error == RestoreError::RevisionConflict && cancelled() {
            LifecycleError::Cancelled
        } else {
            LifecycleError::Restore(error)
        }
    })
}

pub fn detach_codex(
    files: &mut CodexFiles,
    tx: &ProviderTransactions,
    input: Restore<'_>,
    login: LoginInput<'_>,
    catalog: Option<&vw_api_types::model_catalog::SavedCodexCatalog>,
    unchanged: impl Fn() -> bool,
    cancelled: impl Fn() -> bool,
) -> Result<(), LifecycleError> {
    if cancelled() {
        return Err(LifecycleError::Cancelled);
    }
    let result =
        exit_restore::restore_codex_with_intent(files, tx, input, login, catalog, true, || {
            !cancelled() && unchanged()
        });
    result.map_err(|error| {
        if error == RestoreError::RevisionConflict && cancelled() {
            LifecycleError::Cancelled
        } else {
            LifecycleError::Restore(error)
        }
    })
}

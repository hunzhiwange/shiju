//! Project ownership of the existing host/application transaction slot.
use super::{
    commit::{AppState, Pending, PendingTarget, ProviderTransactions},
    stage::FileChange,
    write_guard::{AppWriteGuard, WriteError},
};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

impl AppState {
    /// Read-only authorization: legacy unbound state is never implicitly claimed.
    pub fn require_project(&self, scope: &ProjectScope) -> Result<(), WriteError> {
        if self.project_scope.as_ref() != Some(scope) {
            return Err(WriteError::Conflict);
        }
        crate::providers::mode_state::observe(self, scope.tenant_scope())
            .map_err(|_| WriteError::Conflict)?;
        Ok(())
    }
}

impl ProviderTransactions {
    pub(super) fn check_pending_project(
        &self,
        app: AppId,
        scope: Option<&ProjectScope>,
    ) -> Result<(), WriteError> {
        if self.pending(app)?.is_some_and(|pending| pending.target.project_scope.as_ref() != scope)
        {
            return Err(WriteError::Conflict);
        }
        Ok(())
    }

    /// The trusted executor must authorize the physical slot, project, application
    /// and tenant before calling. `claim_unowned` is an explicit write intent,
    /// permitted only after independently proving ownership of legacy inputs.
    /// `validate` rechecks that proof and saved revision before native publication.
    /// A pending intent requires scoped recovery and a fresh plan first.
    #[allow(clippy::too_many_arguments)]
    pub fn commit_project_checked(
        &self,
        scope: &ProjectScope,
        guard: &mut AppWriteGuard,
        expected: &AppState,
        claim_unowned: bool,
        changes: &[FileChange<'_>],
        mut target: PendingTarget,
        validate: impl Fn() -> Result<(), WriteError>,
    ) -> Result<(), WriteError> {
        if self.state(guard.app())? != *expected || self.pending(guard.app())?.is_some() {
            return Err(WriteError::Conflict);
        }
        crate::providers::mode_state::observe(expected, scope.tenant_scope())
            .map_err(|_| WriteError::Conflict)?;
        match expected.project_scope.as_ref() {
            Some(owner) if owner != scope => return Err(WriteError::Conflict),
            None if !claim_unowned => return Err(WriteError::Conflict),
            _ => {}
        }
        target.project_scope = Some(scope.clone());
        let app = guard.app();
        self.commit_checked(scope.tenant_scope(), guard, changes, target, || {
            if self.state(app)? != *expected {
                return Err(WriteError::Conflict);
            }
            validate()
        })
    }

    /// Request-triggered recovery cannot consume another project's intent.
    /// Startup recovery remains a trusted host lifecycle action and validates the
    /// durable project preimage before changing any native file.
    pub fn recover_project(
        &self,
        scope: &ProjectScope,
        guard: &mut AppWriteGuard,
    ) -> Result<Option<super::recovery::RecoveryOutcome>, WriteError> {
        let app = guard.app();
        let _lock = self.lock(app)?;
        let Some(pending) = self.pending(app)? else {
            self.state(app)?.require_project(scope)?;
            return Ok(None);
        };
        if pending.tenant != *scope.tenant_scope()
            || pending.target.project_scope.as_ref() != Some(scope)
        {
            return Err(WriteError::Conflict);
        }
        self.recover_locked(guard)
    }
}

pub(super) fn validate_update(
    state: &AppState,
    tenant: &TenantScope,
    target: &PendingTarget,
) -> Result<(), WriteError> {
    if let Some(scope) = &target.project_scope {
        if scope.tenant_scope() != tenant {
            return Err(WriteError::Conflict);
        }
        crate::providers::mode_state::observe(state, tenant).map_err(|_| WriteError::Conflict)?;
    }
    if state.project_scope.is_some() && state.project_scope != target.project_scope {
        return Err(WriteError::Conflict);
    }
    Ok(())
}

pub(super) fn validate_recovery(state: &AppState, pending: &Pending) -> Result<(), WriteError> {
    // Settlement may already have saved the target before crashing. Both that
    // state and the original binding are valid; unrelated bindings never are.
    if state.project_scope != pending.project_preimage
        && state.project_scope != pending.target.project_scope
    {
        return Err(WriteError::Conflict);
    }
    if pending.project_preimage.is_some()
        && pending.project_preimage != pending.target.project_scope
    {
        return Err(WriteError::Conflict);
    }
    validate_update(state, &pending.tenant, &pending.target)
}

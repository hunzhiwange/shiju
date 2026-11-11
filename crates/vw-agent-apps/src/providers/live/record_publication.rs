//! Saved provider participation in the existing native publication journal.
use super::{
    commit::{AppState, ExecutionHostId, PendingTarget, ProviderTransactions},
    recovery::RecoveryOutcome,
    stage::FileChange,
    write_guard::{AppWriteGuard, WriteError},
};
use serde::{Deserialize, Serialize};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

/// Private executor input. Payloads can contain credentials: never log this type.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderRecordChange {
    pub operation_id: String,
    pub host: ExecutionHostId,
    pub project: ProjectScope,
    pub app: AppId,
    pub provider_id: String,
    pub before: String,
    pub after: String,
    #[serde(default)]
    pub dependencies: Vec<ProviderRecordDelta>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderRecordDelta {
    pub provider_id: String,
    pub before: String,
    pub after: String,
}
impl ProviderRecordChange {
    pub fn records(&self) -> impl Iterator<Item = (&str, &str, &str)> {
        std::iter::once((self.provider_id.as_str(), self.before.as_str(), self.after.as_str()))
            .chain(self.dependencies.iter().map(|record| {
                (record.provider_id.as_str(), record.before.as_str(), record.after.as_str())
            }))
    }
    pub(super) fn validate(
        &self,
        host: &ExecutionHostId,
        app: AppId,
        tenant: &TenantScope,
        target: &PendingTarget,
    ) -> Result<(), WriteError> {
        if &self.host != host
            || self.app != app
            || self.project.tenant_scope() != tenant
            || target.project_scope.as_ref() != Some(&self.project)
        {
            return Err(WriteError::Conflict);
        }
        crate::providers::validate_identity(&self.operation_id)
            .map_err(|_| WriteError::InvalidResource)?;
        let mut ids = std::collections::HashSet::new();
        let mut total = 0usize;
        if self.dependencies.len() > 255 {
            return Err(WriteError::InvalidResource);
        }
        for (id, before, after) in self.records() {
            crate::providers::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
            if !ids.insert(id) {
                return Err(WriteError::InvalidResource);
            }
            for raw in [before, after] {
                total = total.saturating_add(raw.len());
                if raw.len() > 1024 * 1024
                    || total > 8 * 1024 * 1024
                    || !serde_json::from_str::<serde_json::Value>(raw)
                        .map_err(|_| WriteError::InvalidResource)?
                        .is_object()
                {
                    return Err(WriteError::InvalidResource);
                }
            }
        }
        Ok(())
    }
}

/// Implemented by the saved-record owner, using tenant-scoped conditional writes.
/// Reserve must be idempotent for exactly this intent. Finish must never replace
/// a foreign reservation or a concurrent edit; unpublished intents only unreserve.
pub trait ProviderRecordParticipant {
    fn reserve(&self, change: &ProviderRecordChange) -> Result<(), WriteError>;
    fn finish(&self, change: &ProviderRecordChange, published: bool) -> Result<(), WriteError>;
}
impl ProviderTransactions {
    #[allow(clippy::too_many_arguments)]
    pub fn commit_project_record(
        &self,
        scope: &ProjectScope,
        guard: &mut AppWriteGuard,
        expected: &AppState,
        changes: &[FileChange<'_>],
        mut target: PendingTarget,
        change: &ProviderRecordChange,
        participant: &dyn ProviderRecordParticipant,
        validate: impl Fn() -> Result<(), WriteError>,
    ) -> Result<(), WriteError> {
        expected.require_project(scope)?;
        target.project_scope = Some(scope.clone());
        change.validate(&self.host, guard.app(), scope.tenant_scope(), &target)?;
        let app = guard.app();
        self.commit_with_record(
            scope.tenant_scope(),
            guard,
            changes,
            target,
            || {
                if self.state(app)? != *expected {
                    return Err(WriteError::Conflict);
                }
                validate()
            },
            Some((change, participant)),
        )
    }

    /// Authorization is repeated by the executor before selecting this participant.
    /// Unaware legacy/startup recovery rejects version 10 rather than dropping it.
    pub fn recover_project_record(
        &self,
        scope: &ProjectScope,
        guard: &mut AppWriteGuard,
        participant: &dyn ProviderRecordParticipant,
    ) -> Result<Option<RecoveryOutcome>, WriteError> {
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
        self.recover_locked_with_record(guard, Some(participant))
    }
}

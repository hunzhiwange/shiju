//! Provider-slot orchestration. Native projection and publication stay with the
//! existing provider transaction owner on the authorized execution host.
use vw_api_types::{
    app_providers::mode_state::{ProviderMode, ProviderModeStatus},
    project_profiles::{
        ProjectProfileCategory, ProjectProfileItemResult, ProjectProfileItemStatus,
        ProjectProfilePayload,
    },
};

use super::snapshot::SnapshotScope;

/// Sanitized executor failures, including incomplete route publication. Never
/// turn a durable configuration commit with pending publication into success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderApplyFailure {
    Missing,
    Denied,
    Unsupported,
    Unavailable,
    Conflict,
    Cancelled,
    InvalidReference,
    PublicationPending,
}

impl ProviderApplyFailure {
    fn result(self, id: &str) -> ProjectProfileItemResult {
        use ProjectProfileItemStatus as Status;
        let (status, reason) = match self {
            Self::Missing => (Status::Missing, "provider_missing"),
            Self::Denied => (Status::Denied, "provider_denied"),
            Self::Unsupported => (Status::Unsupported, "provider_unsupported"),
            Self::Unavailable => (Status::Failed, "provider_unavailable"),
            Self::Conflict => (Status::Conflict, "provider_conflict"),
            Self::Cancelled => (Status::Cancelled, "provider_cancelled"),
            Self::InvalidReference => (Status::Failed, "provider_invalid_reference"),
            Self::PublicationPending => (Status::Failed, "provider_publication_pending"),
        };
        ProjectProfileItemResult {
            category: ProjectProfileCategory::Provider,
            reference_id: Some(id.to_owned()),
            status,
            reason_code: Some(reason.to_owned()),
        }
    }
}

/// Apply only the selected group's provider reference. Null means no observation
/// or write and returns None. An unchanged effective provider never calls switch.
///
/// `read` must authorize provider writes, resolve the target record in this
/// tenant/host/app (including on the no-op path), and observe effective mode via
/// providers::mode_state::observe. A dangling ID returns Missing, never an empty
/// state or local fallback. No record contents or credentials cross this boundary.
///
/// `switch` receives the observed preimage and MUST revalidate it, record version,
/// project/physical-slot ownership and permissions under the existing domain
/// locks before publishing. Direct mode uses the application's transaction/login
/// adapter; proxy mode uses route publication and MUST NOT call a direct switch,
/// detach, stop the proxy, or modify the direct pointer. Return a fresh observation
/// only after all required publication succeeds. Unsupported adapters and pending
/// publication return explicit failures. Do not acquire a physical lock here and
/// reenter an executor that acquires the same lock.
///
/// This function owns neither files nor current-profile markers. The outer profile
/// operation must serialize its prepare/apply/select lifecycle; these callbacks
/// are executor integration points, not permissions supplied by an API client.
pub fn apply_provider<R, W>(
    scope: &SnapshotScope<'_>,
    target: &ProjectProfilePayload,
    read: R,
    switch: W,
) -> Option<ProjectProfileItemResult>
where
    R: FnOnce(&SnapshotScope<'_>, &str) -> Result<ProviderModeStatus, ProviderApplyFailure>,
    W: FnOnce(
        &SnapshotScope<'_>,
        &str,
        &ProviderModeStatus,
    ) -> Result<ProviderModeStatus, ProviderApplyFailure>,
{
    let id = target.providers.get(scope.group).as_deref()?;
    let apply = || {
        if scope.target_host_id.trim().is_empty() {
            return Err(ProviderApplyFailure::Denied);
        }
        crate::providers::validate_identity(id)
            .map_err(|_| ProviderApplyFailure::InvalidReference)?;
        let before = read(scope, id)?;
        if !consistent(&before) {
            return Err(ProviderApplyFailure::Conflict);
        }
        if before.in_use_provider.as_deref() == Some(id) {
            return Ok(());
        }
        let after = switch(scope, id, &before)?;
        // A callback claiming success cannot disguise a mode change, an altered
        // inactive pointer, a detached proxy, or a replaced aggregation stack.
        let inactive_preserved = match before.mode {
            ProviderMode::Direct => after.proxy_route == before.proxy_route,
            ProviderMode::Proxy => after.direct_provider == before.direct_provider,
        };
        if !consistent(&after)
            || after.mode != before.mode
            || after.attached != before.attached
            || after.stack != before.stack
            || !inactive_preserved
            || after.in_use_provider.as_deref() != Some(id)
        {
            return Err(ProviderApplyFailure::Conflict);
        }
        Ok(())
    };
    Some(match apply() {
        Ok(()) => ProjectProfileItemResult {
            category: ProjectProfileCategory::Provider,
            reference_id: Some(id.to_owned()),
            status: ProjectProfileItemStatus::Succeeded,
            reason_code: None,
        },
        Err(reason) => reason.result(id),
    })
}

fn consistent(state: &ProviderModeStatus) -> bool {
    match state.mode {
        ProviderMode::Direct => !state.attached && state.in_use_provider == state.direct_provider,
        ProviderMode::Proxy => state.in_use_provider == state.proxy_route,
    }
}

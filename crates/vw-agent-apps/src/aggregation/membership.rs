//! Host-local persistence for planned aggregation members and the default route.

#[cfg(test)]
#[path = "default_provider_tests.rs"]
mod default_provider_tests;

use super::{
    member_keys::allocate_key,
    member_validation::{
        MemberValidationError, ProviderRole, validate_member_removal, validate_provider,
    },
};
use crate::providers::{
    live::{
        commit::{AppState, Mode, ModeState, PendingTarget, ProviderTransactions},
        write_guard::{AppWriteGuard, WriteError},
    },
    mode_state::observe,
    validate_identity,
};
use vw_api_types::{agent_apps_auth::AuthService, id::TenantScope};

/// Metadata resolved from a saved provider in the authorized application scope.
pub struct SavedMember<'a> {
    pub name: &'a str,
    pub icon: Option<&'a str>,
    pub official_service: Option<AuthService>,
}

pub enum MembershipChange<'a> {
    Add { provider_id: &'a str, saved: Option<SavedMember<'a>> },
    Remove { provider_id: &'a str },
    SetDefault { provider_id: &'a str, saved: Option<SavedMember<'a>> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipError {
    Validation(MemberValidationError),
    Forbidden,
    NotFound,
    UnsupportedTarget,
    RevisionConflict,
    RecoveryPending,
    TargetFailed,
}

impl From<MemberValidationError> for MembershipError {
    fn from(error: MemberValidationError) -> Self {
        Self::Validation(error)
    }
}

impl From<WriteError> for MembershipError {
    fn from(error: WriteError) -> Self {
        match error {
            WriteError::Conflict => Self::RevisionConflict,
            WriteError::Recovered => Self::RecoveryPending,
            WriteError::InvalidResource | WriteError::Io => Self::TargetFailed,
        }
    }
}

/// The executor authorizes the tenant, host, project and application before
/// taking the application guard. Supply the state from the requested revision
/// and recheck the other revision inputs (including scope and saved provider)
/// in `unchanged`. Returns true when the membership or default route was saved.
pub fn set_member(
    transactions: &ProviderTransactions,
    guard: &mut AppWriteGuard,
    tenant: &TenantScope,
    expected: &AppState,
    change: MembershipChange<'_>,
    unchanged: impl Fn() -> bool,
) -> Result<bool, MembershipError> {
    set_member_inner(transactions, guard, tenant, None, expected, change, unchanged)
}

/// Project-bound callers must present the persisted project owner. The legacy
/// tenant-only entry remains unable to change a claimed physical slot.
pub fn set_member_project(
    transactions: &ProviderTransactions,
    guard: &mut AppWriteGuard,
    project: &vw_api_types::id::ProjectScope,
    expected: &AppState,
    change: MembershipChange<'_>,
    unchanged: impl Fn() -> bool,
) -> Result<bool, MembershipError> {
    expected.require_project(project).map_err(|_| MembershipError::Forbidden)?;
    set_member_inner(
        transactions,
        guard,
        project.tenant_scope(),
        Some(project),
        expected,
        change,
        unchanged,
    )
}

#[allow(clippy::too_many_arguments)]
fn set_member_inner(
    transactions: &ProviderTransactions,
    guard: &mut AppWriteGuard,
    tenant: &TenantScope,
    project: Option<&vw_api_types::id::ProjectScope>,
    expected: &AppState,
    change: MembershipChange<'_>,
    unchanged: impl Fn() -> bool,
) -> Result<bool, MembershipError> {
    let app = guard.app();
    validate_provider(app, None, ProviderRole::Member)?;
    let current = transactions.state(app)?;
    observe(&current, tenant).map_err(|_| MembershipError::Forbidden)?;
    if let Some(pending) = transactions.pending(app)? {
        return Err(if &pending.tenant == tenant {
            MembershipError::RecoveryPending
        } else {
            MembershipError::Forbidden
        });
    }

    if current != *expected || !unchanged() {
        return Err(MembershipError::RevisionConflict);
    }

    let check_revision = || {
        if transactions.state(app)? != *expected || !unchanged() {
            return Err(WriteError::Conflict);
        }
        Ok(())
    };

    let mut target = plan_member(app, &current, change)?;
    if target.stack.is_none() && target.mode.is_none() {
        return Ok(false);
    }
    target.project_scope = project.cloned();

    // Attached routes need coordinated publication with their live client projection.
    if current.mode.as_ref().is_some_and(|active| {
        active.attached
            && (target.mode.is_some() || current.stack.enabled && active.mode == Mode::Proxy)
    }) {
        return Err(MembershipError::UnsupportedTarget);
    }

    transactions.commit_checked(tenant, guard, &[], target, check_revision)?;
    Ok(true)
}

/// Pure intent planning. Publication owners can include this target and native
/// files in one scoped transaction without saving an intermediate member list.
pub fn plan_member(
    app: vw_api_types::agent_apps::AppId,
    current: &AppState,
    change: MembershipChange<'_>,
) -> Result<PendingTarget, MembershipError> {
    validate_provider(app, None, ProviderRole::Member)?;
    let mut stack = current.stack.clone();
    let mode = match change {
        MembershipChange::Add { provider_id, saved } => {
            validate_identity(provider_id).map_err(|_| MemberValidationError::InvalidMember)?;
            let saved = saved.ok_or(MemberValidationError::InvalidMember)?;
            validate_provider(app, saved.official_service, ProviderRole::Member)?;
            allocate_key(&mut stack, provider_id, saved.name, saved.icon);
            if !stack.members.iter().any(|id| id == provider_id) {
                stack.members.push(provider_id.into());
            }
            None
        }
        MembershipChange::Remove { provider_id } => {
            validate_identity(provider_id).map_err(|_| MemberValidationError::InvalidMember)?;
            validate_member_removal(app, current, provider_id)?;
            stack.members.retain(|id| id != provider_id);
            None
        }
        MembershipChange::SetDefault { provider_id, saved } => {
            validate_identity(provider_id).map_err(|_| MemberValidationError::InvalidMember)?;
            let saved = saved.ok_or(MembershipError::NotFound)?;
            validate_provider(app, saved.official_service, ProviderRole::Default)?;
            if saved.official_service.is_some() {
                // Codex official defaults need verified native login and recovery.
                return Err(MembershipError::UnsupportedTarget);
            }
            allocate_key(&mut stack, provider_id, saved.name, saved.icon);
            if !stack.members.iter().any(|id| id == provider_id) {
                stack.members.push(provider_id.into());
            }
            let mut mode = current.mode.clone().unwrap_or(ModeState {
                mode: Mode::Direct,
                attached: false,
                proxy_route: None,
                contract: None,
            });
            if mode.proxy_route.as_deref() == Some(provider_id) {
                None
            } else {
                mode.proxy_route = Some(provider_id.into());
                Some(mode)
            }
        }
    };
    let stack = (stack != current.stack).then_some(stack);
    Ok(PendingTarget { stack, mode, ..Default::default() })
}

//! Host-local persistence for the planned aggregation membership.

use super::{
    member_keys::allocate_key,
    member_validation::{
        MemberValidationError, ProviderRole, validate_member_removal, validate_provider,
    },
};
use crate::providers::{
    live::{
        commit::{AppState, Mode, PendingTarget, ProviderTransactions},
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipError {
    Validation(MemberValidationError),
    Forbidden,
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
/// in `unchanged`. Returns true when the membership state was saved.
pub fn set_member(
    transactions: &ProviderTransactions,
    guard: &mut AppWriteGuard,
    tenant: &TenantScope,
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

    let mut stack = current.stack.clone();
    match change {
        MembershipChange::Add { provider_id, saved } => {
            validate_identity(provider_id).map_err(|_| MemberValidationError::InvalidMember)?;
            let saved = saved.ok_or(MemberValidationError::InvalidMember)?;
            validate_provider(app, saved.official_service, ProviderRole::Member)?;
            allocate_key(&mut stack, provider_id, saved.name, saved.icon);
            if !stack.members.iter().any(|id| id == provider_id) {
                stack.members.push(provider_id.into());
            }
        }
        MembershipChange::Remove { provider_id } => {
            validate_identity(provider_id).map_err(|_| MemberValidationError::InvalidMember)?;
            validate_member_removal(app, &current, provider_id)?;
            stack.members.retain(|id| id != provider_id);
        }
    }
    if stack == current.stack {
        return Ok(false);
    }

    // An attached aggregation must settle membership and its client projection together.
    if current.stack.enabled
        && current.mode.as_ref().is_some_and(|mode| mode.mode == Mode::Proxy && mode.attached)
    {
        return Err(MembershipError::UnsupportedTarget);
    }

    transactions.commit_checked(
        tenant,
        guard,
        &[],
        PendingTarget { stack: Some(stack), ..Default::default() },
        check_revision,
    )?;
    Ok(true)
}

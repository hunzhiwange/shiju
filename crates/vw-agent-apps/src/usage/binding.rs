//! Resolve a provider's usage credential within its authorized execution scope.

use crate::{auth::flow::AuthAccountScope, providers::validate_identity};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthService},
};

#[derive(Clone, Copy)]
pub enum UnboundUsageAccount {
    ExplicitOnly,
    ManagedDefault,
    /// Slot ownership comes from the execution host's policy.
    NativeLogin {
        is_owner: bool,
    },
}

#[derive(Clone, Copy)]
pub struct UsageAccountSelection<'a> {
    pub app: AppId,
    pub account_id: Option<&'a str>,
    /// Read from the authorized provider record for the selected service.
    pub bound_account_id: Option<&'a str>,
    /// Chosen by the adapter for this provider, independently of request options.
    pub unbound: UnboundUsageAccount,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UsageAccount {
    Managed(String),
    NativeLogin,
}

/// Host-internal locator. The query adapter acquires the selected credential
/// through the corresponding host service for the lifetime of its query.
pub struct UsageCredentialRef<'a> {
    scope: AuthAccountScope<'a>,
    account: UsageAccount,
}

impl<'a> UsageCredentialRef<'a> {
    pub fn scope(&self) -> AuthAccountScope<'a> {
        self.scope
    }

    pub fn account(&self) -> &UsageAccount {
        &self.account
    }
}

/// Resolve a provider query after the caller has authorized the project, provider
/// and credential use on a reachable host. The lookup functions use the supplied
/// tenant, principal, host and service to read the existing account store.
pub fn resolve_usage_credential<'a>(
    requested: AuthAccountScope<'_>,
    authorized_host: AuthAccountScope<'a>,
    selection: UsageAccountSelection<'_>,
    load_default: impl FnOnce(AuthAccountScope<'a>) -> Result<Option<String>, AuthErrorCode>,
    verify_managed: impl FnOnce(AuthAccountScope<'a>, &str) -> Result<(), AuthErrorCode>,
) -> Result<UsageCredentialRef<'a>, AuthErrorCode> {
    use AuthErrorCode::{Conflict, Forbidden, InvalidInput, NotFound};

    if requested != authorized_host {
        return Err(Forbidden);
    }
    authorized_host.validate()?;

    let UsageAccountSelection { app, account_id, bound_account_id, unbound } = selection;
    for id in [account_id, bound_account_id].into_iter().flatten() {
        validate_identity(id).map_err(|_| InvalidInput)?;
    }
    if let (Some(account_id), Some(bound)) = (account_id, bound_account_id)
        && account_id != bound
    {
        return Err(Conflict);
    }
    if authorized_host.service == AuthService::AgyOauth && bound_account_id.is_none() {
        return Err(NotFound);
    }

    let id = match (bound_account_id.or(account_id), unbound) {
        (Some(id), _) => id.to_owned(),
        (None, UnboundUsageAccount::NativeLogin { is_owner: true }) => {
            return Ok(UsageCredentialRef {
                scope: authorized_host,
                account: UsageAccount::NativeLogin,
            });
        }
        (None, UnboundUsageAccount::NativeLogin { is_owner: false }) => return Err(Forbidden),
        (None, UnboundUsageAccount::ManagedDefault) if app != AppId::Codex => {
            let id = load_default(authorized_host)?.ok_or(NotFound)?;
            validate_identity(&id).map_err(|_| InvalidInput)?;
            id
        }
        (None, _) => return Err(NotFound),
    };
    verify_managed(authorized_host, &id)?;
    Ok(UsageCredentialRef { scope: authorized_host, account: UsageAccount::Managed(id) })
}

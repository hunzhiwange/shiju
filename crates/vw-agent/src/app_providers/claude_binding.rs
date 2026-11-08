//! Scoped official Claude references in the existing supplier catalog.
use super::{AppId, AppProviderStore, NAMESPACE, Path};
use crate::agent_apps::auth_accounts::AuthAccountScope;
use serde_json::Value;
use vw_agent_apps::{
    auth::credential_lease::CredentialLease,
    providers::live::commit::{ClaudeManagedLogin, ExecutionHostId, ProviderTransactions},
};
use vw_api_types::{
    agent_apps_auth::{AuthErrorCode, AuthService},
    app_providers::claude::{ClaudeConfig, ClaudeProfile},
};

const FIELD: &str = "managedClaudeLogin";

/// Paths supplied together by the authorized execution host policy.
pub struct ClaudeBindingHost<'a> {
    pub home: &'a Path,
    pub private_root: &'a Path,
}

impl AppProviderStore {
    /// Update a saved official card under the same directory and account leases as checkout.
    /// An active card must first be switched away from before its reference is edited.
    pub fn bind_claude_account(
        &self,
        scope: AuthAccountScope<'_>,
        host: ClaudeBindingHost<'_>,
        provider_id: &str,
        expected_account: Option<&str>,
        account_id: Option<&str>,
    ) -> Result<(), AuthErrorCode> {
        let (tx, _guard) = host.lock(scope)?;
        host.check_idle(&tx)?;
        let key = Self::key(AppId::Claude, provider_id).map_err(|_| AuthErrorCode::InvalidInput)?;
        let raw = self
            .settings
            .get(scope.tenant, NAMESPACE, &key)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
            .ok_or(AuthErrorCode::NotFound)?;
        let mut row: Value =
            serde_json::from_str(&raw).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        official(&row)?;
        let current = scoped_binding(&row, scope)?;
        let current_id = current.as_ref().map(|b| b.account_id.as_str());
        if current_id == account_id {
            return Ok(());
        }
        if current_id != expected_account {
            return Err(AuthErrorCode::Conflict);
        }
        let state = tx.state(AppId::Claude).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        if state.pointer.as_ref().is_some_and(|p| &p.tenant != scope.tenant || p.id == provider_id)
            || state.mode_tenant.as_ref().is_some_and(|tenant| tenant != scope.tenant)
        {
            return Err(AuthErrorCode::Conflict);
        }

        let accounts = self
            .claude_accounts(host.private_root)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let _old_lease =
            current_id.map(|id| accounts.managed_official_lease(scope, id)).transpose()?;
        let _new_lease = if let Some(id) = account_id {
            let lease = accounts.managed_official_lease(scope, id)?;
            // A valid directory marker is required, even though actual checkout re-verifies OAuth.
            accounts.claude_managed_directory(&lease)?;
            let reference = ClaudeManagedLogin {
                tenant: scope.tenant.clone(),
                principal: scope.principal.into(),
                host: scope.host.into(),
                account_id: id.into(),
            };
            row[FIELD] =
                serde_json::to_value(reference).map_err(|_| AuthErrorCode::StorageUnavailable)?;
            Some(lease)
        } else {
            row.as_object_mut().ok_or(AuthErrorCode::StorageUnavailable)?.remove(FIELD);
            None
        };
        if !self
            .settings
            .compare_exchange(scope.tenant, NAMESPACE, &key, &raw, &row.to_string())
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
        {
            return Err(AuthErrorCode::Conflict);
        }
        Ok(())
    }

    pub fn claude_account_binding(
        &self,
        scope: AuthAccountScope<'_>,
        provider_id: &str,
    ) -> Result<Option<String>, AuthErrorCode> {
        check_scope(scope)?;
        let row = self
            .claude_record(scope.tenant, provider_id)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
            .ok_or(AuthErrorCode::NotFound)?;
        Ok(scoped_binding(&row, scope)?.map(|binding| binding.account_id))
    }

    /// Run a caller-owned account mutation after checking all references, including
    /// cards invisible to that caller and the independently retained native login.
    /// The callback uses the supplied lease and must not reacquire either lock.
    pub fn with_claude_provider_unreferenced<T>(
        &self,
        scope: AuthAccountScope<'_>,
        host: ClaudeBindingHost<'_>,
        account_id: &str,
        mutate: impl FnOnce(&CredentialLease) -> Result<T, AuthErrorCode>,
    ) -> Result<T, AuthErrorCode> {
        let (tx, _guard) = host.lock(scope)?;
        host.check_idle(&tx)?;
        let accounts = self
            .claude_accounts(host.private_root)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let lease = accounts.managed_official_lease(scope, account_id)?;
        let matches = |binding: &ClaudeManagedLogin| {
            same_scope(binding, scope) && binding.account_id == account_id
        };
        if tx
            .state(AppId::Claude)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
            .claude_managed_login
            .as_ref()
            .is_some_and(matches)
        {
            return Err(AuthErrorCode::AccountInUse);
        }
        for row in self
            .settings
            .list(scope.tenant, NAMESPACE)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
        {
            if !row.config_key.starts_with("\"claude\":") {
                continue;
            }
            let row: Value = serde_json::from_str(&row.payload_json)
                .map_err(|_| AuthErrorCode::StorageUnavailable)?;
            if managed_claude_binding(&row)?.as_ref().is_some_and(matches) {
                return Err(AuthErrorCode::AccountInUse);
            }
        }
        mutate(&lease)
    }
}

impl ClaudeBindingHost<'_> {
    fn lock(
        &self,
        scope: AuthAccountScope<'_>,
    ) -> Result<(ProviderTransactions, std::fs::File), AuthErrorCode> {
        check_scope(scope)?;
        if !self.home.is_absolute() {
            return Err(AuthErrorCode::InvalidInput);
        }
        let host = ExecutionHostId::new(scope.host).map_err(|_| AuthErrorCode::InvalidInput)?;
        let tx = ProviderTransactions::new(self.private_root, host)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let guard = tx.lock_claude_directory().map_err(|_| AuthErrorCode::StorageUnavailable)?;
        Ok((tx, guard))
    }

    fn check_idle(&self, tx: &ProviderTransactions) -> Result<(), AuthErrorCode> {
        if tx.pending(AppId::Claude).map_err(|_| AuthErrorCode::StorageUnavailable)?.is_some() {
            return Err(AuthErrorCode::Conflict);
        }
        match self.home.join(".claude/.shiju-login.json").symlink_metadata() {
            Ok(_) => Err(AuthErrorCode::Conflict),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(AuthErrorCode::StorageUnavailable),
        }
    }
}

pub(crate) fn managed_claude_binding(
    row: &Value,
) -> Result<Option<ClaudeManagedLogin>, AuthErrorCode> {
    row.get(FIELD)
        .map(|raw| {
            official(row)?;
            serde_json::from_value(raw.clone()).map_err(|_| AuthErrorCode::StorageUnavailable)
        })
        .transpose()
}

fn scoped_binding(
    row: &Value,
    scope: AuthAccountScope<'_>,
) -> Result<Option<ClaudeManagedLogin>, AuthErrorCode> {
    let binding = managed_claude_binding(row)?;
    if binding.as_ref().is_some_and(|binding| !same_scope(binding, scope)) {
        return Err(AuthErrorCode::Forbidden);
    }
    Ok(binding)
}

pub(crate) fn same_scope(binding: &ClaudeManagedLogin, scope: AuthAccountScope<'_>) -> bool {
    &binding.tenant == scope.tenant
        && binding.host == scope.host
        && binding.principal == scope.principal
}

fn official(row: &Value) -> Result<(), AuthErrorCode> {
    let profile: ClaudeProfile =
        serde_json::from_value(row.get("claude").cloned().ok_or(AuthErrorCode::Unsupported)?)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
    if !matches!(profile.config, ClaudeConfig::Official { account: None })
        || row.pointer("/meta/authBinding").is_some()
        || row.pointer("/meta/claudeAccountId").is_some()
    {
        return Err(AuthErrorCode::Unsupported);
    }
    Ok(())
}

fn check_scope(scope: AuthAccountScope<'_>) -> Result<(), AuthErrorCode> {
    scope.validate()?;
    if scope.service != AuthService::ClaudeOfficial {
        return Err(AuthErrorCode::Unsupported);
    }
    Ok(())
}

pub(super) fn preserve_managed_binding(
    stored: Option<&Value>,
    profile: &ClaudeProfile,
    row: &mut Value,
) -> Result<(), String> {
    if let Some(reference) = stored.and_then(|row| row.get(FIELD)) {
        if !matches!(profile.config, ClaudeConfig::Official { account: None }) {
            return Err(
                "Claude managed binding must be removed before replacing the official profile"
                    .into(),
            );
        }
        row[FIELD] = reference.clone();
    }
    Ok(())
}

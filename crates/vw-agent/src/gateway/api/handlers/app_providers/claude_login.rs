//! Bound native login orchestration for the existing Claude transaction.
use super::*;
use crate::{
    agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore},
    app_providers::{claude_binding_matches, managed_claude_binding},
};
use vw_agent_apps::{
    auth::claude_keychain::ClaudeKeychain,
    providers::{
        claude_login::LoginMaterial,
        live::{commit::ClaudeManagedLogin, write_guard::WriteError},
    },
};
use vw_api_types::agent_apps_auth::AuthService;
impl super::Host {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply(
        mut self,
        store: &AppProviderStore,
        tenant: &TenantScope,
        (principal, owns_slot): (&str, Option<bool>),
        id: &str,
        row: &serde_json::Value,
        activation: ClaudeActivation,
        keychain: &dyn vw_agent_apps::auth::claude_keychain::ClaudeKeychain,
    ) -> ClaudeLiveResult {
        let current = self.state.pointer.as_ref().map(|p| p.id.as_str());
        let should = match activation {
            ClaudeActivation::CatalogOnly => false,
            ClaudeActivation::First => current.is_none(),
            ClaudeActivation::IfCurrent => current == Some(id),
            ClaudeActivation::Switch => true,
        };
        if !should {
            return ClaudeLiveResult::NotAttempted;
        }
        let previous = self.previous.as_ref().and_then(|r| r.get("settingsConfig"));
        if store.claude_record(tenant, id).ok().flatten().as_ref() != Some(row) {
            return ClaudeLiveResult::Failed;
        }
        let Some(target) = row.get("settingsConfig") else {
            return ClaudeLiveResult::Failed;
        };
        if row.pointer("/meta/authBinding").is_some()
            || row.pointer("/meta/claudeAccountId").is_some()
        {
            return ClaudeLiveResult::Failed;
        }
        let profile: Option<ClaudeProfile> = match row.get("claude") {
            Some(value) => match serde_json::from_value(value.clone()) {
                Ok(profile) => Some(profile),
                Err(_) => return ClaudeLiveResult::Failed,
            },
            None => None,
        };
        let scope = AuthAccountScope {
            tenant,
            principal,
            host: &self.host_id,
            service: AuthService::ClaudeOfficial,
        };
        let Ok(managed) = managed_claude_binding(row) else {
            return ClaudeLiveResult::Failed;
        };
        let previous_owner = self.state.claude_managed_login.as_ref();
        if owns_slot == Some(false)
            || (managed.is_some() || previous_owner.is_some()) && owns_slot != Some(true)
        {
            return ClaudeLiveResult::Failed;
        }
        if managed
            .as_ref()
            .into_iter()
            .chain(previous_owner)
            .any(|binding| !claude_binding_matches(binding, scope))
        {
            return ClaudeLiveResult::Failed;
        }
        let legacy = match profile {
            Some(ClaudeProfile { config: ClaudeConfig::Official { account }, .. }) => account,
            _ => None,
        };
        if legacy.is_some() && previous_owner.is_some() {
            return ClaudeLiveResult::Failed;
        }

        if self.login.as_ref().is_some_and(|login| login.recover(&self.tx, keychain).is_err()) {
            return ClaudeLiveResult::RecoveryRequired;
        }
        let mut operation = None;
        let mut account_lease = None;
        if managed.is_some() || legacy.is_some() {
            let Some(login) = &self.login else {
                return ClaudeLiveResult::Failed;
            };
            let Ok(accounts) = store.claude_accounts(&self.root) else {
                return ClaudeLiveResult::Failed;
            };
            let checkout = if let Some(binding) = &managed {
                let Ok(lease) = accounts.managed_official_lease(scope, &binding.account_id) else {
                    return ClaudeLiveResult::Failed;
                };
                let Ok(directory) = accounts.claude_managed_directory(&lease) else {
                    return ClaudeLiveResult::Failed;
                };
                let Ok(metadata) = accounts.read_metadata(&scope, &binding.account_id) else {
                    return ClaudeLiveResult::Failed;
                };
                account_lease = Some(lease);
                login.begin_managed(
                    &self.tx,
                    tenant,
                    directory.path(),
                    keychain,
                    metadata.external_identity_sha256,
                    |material| {
                        archive_managed_previous(
                            &accounts,
                            scope,
                            previous_owner,
                            binding,
                            material,
                            keychain,
                        )
                    },
                )
            } else if let Some(binding) = legacy {
                let Ok((lease, directory)) =
                    accounts.claude_login_account(tenant, &self.host_id, &binding.account_id)
                else {
                    return ClaudeLiveResult::Failed;
                };
                account_lease = Some(lease);
                login.begin(&self.tx, tenant, directory.path(), keychain, |material| {
                    accounts.archive_claude_default(tenant, &self.host_id, material, keychain)
                })
            } else {
                return ClaudeLiveResult::Failed;
            };
            match checkout {
                Ok(op) => operation = op,
                Err(_) => {
                    return if login.has_journal().unwrap_or(true) {
                        ClaudeLiveResult::RecoveryRequired
                    } else {
                        ClaudeLiveResult::Failed
                    };
                }
            }
        }
        let pointer =
            if activation == ClaudeActivation::IfCurrent { None } else { Some(id.to_owned()) };
        let unchanged = || store.claude_record(tenant, id).ok().flatten().as_ref() == Some(row);
        let result = self.file.apply_with_managed_login(
            &self.tx,
            tenant,
            &self.state,
            previous,
            target,
            pointer,
            unchanged,
            operation,
            managed,
        );
        let settled = self.login.as_ref().map(|login| login.settle(&self.tx, keychain)).transpose();
        drop(account_lease);
        if settled.is_err() {
            return ClaudeLiveResult::RecoveryRequired;
        }
        match result {
            Ok(()) => ClaudeLiveResult::Applied,
            Err(_) if self.tx.pending(AppId::Claude).ok().flatten().is_some() => {
                ClaudeLiveResult::RecoveryRequired
            }
            Err(_) => ClaudeLiveResult::Failed,
        }
    }
}

fn archive_managed_previous(
    accounts: &AuthAccountStore,
    scope: AuthAccountScope<'_>,
    previous: Option<&ClaudeManagedLogin>,
    target: &ClaudeManagedLogin,
    material: &LoginMaterial,
    keychain: &dyn ClaudeKeychain,
) -> Result<(), WriteError> {
    use sha2::{Digest, Sha256};
    let Some(identity) = material.identity()? else {
        return Ok(());
    };
    let previous = previous
        .filter(|previous| previous.account_id != target.account_id)
        .ok_or(WriteError::Conflict)?;
    let lease = accounts
        .managed_official_lease(scope, &previous.account_id)
        .map_err(|_| WriteError::Conflict)?;
    let metadata =
        accounts.read_metadata(&scope, &previous.account_id).map_err(|_| WriteError::Conflict)?;
    let actual: [u8; 32] = Sha256::digest(identity.as_bytes()).into();
    if actual != metadata.external_identity_sha256 {
        return Err(WriteError::Conflict);
    }
    let directory = accounts.claude_managed_directory(&lease).map_err(|_| WriteError::Conflict)?;
    material.archive(directory.path(), keychain)
}

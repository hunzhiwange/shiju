//! Bound native login orchestration for the existing Claude transaction.
use super::*;
impl super::Host {
    pub(super) fn apply(
        mut self,
        store: &AppProviderStore,
        tenant: &TenantScope,
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
        if self.login.as_ref().is_some_and(|login| login.recover(&self.tx, keychain).is_err()) {
            return ClaudeLiveResult::RecoveryRequired;
        }
        let profile: Option<ClaudeProfile> = match row.get("claude") {
            Some(value) => match serde_json::from_value(value.clone()) {
                Ok(profile) => Some(profile),
                Err(_) => return ClaudeLiveResult::Failed,
            },
            None => None,
        };
        let mut operation = None;
        let mut account_lease = None;
        if let Some(ClaudeProfile {
            config: ClaudeConfig::Official { account: Some(binding) },
            ..
        }) = profile
        {
            let Some(login) = &self.login else {
                return ClaudeLiveResult::Failed;
            };
            let Ok(accounts) = store.claude_accounts(&self.root) else {
                return ClaudeLiveResult::Failed;
            };
            let Ok((lease, directory)) =
                accounts.claude_login_account(tenant, &self.host_id, &binding.account_id)
            else {
                return ClaudeLiveResult::Failed;
            };
            account_lease = Some(lease);
            match login.begin(&self.tx, tenant, directory.path(), keychain, |material| {
                accounts.archive_claude_default(tenant, &self.host_id, material, keychain)
            }) {
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
        let result = self.file.apply_with_login(
            &self.tx,
            tenant,
            &self.state,
            previous,
            target,
            pointer,
            unchanged,
            operation,
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

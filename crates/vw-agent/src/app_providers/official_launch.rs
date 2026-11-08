//! Isolated execution of an explicitly scoped Claude or Codex supplier account.
use super::*;
use crate::agent_apps::auth_accounts::{AuthAccountLaunch, AuthAccountScope, AuthAccountStore};
use vw_agent_apps::apps::launch::request::LaunchMaterial;
use vw_api_types::agent_apps_auth::{AuthErrorCode, AuthService};

impl AppProviderStore {
    pub(crate) fn prepare_bound_official_launch(
        &self,
        scope: AuthAccountScope<'_>,
        provider_id: &str,
        root: Option<&Path>,
        material: LaunchMaterial,
    ) -> Result<Option<AuthAccountLaunch>, AuthErrorCode> {
        let read = || match scope.service {
            AuthService::ClaudeOfficial => self.claude_record(scope.tenant, provider_id),
            AuthService::CodexOauth => self.codex_record(scope.tenant, provider_id),
            _ => Err("unsupported official provider".into()),
        };
        let Some(row) = read().map_err(|_| AuthErrorCode::StorageUnavailable)? else {
            return Ok(None);
        };
        let resolved = match scope.service {
            AuthService::ClaudeOfficial => {
                if let Some(binding) = managed_claude_binding(&row)? {
                    if !claude_binding_matches(&binding, scope) {
                        return Err(AuthErrorCode::Forbidden);
                    }
                    let model = row
                        .pointer("/claude/model")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned);
                    Some((binding.account_id, model))
                } else {
                    None
                }
            }
            AuthService::CodexOauth => super::codex_binding::account(&row, scope)?,
            _ => return Err(AuthErrorCode::Unsupported),
        };
        let Some((account, model)) = resolved else {
            return Ok(None);
        };
        let root = root.ok_or(AuthErrorCode::ExecutionHostUnavailable)?;
        let accounts = AuthAccountStore::new(self.database.clone(), root)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        accounts
            .prepare_bound_official_launch(
                scope,
                &account,
                model.as_deref(),
                material,
                || matches!(read(), Ok(Some(latest)) if latest == row),
            )
            .map(Some)
    }
}

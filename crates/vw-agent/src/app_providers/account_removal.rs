//! Remove an authorized account under its provider and credential guards.

use super::{AppProviderStore, ClaudeBindingHost, Path};
use crate::agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore};
use vw_agent_apps::auth::{
    agy_account::{AgyNativeTarget, remove_managed_account},
    dsh_account::DshAccount,
};
use vw_api_types::agent_apps_auth::{AuthErrorCode, AuthService};

/// The execution host supplies any native target from trusted host policy.
#[derive(Clone, Copy)]
pub enum NativeAccountRemoval<'a> {
    Claude { home: &'a Path },
    Agy(AgyNativeTarget<'a>),
    Dsh(DshAccount<'a>),
}

impl AppProviderStore {
    /// Delete only the selected managed account. Run this blocking operation off the async runtime.
    pub fn remove_auth_account(
        &self,
        scope: AuthAccountScope<'_>,
        private_root: &Path,
        account_id: &str,
        native: Option<NativeAccountRemoval<'_>>,
    ) -> Result<(), AuthErrorCode> {
        scope.validate()?;
        vw_agent_apps::providers::validate_identity(account_id)
            .map_err(|_| AuthErrorCode::InvalidInput)?;

        match (scope.service, native) {
            (AuthService::AgyOauth, Some(NativeAccountRemoval::Agy(target))) => {
                return remove_managed_account(scope, account_id, target, || {
                    self.check_agy_references(scope, account_id)
                });
            }
            (AuthService::DshAccount, Some(NativeAccountRemoval::Dsh(target))) => {
                return target.remove_account(scope, account_id);
            }
            (AuthService::AgyOauth | AuthService::DshAccount, None) => {
                return Err(AuthErrorCode::Forbidden);
            }
            (_, Some(NativeAccountRemoval::Agy(_) | NativeAccountRemoval::Dsh(_))) => {
                return Err(AuthErrorCode::Unsupported);
            }
            _ => {}
        }

        let accounts = AuthAccountStore::new(self.database.clone(), private_root)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let remove = |lease: &_| accounts.remove_managed_file_account(scope, account_id, lease);
        match (scope.service, native) {
            (AuthService::ClaudeOfficial, Some(NativeAccountRemoval::Claude { home })) => self
                .with_claude_provider_unreferenced(
                    scope,
                    ClaudeBindingHost { home, private_root },
                    account_id,
                    remove,
                ),
            (AuthService::ClaudeOfficial, None) => Err(AuthErrorCode::Forbidden),
            (AuthService::CodexOauth, None) => {
                self.with_codex_provider_unreferenced(scope, private_root, account_id, remove)
            }
            (AuthService::KimiOauth, None) => {
                self.with_kimi_provider_unreferenced(scope, private_root, account_id, remove)
            }
            (AuthService::GithubCopilot | AuthService::XaiOauth, None) => {
                let lease = accounts.lease_device_removal(scope, account_id)?;
                remove(&lease)
            }
            _ => Err(AuthErrorCode::Unsupported),
        }
    }
}

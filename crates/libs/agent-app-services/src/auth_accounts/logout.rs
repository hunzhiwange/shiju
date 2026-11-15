//! Resolve scoped logout targets against the host's existing account stores.

use super::{
    AuthAccountScope, AuthAccountStore, AuthErrorCode, AuthFlows, AuthService,
    defaults::MANAGED_DEFAULT_NAMESPACE,
};
use vw_agent_apps::auth::{
    accounts::managed_default_key,
    agy_account::{AgyNativeTarget, logout_accounts},
    dsh_account::DshAccount,
    kimi_account::KimiNativeTarget,
    logout::{LogoutSelection, check_revocation_support, clear_managed_credentials},
};

/// The execution host supplies native targets resolved from trusted policy.
pub enum NativeAccountLogout<'a> {
    Agy(AgyNativeTarget<'a>),
    Dsh(DshAccount<'a>),
    Kimi { targets: &'a [KimiNativeTarget<'a>], exclusive_owner: AuthAccountScope<'a> },
}

impl AuthAccountStore {
    /// Clear local credentials while preserving account metadata and provider bindings.
    /// Invoke this blocking operation through `tokio::task::spawn_blocking` when using a native Kimi target.
    pub fn logout_accounts(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        selection: LogoutSelection<'_>,
        native: Option<NativeAccountLogout<'_>>,
    ) -> Result<(), AuthErrorCode> {
        scope.validate()?;
        let kimi = match (scope.service, native) {
            (AuthService::AgyOauth, Some(NativeAccountLogout::Agy(target))) => {
                return logout_accounts(flows, scope, selection, target);
            }
            (AuthService::DshAccount, Some(NativeAccountLogout::Dsh(target))) => {
                return target.logout(flows, scope, selection);
            }
            (
                AuthService::KimiOauth,
                Some(NativeAccountLogout::Kimi { targets, exclusive_owner }),
            ) => {
                if exclusive_owner != scope {
                    return Err(AuthErrorCode::Forbidden);
                }
                for target in targets {
                    target.check_scope(scope)?;
                }
                Some(targets)
            }
            (AuthService::AgyOauth | AuthService::DshAccount, None) => {
                return Err(AuthErrorCode::Forbidden);
            }
            (_, Some(_)) => return Err(AuthErrorCode::Unsupported),
            (_, None) => None,
        };
        match selection {
            LogoutSelection::Account(id) => {
                self.read_metadata(&scope, id)?;
                flows.cancel_for_logout(scope, Some(id))?;
                self.logout_managed_account(scope, id, kimi)
            }
            LogoutSelection::Default => {
                let id = if scope.service == AuthService::KimiOauth {
                    let targets = kimi.ok_or(AuthErrorCode::Forbidden)?;
                    let ids = self.scoped_account_ids(scope)?;
                    let mut selected = None;
                    for target in targets {
                        match target.default_managed_account(&self.credentials, scope, &ids) {
                            Ok(id) if selected.is_none() => selected = Some(id),
                            Ok(_) => return Err(AuthErrorCode::Conflict),
                            Err(AuthErrorCode::NotFound) => {}
                            Err(error) => return Err(error),
                        }
                    }
                    selected.ok_or(AuthErrorCode::NotFound)?
                } else {
                    self.managed_default_account(scope)?.ok_or(AuthErrorCode::NotFound)?
                };
                self.read_metadata(&scope, &id)?;
                flows.cancel_for_logout(scope, Some(&id))?;
                self.logout_managed_account(scope, &id, kimi)
            }
            LogoutSelection::All => {
                flows.cancel_for_logout(scope, None)?;
                for id in self.scoped_account_ids(scope)? {
                    self.logout_managed_account(scope, &id, kimi)?;
                }
                Ok(())
            }
        }
    }

    /// DSH revocation is the only remote account invalidation supported by the source.
    pub async fn revoke_account(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        native: Option<DshAccount<'_>>,
        locale: &str,
        timezone_offset_seconds: i32,
    ) -> Result<(), AuthErrorCode> {
        check_revocation_support(scope)?;
        native
            .ok_or(AuthErrorCode::Forbidden)?
            .revoke(flows, scope, account_id, locale, timezone_offset_seconds)
            .await
    }

    fn scoped_account_ids(
        &self,
        scope: AuthAccountScope<'_>,
    ) -> Result<Vec<String>, AuthErrorCode> {
        Ok(self.scoped_account_rows(scope)?.into_iter().map(|(id, _)| id).collect())
    }

    fn logout_managed_account(
        &self,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        kimi: Option<&[KimiNativeTarget<'_>]>,
    ) -> Result<(), AuthErrorCode> {
        let metadata = self.read_metadata(&scope, account_id)?;
        let kimi = kimi
            .filter(|targets| !targets.is_empty())
            .map(|targets| {
                metadata.region.map(|region| (targets, region)).ok_or(AuthErrorCode::Conflict)
            })
            .transpose()?;
        clear_managed_credentials(
            &self.credentials,
            scope,
            account_id,
            metadata.external_identity_sha256,
            kimi,
            || {
                let key = match managed_default_key(scope) {
                    Ok(key) => key,
                    Err(AuthErrorCode::Unsupported) => return Ok(()),
                    Err(error) => return Err(error),
                };
                let expected =
                    serde_json::to_string(account_id).map_err(|_| AuthErrorCode::InvalidInput)?;
                self.metadata
                    .compare_delete(scope.tenant, MANAGED_DEFAULT_NAMESPACE, &key, &expected)
                    .map_err(|_| AuthErrorCode::StorageUnavailable)?;
                Ok(())
            },
        )
    }
}

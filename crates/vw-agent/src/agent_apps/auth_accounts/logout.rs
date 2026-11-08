//! Resolve scoped logout targets against the host's existing account stores.

use super::{
    AuthAccountScope, AuthAccountStore, AuthErrorCode, AuthFlows, AuthService,
    MANAGED_ACCOUNT_NAMESPACE, defaults::MANAGED_DEFAULT_NAMESPACE,
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
    Kimi { target: KimiNativeTarget<'a>, exclusive_owner: AuthAccountScope<'a> },
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
                Some(NativeAccountLogout::Kimi { target, exclusive_owner }),
            ) => {
                if exclusive_owner != scope {
                    return Err(AuthErrorCode::Forbidden);
                }
                target.check_scope(scope)?;
                Some(target)
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
                self.logout_managed_account(scope, id, kimi.as_ref())
            }
            LogoutSelection::Default => {
                let id = if scope.service == AuthService::KimiOauth {
                    let kimi = kimi.as_ref().ok_or(AuthErrorCode::Forbidden)?;
                    kimi.default_managed_account(
                        &self.credentials,
                        scope,
                        &self.scoped_account_ids(scope)?,
                    )?
                } else {
                    self.managed_default_account(scope)?.ok_or(AuthErrorCode::NotFound)?
                };
                self.read_metadata(&scope, &id)?;
                flows.cancel_for_logout(scope, Some(&id))?;
                self.logout_managed_account(scope, &id, kimi.as_ref())
            }
            LogoutSelection::All => {
                flows.cancel_for_logout(scope, None)?;
                for id in self.scoped_account_ids(scope)? {
                    self.logout_managed_account(scope, &id, kimi.as_ref())?;
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
        let rows = self
            .metadata
            .list(scope.tenant, MANAGED_ACCOUNT_NAMESPACE)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let mut ids = Vec::new();
        for row in rows {
            let (principal, host, service, id): (String, String, AuthService, String) =
                serde_json::from_str(&row.config_key)
                    .map_err(|_| AuthErrorCode::StorageUnavailable)?;
            if principal == scope.principal && host == scope.host && service == scope.service {
                ids.push(id);
            }
        }
        Ok(ids)
    }

    fn logout_managed_account(
        &self,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        kimi: Option<&KimiNativeTarget<'_>>,
    ) -> Result<(), AuthErrorCode> {
        let metadata = self.read_metadata(&scope, account_id)?;
        let kimi = kimi
            .map(|target| {
                metadata.region.map(|region| (target, region)).ok_or(AuthErrorCode::Conflict)
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

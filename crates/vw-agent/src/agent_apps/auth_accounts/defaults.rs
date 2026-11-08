//! Persist managed defaults and delegate native selection to the execution host.

use super::{AuthAccountScope, AuthAccountStore, AuthErrorCode, AuthService};
use vw_agent_apps::auth::{
    accounts::{managed_default_key, set_managed_default},
    agy_account::{AgyNativeTarget, switch_default_account},
};

const MANAGED_DEFAULT_NAMESPACE: &str = "agent_apps.auth_defaults.v2";

/// The execution host resolves native targets from trusted host policy.
pub enum NativeAccountDefault<'a> {
    Agy(AgyNativeTarget<'a>),
    #[cfg(target_os = "macos")]
    Kimi {
        target: vw_agent_apps::auth::kimi_account::KimiNativeTarget<'a>,
        exclusive_owner: AuthAccountScope<'a>,
    },
}

impl AuthAccountStore {
    /// Set the default for an authorized scope. The native slot is the sole
    /// publication point for AGY and Kimi.
    pub async fn set_default_account(
        &self,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        native: Option<NativeAccountDefault<'_>>,
    ) -> Result<(), AuthErrorCode> {
        scope.validate()?;
        match (scope.service, native) {
            (AuthService::AgyOauth, Some(NativeAccountDefault::Agy(target))) => {
                crate::global_database::run_sync_database(|| {
                    Ok(switch_default_account(scope, account_id, target))
                })
                .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?
            }
            #[cfg(target_os = "macos")]
            (
                AuthService::KimiOauth,
                Some(NativeAccountDefault::Kimi { target, exclusive_owner }),
            ) => {
                if exclusive_owner != scope {
                    return Err(AuthErrorCode::Forbidden);
                }
                let adapter = vw_agent_apps::auth::kimi::KimiDeviceAuth::new()
                    .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
                self.use_kimi_account(&adapter, scope, account_id, target).await
            }
            #[cfg(target_os = "macos")]
            (AuthService::AgyOauth | AuthService::KimiOauth, None) => Err(AuthErrorCode::Forbidden),
            (_, Some(_)) => Err(AuthErrorCode::Unsupported),
            (_, None) => crate::global_database::run_sync_database(|| {
                Ok(self.set_managed_default_account(scope, account_id))
            })
            .map_err(|_| AuthErrorCode::StorageUnavailable)?,
        }
    }

    pub(super) fn set_managed_default_account(
        &self,
        scope: AuthAccountScope<'_>,
        account_id: &str,
    ) -> Result<(), AuthErrorCode> {
        set_managed_default(&self.credentials, scope, account_id, |key| {
            self.read_metadata(&scope, account_id)?;
            let payload =
                serde_json::to_string(account_id).map_err(|_| AuthErrorCode::StorageUnavailable)?;
            self.metadata
                .upsert(scope.tenant, MANAGED_DEFAULT_NAMESPACE, key, &payload)
                .map_err(|_| AuthErrorCode::StorageUnavailable)
        })
    }

    /// Read the explicit managed default in this scope through the storage owner.
    /// Run this blocking read outside the async runtime.
    pub fn managed_default_account(
        &self,
        scope: AuthAccountScope<'_>,
    ) -> Result<Option<String>, AuthErrorCode> {
        let key = managed_default_key(scope)?;
        self.metadata
            .get(scope.tenant, MANAGED_DEFAULT_NAMESPACE, &key)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
            .map(|payload| {
                let id: String = serde_json::from_str(&payload)
                    .map_err(|_| AuthErrorCode::StorageUnavailable)?;
                vw_agent_apps::providers::validate_identity(&id)
                    .map_err(|_| AuthErrorCode::StorageUnavailable)?;
                Ok(id)
            })
            .transpose()
    }
}

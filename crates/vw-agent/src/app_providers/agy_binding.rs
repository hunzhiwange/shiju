//! Official AGY card references. The native slot remains owned by the Keychain adapter.
use super::*;
use crate::agent_apps::auth_accounts::AuthAccountScope;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vw_agent_apps::auth::agy_account::{AgyNativeTarget, with_provider_account};
use vw_api_types::{
    agent_apps_auth::{AuthErrorCode, AuthService},
    app_providers::agy::AgyProfile,
};

const FIELD: &str = "managedAgyLogin";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgyBinding {
    host: String,
    principal: String,
    account_id: String,
}

impl AppProviderStore {
    /// The dispatcher supplies an independently authorized owner for the physical AGY slot.
    pub fn bind_agy_account(
        &self,
        scope: AuthAccountScope<'_>,
        native: AgyNativeTarget<'_>,
        private_root: &Path,
        provider_id: &str,
        expected_account: Option<&str>,
        account_id: Option<&str>,
    ) -> Result<(), AuthErrorCode> {
        check_scope(scope)?;
        if native.exclusive_owner != Some(scope) {
            return Err(AuthErrorCode::Forbidden);
        }
        let _catalog = vw_agent_apps::providers::agy::catalog_lease(
            &vw_agent_apps::providers::agy::Scope {
                home: native.home,
                private_root,
                host: scope.host,
                tenant: scope.tenant,
                provider: provider_id,
            },
            true,
        )
        .map_err(|_| AuthErrorCode::Conflict)?;
        let key = Self::key(AppId::Agy, provider_id).map_err(|_| AuthErrorCode::InvalidInput)?;
        let raw = self
            .settings
            .get(scope.tenant, NAMESPACE, &key)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
            .ok_or(AuthErrorCode::NotFound)?;
        let mut row: Value =
            serde_json::from_str(&raw).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        official(&row)?;
        let current = account(&row, scope)?;
        if current.as_deref() == account_id {
            return Ok(());
        }
        if current.as_deref() != expected_account {
            return Err(AuthErrorCode::Conflict);
        }
        with_provider_account(scope, account_id, native, false, |_| {
            if let Some(id) = account_id {
                row[FIELD] = serde_json::to_value(AgyBinding {
                    host: scope.host.into(),
                    principal: scope.principal.into(),
                    account_id: id.into(),
                })
                .map_err(|_| AuthErrorCode::StorageUnavailable)?;
            } else {
                row.as_object_mut().ok_or(AuthErrorCode::StorageUnavailable)?.remove(FIELD);
            }
            self.settings
                .compare_exchange(scope.tenant, NAMESPACE, &key, &raw, &row.to_string())
                .map_err(|_| AuthErrorCode::StorageUnavailable)?
                .then_some(())
                .ok_or(AuthErrorCode::Conflict)
        })
    }

    pub fn agy_account_binding(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
    ) -> Result<Option<String>, AuthErrorCode> {
        check_scope(scope)?;
        let row = self
            .agy_record(scope.tenant, id)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
            .ok_or(AuthErrorCode::NotFound)?;
        account(&row, scope)
    }

    /// The callback runs under the same physical lock as binding, checkout, and native writes.
    pub fn with_agy_provider_unreferenced<T>(
        &self,
        scope: AuthAccountScope<'_>,
        native: AgyNativeTarget<'_>,
        account_id: &str,
        mutate: impl FnOnce() -> Result<T, AuthErrorCode>,
    ) -> Result<T, AuthErrorCode> {
        check_scope(scope)?;
        with_provider_account(scope, Some(account_id), native, true, |active| {
            if active {
                return Err(AuthErrorCode::AccountInUse);
            }
            self.check_agy_references(scope, account_id)?;
            mutate()
        })
    }

    pub(super) fn check_agy_references(
        &self,
        scope: AuthAccountScope<'_>,
        account_id: &str,
    ) -> Result<(), AuthErrorCode> {
        for row in self
            .settings
            .list(scope.tenant, NAMESPACE)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
        {
            if !row.config_key.starts_with("\"agy\":") {
                continue;
            }
            let row: Value = serde_json::from_str(&row.payload_json)
                .map_err(|_| AuthErrorCode::StorageUnavailable)?;
            if binding(&row)?.is_some_and(|b| {
                b.host == scope.host && b.principal == scope.principal && b.account_id == account_id
            }) {
                return Err(AuthErrorCode::AccountInUse);
            }
        }
        Ok(())
    }
}

pub(super) fn account(
    row: &Value,
    scope: AuthAccountScope<'_>,
) -> Result<Option<String>, AuthErrorCode> {
    let Some(binding) = binding(row)? else {
        return Ok(None);
    };
    if binding.host != scope.host || binding.principal != scope.principal {
        return Err(AuthErrorCode::Forbidden);
    }
    Ok(Some(binding.account_id))
}

pub(crate) fn has_agy_binding(row: &Value) -> bool {
    row.get(FIELD).is_some()
}

pub(super) fn check_profile_edit(row: &Value, profile: &AgyProfile) -> Result<(), String> {
    if has_agy_binding(row) && (profile.api_key.is_some() || profile.base_url.is_some()) {
        return Err(
            "Agy managed binding must be removed before replacing the official profile".into()
        );
    }
    Ok(())
}

fn binding(row: &Value) -> Result<Option<AgyBinding>, AuthErrorCode> {
    row.get(FIELD)
        .map(|raw| {
            official(row)?;
            serde_json::from_value(raw.clone()).map_err(|_| AuthErrorCode::StorageUnavailable)
        })
        .transpose()
}

fn official(row: &Value) -> Result<(), AuthErrorCode> {
    let profile = super::agy::profile_from_record(row).map_err(|_| AuthErrorCode::Unsupported)?;
    if profile.api_key.is_some()
        || profile.base_url.is_some()
        || row.get("requestOptions").is_some_and(|value| !value.is_null())
    {
        return Err(AuthErrorCode::Unsupported);
    }
    Ok(())
}

fn check_scope(scope: AuthAccountScope<'_>) -> Result<(), AuthErrorCode> {
    scope.validate()?;
    if scope.service != AuthService::AgyOauth {
        return Err(AuthErrorCode::Unsupported);
    }
    Ok(())
}

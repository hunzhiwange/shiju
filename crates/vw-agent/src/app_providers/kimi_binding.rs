//! Internal entry points for the authorized host dispatcher.
use super::{AppProviderStore, NAMESPACE, kimi::kimi_key};
use crate::agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore};
use crate::global_database::run_sync_database;
use serde_json::Value;
use std::path::Path;
use vw_agent_apps::auth::kimi_account::KimiNativeTarget;
use vw_api_types::{
    agent_apps_auth::{AuthErrorCode, AuthService},
    id::TenantScope,
};

impl AppProviderStore {
    /// Commit an official Kimi provider binding and its native model selection.
    /// Scope, native target and credential root must come from the same authorized host.
    pub async fn bind_kimi_account(
        &self,
        scope: AuthAccountScope<'_>,
        credential_root: &Path,
        native: &KimiNativeTarget<'_>,
        expected_account: Option<&str>,
        account_id: &str,
    ) -> Result<(), AuthErrorCode> {
        native.check_scope(scope)?;
        let row = binding_row(self, scope, native.provider_id)?;
        let connection = connection(&row)?;
        let accounts = self.binding_accounts(credential_root)?;
        let catalog_unchanged =
            self.binding_catalog_unchanged(scope.tenant, native.provider_id, &row)?;
        accounts
            .bind_kimi_provider(
                scope,
                account_id,
                native,
                expected_account,
                connection,
                catalog_unchanged,
            )
            .await
    }

    pub(crate) fn unbind_kimi_account(
        &self,
        scope: AuthAccountScope<'_>,
        native: &KimiNativeTarget<'_>,
    ) -> Result<(), AuthErrorCode> {
        native.check_scope(scope)?;
        let row = binding_row(self, scope, native.provider_id)?;
        connection(&row)?;
        let catalog_unchanged =
            self.binding_catalog_unchanged(scope.tenant, native.provider_id, &row)?;
        native.unbind_account(scope, catalog_unchanged)
    }

    /// Resolve only a committed binding for this authorized account owner.
    pub fn kimi_account_binding(
        &self,
        scope: AuthAccountScope<'_>,
        credential_root: &Path,
        native: &KimiNativeTarget<'_>,
    ) -> Result<Option<String>, AuthErrorCode> {
        native.check_scope(scope)?;
        let row = binding_row(self, scope, native.provider_id)?;
        let Some(id) = native.bound_account(scope, connection(&row)?)? else {
            return Ok(None);
        };
        let accounts = self.binding_accounts(credential_root)?;
        run_sync_database(|| Ok(accounts.read_metadata(&scope, &id)))
            .map_err(|_| AuthErrorCode::StorageUnavailable)??;
        Ok(Some(id))
    }

    /// Use the committed binding through Kimi's native grant verification and selection.
    pub async fn use_kimi_account_binding(
        &self,
        scope: AuthAccountScope<'_>,
        credential_root: &Path,
        native: KimiNativeTarget<'_>,
    ) -> Result<(), AuthErrorCode> {
        native.check_scope(scope)?;
        let row = binding_row(self, scope, native.provider_id)?;
        let connection = connection(&row)?;
        let id = native.bound_account(scope, connection)?.ok_or(AuthErrorCode::NotFound)?;
        let catalog_unchanged =
            self.binding_catalog_unchanged(scope.tenant, native.provider_id, &row)?;
        self.binding_accounts(credential_root)?
            .use_bound_kimi_account(scope, &id, native, connection, catalog_unchanged)
            .await
    }

    /// Share the account lease with binding and use while checking every host-local reference.
    pub fn with_kimi_provider_unreferenced<T>(
        &self,
        scope: AuthAccountScope<'_>,
        root: &Path,
        account: &str,
        mutate: impl FnOnce(
            &vw_agent_apps::auth::credential_lease::CredentialLease,
        ) -> Result<T, AuthErrorCode>,
    ) -> Result<T, AuthErrorCode> {
        use vw_agent_apps::{
            auth::credential_file::CredentialFileStore,
            providers::live::commit::{ExecutionHostId, ProviderTransactions},
        };
        use vw_api_types::agent_apps::AppId;
        scope.validate()?;
        if scope.service != AuthService::KimiOauth {
            return Err(AuthErrorCode::Unsupported);
        }
        let accounts = self.binding_accounts(root)?;
        run_sync_database(|| Ok(accounts.read_metadata(&scope, account)))
            .map_err(|_| AuthErrorCode::StorageUnavailable)??;
        let credentials =
            CredentialFileStore::new(root).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let lease = credentials.lease_kimi(scope, account, false).map_err(|error| {
            if error == "official account is in use" {
                AuthErrorCode::AccountInUse
            } else {
                AuthErrorCode::StorageUnavailable
            }
        })?;
        run_sync_database(|| Ok(accounts.read_metadata(&scope, account)))
            .map_err(|_| AuthErrorCode::StorageUnavailable)??;
        let host = ExecutionHostId::new(scope.host).map_err(|_| AuthErrorCode::InvalidInput)?;
        if let Some(tx) = ProviderTransactions::open_existing(root, host)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
        {
            if tx.pending(AppId::Kimi).map_err(|_| AuthErrorCode::StorageUnavailable)?.is_some() {
                return Err(AuthErrorCode::Conflict);
            }
            if tx
                .state(AppId::Kimi)
                .map_err(|_| AuthErrorCode::StorageUnavailable)?
                .uses_kimi_account(scope.tenant, scope.principal, account)
            {
                return Err(AuthErrorCode::AccountInUse);
            }
        }
        mutate(&lease)
    }

    fn binding_catalog_unchanged(
        &self,
        tenant: &TenantScope,
        provider_id: &str,
        row: &Value,
    ) -> Result<impl Fn() -> bool + Send + use<>, AuthErrorCode> {
        let tenant = tenant.clone();
        let key = kimi_key(provider_id).map_err(|_| AuthErrorCode::InvalidInput)?;
        let settings = self.settings.clone();
        let row = row.clone();
        Ok(move || {
            run_sync_database(|| {
                let current = settings.get(&tenant, NAMESPACE, &key)?;
                Ok(current.and_then(|raw| serde_json::from_str::<Value>(&raw).ok()).as_ref()
                    == Some(&row))
            })
            .unwrap_or(false)
        })
    }

    fn binding_accounts(&self, root: &Path) -> Result<AuthAccountStore, AuthErrorCode> {
        run_sync_database(|| {
            AuthAccountStore::new(self.database.clone(), root).map_err(anyhow::Error::msg)
        })
        .map_err(|_| AuthErrorCode::StorageUnavailable)
    }
}

fn binding_row(
    store: &AppProviderStore,
    scope: AuthAccountScope<'_>,
    id: &str,
) -> Result<Value, AuthErrorCode> {
    scope.validate()?;
    if scope.service != AuthService::KimiOauth {
        return Err(AuthErrorCode::Unsupported);
    }
    vw_agent_apps::providers::kimi::validate_identity(id)
        .map_err(|_| AuthErrorCode::InvalidInput)?;
    let tenant: &TenantScope = scope.tenant;
    run_sync_database(|| store.kimi_record(tenant, id).map_err(anyhow::Error::msg))
        .map_err(|_| AuthErrorCode::StorageUnavailable)?
        .ok_or(AuthErrorCode::NotFound)
}

fn connection(row: &Value) -> Result<&Value, AuthErrorCode> {
    use vw_agent_apps::providers::kimi::{form::managed, official};
    let connection = row
        .pointer("/kimi/declaration/provider")
        .filter(|node| official(node))
        .ok_or(AuthErrorCode::Unsupported)?;
    if managed(row) || row.pointer("/settingsConfig/provider") != Some(connection) {
        return Err(AuthErrorCode::Unsupported);
    }
    Ok(connection)
}

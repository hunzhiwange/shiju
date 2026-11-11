//! Routing-only Gemini grants. Caller verifies membership and host/account permission.
use super::{AuthAccountMetadata, AuthAccountStore, MANAGED_ACCOUNT_NAMESPACE};
use vw_agent_apps::auth::{
    credential_file::AccountCredential,
    gemini_refresh::{GeminiRefresh, GeminiScope},
};
use vw_api_types::{agent_apps::AppId, agent_apps_auth::AuthErrorCode};

fn key(scope: GeminiScope<'_>, account: &str) -> Result<String, AuthErrorCode> {
    vw_agent_apps::providers::validate_identity(scope.host)
        .map_err(|_| AuthErrorCode::InvalidInput)?;
    vw_agent_apps::providers::validate_identity(account)
        .map_err(|_| AuthErrorCode::InvalidInput)?;
    if scope.principal.trim().is_empty() || scope.principal.chars().any(char::is_control) {
        return Err(AuthErrorCode::InvalidInput);
    }
    serde_json::to_string(&(scope.principal, scope.host, "gemini_oauth", account))
        .map_err(|_| AuthErrorCode::InvalidInput)
}
impl AuthAccountStore {
    /// Import only an explicitly verified grant, never ambient CLI login files.
    pub fn import_gemini_grant(
        &self,
        scope: GeminiScope<'_>,
        account: &str,
        metadata: &AuthAccountMetadata,
        credential: &AccountCredential,
    ) -> Result<(), AuthErrorCode> {
        let key = key(scope, account)?;
        credential.validate(AppId::Gemini).map_err(|_| AuthErrorCode::InvalidInput)?;
        let lease = self
            .credentials
            .for_owner(scope.principal)
            .and_then(|owner| owner.lease(scope.tenant, scope.host, AppId::Gemini, account, true))
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        if self
            .metadata
            .get(scope.tenant, MANAGED_ACCOUNT_NAMESPACE, &key)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
            .is_some()
        {
            return Err(AuthErrorCode::Conflict);
        }
        lease.save(credential).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let payload =
            serde_json::to_string(metadata).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        if !self
            .metadata
            .insert_if_absent(scope.tenant, MANAGED_ACCOUNT_NAMESPACE, &key, &payload)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
        {
            return Err(AuthErrorCode::Conflict);
        }
        Ok(())
    }
    pub async fn gemini_access_token(
        &self,
        refresh: &GeminiRefresh,
        scope: GeminiScope<'_>,
        account: &str,
    ) -> Result<String, AuthErrorCode> {
        let key = key(scope, account)?;
        crate::global_database::run_sync_database(|| {
            self.metadata.get(scope.tenant, MANAGED_ACCOUNT_NAMESPACE, &key)
        })
        .map_err(|_| AuthErrorCode::StorageUnavailable)?
        .ok_or(AuthErrorCode::NotFound)?;
        refresh.access_token(&self.credentials, scope, account).await
    }
}

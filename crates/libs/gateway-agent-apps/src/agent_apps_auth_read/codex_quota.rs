//! Official quota for a managed Codex account, independent of provider bindings.

use std::sync::{Arc, OnceLock};

use axum::{
    Extension, Json,
    extract::{Query, rejection::QueryRejection},
    http::HeaderMap,
};
use sha2::{Digest, Sha256};
use vw_agent_apps::{
    auth::{
        codex_device::managed_identity, codex_refresh::CodexRefresh,
        credential_file::CredentialFileStore,
    },
    usage::subscriptions::codex::ManagedCodexUsage,
};
use vw_api_apps_usage::apps_usage::UsageQueryResult;
use vw_api_transport::TimestampMs;

use super::{
    AuthAccountScope, AuthAccountStore, AuthErrorCode, AuthService, GlobalDatabase, PolicyFile,
    ReadError, ScopeQuery, TenantScope, authorize_auth_account_read,
};

#[derive(Default)]
pub(super) struct CodexQuota {
    refresh: OnceLock<Result<CodexRefresh, AuthErrorCode>>,
}

pub(super) async fn query_codex_quota(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    Extension(quota): Extension<Arc<CodexQuota>>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<ScopeQuery>, QueryRejection>,
) -> Result<Json<UsageQueryResult>, ReadError> {
    let Query(query) = query.map_err(|_| AuthErrorCode::InvalidInput)?;
    if query.service != AuthService::CodexOauth {
        return Err(AuthErrorCode::Unsupported.into());
    }
    let account = query.account_id.ok_or(AuthErrorCode::InvalidInput)?;
    for id in [&query.host_id, &account] {
        vw_agent_apps::providers::validate_identity(id).map_err(|_| AuthErrorCode::InvalidInput)?;
    }
    let tenant_for_host = tenant.clone();
    let (host, accounts, credentials) = tokio::task::spawn_blocking(move || {
        let host = authorize_auth_account_read(
            policy.0.as_deref(),
            &headers,
            &tenant_for_host,
            &query.host_id,
            query.service,
            false,
        )
        .map_err(ReadError::access)?;
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&agent_paths::paths().home)
                .map_err(|_| AuthErrorCode::StorageUnavailable)?,
        };
        let accounts = AuthAccountStore::new(database, &host.credential_root)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let credentials = CredentialFileStore::new(&host.credential_root)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        Ok::<_, ReadError>((host, accounts, credentials))
    })
    .await
    .map_err(|_| AuthErrorCode::StorageUnavailable)??;

    let scope = AuthAccountScope {
        tenant: &tenant,
        host: &host.id,
        principal: &host.principal,
        service: AuthService::CodexOauth,
    };
    let read_metadata = || {
        agent_database::run_sync_database(|| Ok(accounts.read_metadata(&scope, &account)))
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
    };
    let metadata = read_metadata()?;
    let refresh = quota.refresh.get_or_init(CodexRefresh::new).as_ref().map_err(|error| *error)?;
    let access_token = accounts.codex_access_token(refresh, scope, &account).await?;
    let (auth, version) =
        credentials.codex_usage_snapshot(scope, &account)?.ok_or(AuthErrorCode::ReauthRequired)?;
    if auth["tokens"]["access_token"].as_str() != Some(access_token.as_str()) {
        return Err(AuthErrorCode::Conflict.into());
    }
    let identity: [u8; 32] = Sha256::digest(managed_identity(&auth)?.as_bytes()).into();
    if identity != metadata.external_identity_sha256 {
        return Err(AuthErrorCode::IdentityMismatch.into());
    }

    let result = ManagedCodexUsage::new(reqwest::Client::builder())?
        .query(
            &auth,
            TimestampMs(chrono::Utc::now().timestamp_millis()),
            &uuid::Uuid::new_v4().to_string(),
        )
        .await;
    let current = read_metadata()?;
    if current.external_identity_sha256 != metadata.external_identity_sha256
        || current.authenticated_at != metadata.authenticated_at
        || credentials.codex_usage_version(scope, &account)? != Some(version)
    {
        return Err(AuthErrorCode::Conflict.into());
    }
    Ok(Json(result))
}

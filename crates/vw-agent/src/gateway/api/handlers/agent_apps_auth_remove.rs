//! Scoped deletion on the management chain.

use super::{
    agent_apps::authorize_auth_account_delete,
    agent_apps_auth_read::PolicyFile,
    agent_apps_auth_start::{FlowError, ScopeQuery},
};
use crate::global_database::GlobalDatabase;
#[cfg(target_os = "macos")]
use crate::{
    agent_apps::auth_accounts::AuthAccountScope,
    app_providers::{AppProviderStore, NativeAccountRemoval},
};
use axum::{
    Extension,
    extract::{
        Path, Query,
        rejection::{PathRejection, QueryRejection},
    },
    http::{HeaderMap, StatusCode},
};
use vw_agent_apps::providers::validate_identity;
#[cfg(target_os = "macos")]
use vw_agent_apps::{
    auth::{agy_account::AgyNativeTarget, dsh_account::DshAccount},
    providers::live::commit::{ExecutionHostId, ProviderTransactions},
};
#[cfg(target_os = "macos")]
use vw_api_types::agent_apps_auth::AuthService;
use vw_api_types::{agent_apps_auth::AuthErrorCode, id::TenantScope};

pub(super) async fn delete(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    account: Result<Path<String>, PathRejection>,
    query: Result<Query<ScopeQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<StatusCode, FlowError> {
    let Query(ScopeQuery { host_id, service }) = query.map_err(|_| AuthErrorCode::InvalidInput)?;
    validate_identity(&host_id).map_err(|_| AuthErrorCode::InvalidInput)?;

    // A started blocking transaction runs to completion even if the caller disconnects.
    tokio::task::spawn_blocking(move || {
        let host = authorize_auth_account_delete(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &host_id,
            service,
        )
        .map_err(FlowError::access)?;
        let Path(account) = account.map_err(|_| AuthErrorCode::InvalidInput)?;
        validate_identity(&account).map_err(|_| AuthErrorCode::InvalidInput)?;

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (host, database);
            Err(AuthErrorCode::Unsupported.into())
        }
        #[cfg(target_os = "macos")]
        {
            let database = match database {
                Some(Extension(database)) => database,
                None => GlobalDatabase::for_home(&crate::global::paths().home)
                    .map_err(|_| AuthErrorCode::StorageUnavailable)?,
            };
            let providers =
                AppProviderStore::new(database).map_err(|_| AuthErrorCode::StorageUnavailable)?;
            let scope = AuthAccountScope {
                tenant: &tenant,
                host: &host.id,
                principal: &host.principal,
                service,
            };
            let transactions;
            let native = match service {
                AuthService::ClaudeOfficial => {
                    Some(NativeAccountRemoval::Claude { home: &host.home })
                }
                AuthService::AgyOauth => Some(NativeAccountRemoval::Agy(AgyNativeTarget {
                    home: &host.home,
                    exclusive_owner: Some(scope),
                })),
                AuthService::DshAccount => {
                    let id = ExecutionHostId::new(scope.host)
                        .map_err(|_| AuthErrorCode::InvalidInput)?;
                    transactions = ProviderTransactions::new(&host.credential_root, id)
                        .map_err(|_| AuthErrorCode::StorageUnavailable)?;
                    Some(NativeAccountRemoval::Dsh(DshAccount::new(
                        &host.home,
                        &transactions,
                        scope,
                    )?))
                }
                _ => None,
            };
            providers.remove_auth_account(scope, &host.credential_root, &account, native)?;
            Ok(StatusCode::NO_CONTENT)
        }
    })
    .await
    .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?
}

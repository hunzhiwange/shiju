//! Scoped default selection on the management chain.

use super::{
    agent_apps::authorize_auth_default,
    agent_apps_auth_read::PolicyFile,
    agent_apps_auth_start::{FlowError, ScopeQuery},
};
use crate::{
    agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore, NativeAccountDefault},
    global_database::GlobalDatabase,
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
    auth::agy_account::AgyNativeTarget,
    providers::live::commit::{ExecutionHostId, ProviderTransactions},
};
use vw_api_types::{
    agent_apps_auth::{AuthErrorCode, AuthService},
    id::TenantScope,
};

pub(super) fn supports_default(service: AuthService, owns_native_slot: bool) -> bool {
    match service {
        AuthService::GithubCopilot | AuthService::CodexOauth | AuthService::XaiOauth => true,
        AuthService::AgyOauth | AuthService::KimiOauth => {
            cfg!(target_os = "macos") && owns_native_slot
        }
        AuthService::ClaudeOfficial | AuthService::DshAccount => false,
    }
}

pub(super) async fn put(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    account: Result<Path<String>, PathRejection>,
    query: Result<Query<ScopeQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<StatusCode, FlowError> {
    let Query(ScopeQuery { host_id, service }) = query.map_err(|_| AuthErrorCode::InvalidInput)?;
    validate_identity(&host_id).map_err(|_| AuthErrorCode::InvalidInput)?;
    let (tenant, host) = tokio::task::spawn_blocking(move || {
        let host =
            authorize_auth_default(policy.0.as_deref(), &headers, &tenant, &host_id, service)
                .map_err(FlowError::access)?;
        Ok::<_, FlowError>((tenant, host))
    })
    .await
    .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)??;

    let Path(account) = account.map_err(|_| AuthErrorCode::InvalidInput)?;
    validate_identity(&account).map_err(|_| AuthErrorCode::InvalidInput)?;
    if !supports_default(service, true) {
        return Err(AuthErrorCode::Unsupported.into());
    }
    if !supports_default(service, host.owns_native_slot) {
        return Err(AuthErrorCode::Forbidden.into());
    }

    // Once started, a native transaction must finish even if the caller disconnects.
    tokio::spawn(async move {
        let scope = AuthAccountScope {
            tenant: &tenant,
            host: &host.id,
            principal: &host.principal,
            service,
        };
        let store = crate::global_database::run_sync_database(|| {
            let database = match database {
                Some(Extension(database)) => database,
                None => GlobalDatabase::for_home(&crate::global::paths().home)?,
            };
            AuthAccountStore::new(database, &host.credential_root).map_err(anyhow::Error::msg)
        })
        .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        #[cfg(target_os = "macos")]
        let transactions;
        let native: Option<NativeAccountDefault<'_>> = match service {
            #[cfg(target_os = "macos")]
            AuthService::AgyOauth => Some(NativeAccountDefault::Agy(AgyNativeTarget {
                home: &host.home,
                exclusive_owner: Some(scope),
            })),
            #[cfg(target_os = "macos")]
            AuthService::KimiOauth => {
                let id =
                    ExecutionHostId::new(scope.host).map_err(|_| AuthErrorCode::InvalidInput)?;
                transactions = std::sync::Arc::new(
                    ProviderTransactions::new(&host.credential_root, id)
                        .map_err(|_| AuthErrorCode::StorageUnavailable)?,
                );
                Some(NativeAccountDefault::Kimi {
                    home: &host.home,
                    transactions: &transactions,
                    exclusive_owner: scope,
                })
            }
            _ => None,
        };
        store.set_default_account(scope, &account, native).await
    })
    .await
    .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)??;
    Ok(StatusCode::NO_CONTENT)
}

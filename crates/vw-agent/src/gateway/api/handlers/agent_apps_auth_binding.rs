//! Scoped supplier binding mutations on the management chain.

#[cfg(target_os = "macos")]
use super::agent_apps::AuthAccountHost;
use super::{
    agent_apps::authorize_auth_binding,
    agent_apps_auth_read::PolicyFile,
    agent_apps_auth_start::{FlowError, ScopeQuery},
};
use crate::global_database::GlobalDatabase;
#[cfg(target_os = "macos")]
use crate::{
    agent_apps::auth_accounts::AuthAccountScope,
    app_providers::{AppProviderStore, ClaudeBindingHost},
};
use axum::{
    Extension, Json,
    extract::{
        FromRequest, Path, Query, Request,
        rejection::{PathRejection, QueryRejection},
    },
    http::StatusCode,
};
use serde::Deserialize;
#[cfg(target_os = "macos")]
use std::sync::Arc;
#[cfg(target_os = "macos")]
use tokio::task::JoinHandle;
use vw_agent_apps::providers::validate_identity;
#[cfg(target_os = "macos")]
use vw_agent_apps::{
    auth::{agy_account::AgyNativeTarget, kimi_account::KimiNativeTarget},
    providers::live::commit::{ExecutionHostId, ProviderTransactions},
};
#[cfg(target_os = "macos")]
use vw_api_types::agent_apps_auth::AuthService;
use vw_api_types::{agent_apps_auth::AuthErrorCode, id::TenantScope};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingRequest {
    // An omitted field is invalid. Only explicit null removes a binding.
    #[serde(deserialize_with = "Option::<String>::deserialize")]
    account_id: Option<String>,
}

pub(super) async fn put(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    provider: Result<Path<String>, PathRejection>,
    query: Result<Query<ScopeQuery>, QueryRejection>,
    request: Request,
) -> Result<StatusCode, FlowError> {
    let Query(ScopeQuery { host_id, service }) = query.map_err(|_| AuthErrorCode::InvalidInput)?;
    validate_identity(&host_id).map_err(|_| AuthErrorCode::InvalidInput)?;
    let headers = request.headers().clone();
    let (tenant, host) = tokio::task::spawn_blocking(move || {
        let host =
            authorize_auth_binding(policy.0.as_deref(), &headers, &tenant, &host_id, service)
                .map_err(FlowError::access)?;
        Ok::<_, FlowError>((tenant, host))
    })
    .await
    .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)??;

    let Path(provider) = provider.map_err(|_| AuthErrorCode::InvalidInput)?;
    #[cfg(target_os = "macos")]
    if service == AuthService::KimiOauth {
        vw_agent_apps::providers::kimi::validate_identity(&provider)
            .map_err(|_| AuthErrorCode::InvalidInput)?;
    } else {
        validate_identity(&provider).map_err(|_| AuthErrorCode::InvalidInput)?;
    }
    let Json(input) = Json::<BindingRequest>::from_request(request, &())
        .await
        .map_err(|_| AuthErrorCode::InvalidInput)?;
    if let Some(account) = &input.account_id {
        validate_identity(account).map_err(|_| AuthErrorCode::InvalidInput)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (tenant, host, database, provider);
        Err(AuthErrorCode::Unsupported.into())
    }
    #[cfg(target_os = "macos")]
    {
        if matches!(
            service,
            AuthService::GithubCopilot | AuthService::XaiOauth | AuthService::DshAccount
        ) {
            return Err(AuthErrorCode::Unsupported.into());
        }
        // Both the blocking transactions and the async Kimi commit can finish after disconnect.
        let result = async {
            let pending = tokio::task::spawn_blocking(move || {
                let database = match database {
                    Some(Extension(database)) => database,
                    None => GlobalDatabase::for_home(&crate::global::paths().home)
                        .map_err(|_| AuthErrorCode::StorageUnavailable)?,
                };
                let store = AppProviderStore::new(database)
                    .map_err(|_| AuthErrorCode::StorageUnavailable)?;
                update(store, tenant, host, service, provider, input.account_id)
            })
            .await
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)??;
            if let Some(task) = pending {
                task.await.map_err(|_| AuthErrorCode::ExecutionHostUnavailable)??;
            }
            Ok(())
        };
        result.await.map_err(|error| {
            if error == AuthErrorCode::Forbidden { AuthErrorCode::NotFound } else { error }
        })?;
        Ok(StatusCode::NO_CONTENT)
    }
}

#[cfg(target_os = "macos")]
type PendingBinding = JoinHandle<Result<(), AuthErrorCode>>;

#[cfg(target_os = "macos")]
fn update(
    store: AppProviderStore,
    tenant: TenantScope,
    host: AuthAccountHost,
    service: AuthService,
    provider: String,
    account: Option<String>,
) -> Result<Option<PendingBinding>, AuthErrorCode> {
    let scope =
        AuthAccountScope { tenant: &tenant, host: &host.id, principal: &host.principal, service };
    let root = &host.credential_root;
    match scope.service {
        AuthService::CodexOauth => {
            let previous = store.codex_account_binding(scope, &provider)?;
            store.bind_codex_account(
                scope,
                root,
                &provider,
                previous.as_deref(),
                account.as_deref(),
            )?;
        }
        AuthService::ClaudeOfficial => {
            let previous = store.claude_account_binding(scope, &provider)?;
            let host = ClaudeBindingHost { home: &host.home, private_root: root };
            store.bind_claude_account(
                scope,
                host,
                &provider,
                previous.as_deref(),
                account.as_deref(),
            )?;
        }
        AuthService::AgyOauth => {
            let previous = store.agy_account_binding(scope, &provider)?;
            let native = AgyNativeTarget {
                home: &host.home,
                exclusive_owner: host.owns_native_slot.then_some(scope),
            };
            store.bind_agy_account(
                scope,
                native,
                root,
                &provider,
                previous.as_deref(),
                account.as_deref(),
            )?;
        }
        AuthService::KimiOauth => {
            let id = ExecutionHostId::new(scope.host).map_err(|_| AuthErrorCode::InvalidInput)?;
            let tx = Arc::new(
                ProviderTransactions::new(root, id)
                    .map_err(|_| AuthErrorCode::StorageUnavailable)?,
            );
            let native =
                KimiNativeTarget { home: &host.home, transactions: &tx, provider_id: &provider };
            if let Some(account) = account {
                let previous = store.kimi_account_binding(scope, root, &native)?;
                return Ok(Some(tokio::spawn(async move {
                    let scope = AuthAccountScope {
                        tenant: &tenant,
                        host: &host.id,
                        principal: &host.principal,
                        service,
                    };
                    let native = KimiNativeTarget {
                        home: &host.home,
                        transactions: &tx,
                        provider_id: &provider,
                    };
                    store
                        .bind_kimi_account(
                            scope,
                            &host.credential_root,
                            &native,
                            previous.as_deref(),
                            &account,
                        )
                        .await
                })));
            }
            store.unbind_kimi_account(scope, &native)?;
        }
        AuthService::GithubCopilot | AuthService::XaiOauth | AuthService::DshAccount => {
            return Err(AuthErrorCode::Unsupported);
        }
    }
    Ok(None)
}

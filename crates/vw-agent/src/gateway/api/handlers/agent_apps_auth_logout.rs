//! Local logout and confirmed remote revocation on the management chain.

use super::{
    agent_apps::{AccountTermination, authorize_auth_account_termination},
    agent_apps_auth_read::PolicyFile,
    agent_apps_auth_start::{FlowError, Runtime, ScopeQuery},
};
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Json, Router,
    extract::{
        DefaultBodyLimit, Path, Query,
        rejection::{JsonRejection, PathRejection, QueryRejection},
    },
    http::{HeaderMap, StatusCode},
    routing::post,
};
use serde::Deserialize;
use std::sync::Arc;
use vw_agent_apps::providers::validate_identity;
#[cfg(target_os = "macos")]
use vw_agent_apps::{
    auth::{
        agy_account::AgyNativeTarget,
        dsh_account::DshAccount,
        kimi_account::KimiNativeTarget,
        logout::{LogoutSelection, check_revocation_support},
    },
    providers::{
        kimi,
        live::commit::{ExecutionHostId, ProviderTransactions},
    },
};
#[cfg(target_os = "macos")]
use vw_api_types::agent_apps_auth::AuthService;
use vw_api_types::{agent_apps_auth::AuthErrorCode, id::TenantScope};
#[cfg(target_os = "macos")]
use {
    super::agent_apps::AuthAccountHost,
    crate::agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore, NativeAccountLogout},
};

#[derive(Deserialize)]
#[serde(tag = "selection", rename_all = "snake_case", deny_unknown_fields)]
enum LogoutRequest {
    Account { account_id: String },
    Default {},
    All {},
}

enum Operation {
    Logout(LogoutRequest),
    Revoke(String),
}

pub(super) fn routes(runtime: Arc<Runtime>) -> Router {
    Router::new()
        .route("/v1/agent-apps/auth/logout", post(logout).layer(DefaultBodyLimit::max(4 * 1024)))
        .route("/v1/agent-apps/auth/accounts/{account_id}/revoke", post(revoke))
        .layer(Extension(runtime))
}

async fn logout(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    Extension(runtime): Extension<Arc<Runtime>>,
    database: Option<Extension<GlobalDatabase>>,
    query: Result<Query<ScopeQuery>, QueryRejection>,
    headers: HeaderMap,
    body: Result<Json<LogoutRequest>, JsonRejection>,
) -> Result<StatusCode, FlowError> {
    let operation =
        body.map(|Json(body)| Operation::Logout(body)).map_err(|_| AuthErrorCode::InvalidInput);
    run(policy, tenant, runtime, database, query, headers, (AccountTermination::Logout, operation))
        .await
}

async fn revoke(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    Extension(runtime): Extension<Arc<Runtime>>,
    database: Option<Extension<GlobalDatabase>>,
    query: Result<Query<ScopeQuery>, QueryRejection>,
    headers: HeaderMap,
    account: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, FlowError> {
    let operation = account
        .map(|Path(account)| Operation::Revoke(account))
        .map_err(|_| AuthErrorCode::InvalidInput);
    run(policy, tenant, runtime, database, query, headers, (AccountTermination::Revoke, operation))
        .await
}

async fn run(
    policy: PolicyFile,
    tenant: TenantScope,
    runtime: Arc<Runtime>,
    database: Option<Extension<GlobalDatabase>>,
    query: Result<Query<ScopeQuery>, QueryRejection>,
    headers: HeaderMap,
    (action, operation): (AccountTermination, Result<Operation, AuthErrorCode>),
) -> Result<StatusCode, FlowError> {
    let Query(ScopeQuery { host_id, service }) = query.map_err(|_| AuthErrorCode::InvalidInput)?;
    validate_identity(&host_id).map_err(|_| AuthErrorCode::InvalidInput)?;
    // Keep the transaction alive after it starts, even if the HTTP client disconnects.
    tokio::task::spawn_blocking(move || {
        let host = authorize_auth_account_termination(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &host_id,
            service,
            action,
        )
        .map_err(FlowError::access)?;
        let operation = operation?;
        if let Operation::Logout(LogoutRequest::Account { account_id })
        | Operation::Revoke(account_id) = &operation
        {
            validate_identity(account_id).map_err(|_| AuthErrorCode::InvalidInput)?;
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (host, runtime, database);
            Err(AuthErrorCode::Unsupported.into())
        }
        #[cfg(target_os = "macos")]
        {
            let scope = AuthAccountScope {
                tenant: &tenant,
                host: &host.id,
                principal: &host.principal,
                service,
            };
            if matches!(&operation, Operation::Revoke(_)) {
                check_revocation_support(scope)?;
            }
            if let Operation::Logout(LogoutRequest::Default {}) = &operation {
                use super::agent_apps_auth_default::supports_default;
                if !supports_default(service, true) {
                    return Err(AuthErrorCode::Unsupported.into());
                }
                if !supports_default(service, host.owns_native_slot) {
                    return Err(AuthErrorCode::Forbidden.into());
                }
            }

            let database = match database {
                Some(Extension(database)) => database,
                None => GlobalDatabase::for_home(&crate::global::paths().home)
                    .map_err(|_| AuthErrorCode::StorageUnavailable)?,
            };
            let accounts = AuthAccountStore::new(database, &host.credential_root)
                .map_err(|_| AuthErrorCode::StorageUnavailable)?;

            match operation {
                Operation::Logout(request) => {
                    logout_accounts(&accounts, &runtime, scope, &host, request.selection())?;
                }
                Operation::Revoke(account_id) => {
                    let transactions = native_transactions(&host.credential_root, scope.host)?;
                    let target = DshAccount::new(&host.home, &transactions, scope)?;
                    tokio::runtime::Handle::current().block_on(accounts.revoke_account(
                        &runtime.flows,
                        scope,
                        &account_id,
                        Some(target),
                        "en-US",
                        0,
                    ))?;
                }
            }
            Ok(StatusCode::NO_CONTENT)
        }
    })
    .await
    .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?
}

#[cfg(target_os = "macos")]
impl LogoutRequest {
    fn selection(&self) -> LogoutSelection<'_> {
        match self {
            Self::Account { account_id } => LogoutSelection::Account(account_id),
            Self::Default {} => LogoutSelection::Default,
            Self::All {} => LogoutSelection::All,
        }
    }
}

#[cfg(target_os = "macos")]
fn logout_accounts(
    accounts: &AuthAccountStore,
    runtime: &Runtime,
    scope: AuthAccountScope<'_>,
    host: &AuthAccountHost,
    selection: LogoutSelection<'_>,
) -> Result<(), AuthErrorCode> {
    let transactions;
    let kimi_transactions;
    let profiles;
    let targets;
    let native = match scope.service {
        AuthService::AgyOauth => Some(NativeAccountLogout::Agy(AgyNativeTarget {
            home: &host.home,
            exclusive_owner: Some(scope),
        })),
        AuthService::DshAccount => {
            transactions = native_transactions(&host.credential_root, scope.host)?;
            Some(NativeAccountLogout::Dsh(DshAccount::new(&host.home, &transactions, scope)?))
        }
        AuthService::KimiOauth if host.owns_native_slot => {
            kimi_transactions = Arc::new(native_transactions(&host.credential_root, scope.host)?);
            let config = kimi::read(&host.home).map_err(|_| AuthErrorCode::StorageUnavailable)?;
            profiles = kimi::declarations(config.as_deref())
                .map_err(|_| AuthErrorCode::StorageUnavailable)?;
            targets = profiles
                .iter()
                .filter(|(_, profile)| kimi::official(&profile.declaration["provider"]))
                .map(|(provider_id, _)| KimiNativeTarget {
                    home: &host.home,
                    transactions: &kimi_transactions,
                    provider_id,
                })
                .collect::<Vec<_>>();
            Some(NativeAccountLogout::Kimi { targets: &targets, exclusive_owner: scope })
        }
        _ => None,
    };
    let result = accounts.logout_accounts(&runtime.flows, scope, selection, native);
    if result == Err(AuthErrorCode::ExecutionHostUnavailable)
        && scope.service == AuthService::ClaudeOfficial
        && matches!(selection, LogoutSelection::All)
    {
        tokio::runtime::Handle::current().block_on(runtime.wait_for_claude_logout(scope))?;
        return accounts.logout_accounts(&runtime.flows, scope, selection, None);
    }
    result
}

#[cfg(target_os = "macos")]
fn native_transactions(
    root: &std::path::Path,
    host: &str,
) -> Result<ProviderTransactions, AuthErrorCode> {
    let id = ExecutionHostId::new(host).map_err(|_| AuthErrorCode::InvalidInput)?;
    ProviderTransactions::new(root, id).map_err(|_| AuthErrorCode::StorageUnavailable)
}

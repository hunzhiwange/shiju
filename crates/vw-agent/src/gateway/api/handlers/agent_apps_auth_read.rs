//! The versioned, management-only view of scoped authorization accounts.

use super::{
    agent_apps::{AuthAccountHost, authorize_auth_account_read},
    agent_apps_auth_start::{Runtime, routes},
};
#[cfg(target_os = "macos")]
use crate::app_providers::AppProviderStore;
use crate::{
    agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore},
    global_database::GlobalDatabase,
};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Query, Request, rejection::QueryRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, put},
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
#[cfg(target_os = "macos")]
use vw_agent_apps::{
    auth::{
        agy_account::{
            AgyNativeTarget, read_accounts as read_agy_accounts,
            require_account as require_agy_account,
        },
        dsh_account::DshAccount,
        kimi_account::KimiNativeTarget,
    },
    providers::live::commit::{ExecutionHostId, ProviderTransactions},
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::{
        AuthAccount, AuthCapabilities, AuthError, AuthErrorCode, AuthService, AuthStatus,
    },
    id::TenantScope,
};

#[derive(Clone)]
pub(super) struct PolicyFile(pub(super) Option<PathBuf>);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopeQuery {
    host_id: String,
    service: AuthService,
    account_id: Option<String>,
}

#[derive(Serialize)]
struct Binding {
    provider_id: String,
    service: AuthService,
    account_id: String,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ReadResult {
    Accounts { accounts: Vec<AuthAccount> },
    Status(AuthStatus),
    Bindings { bindings: Vec<Binding> },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Resource {
    Accounts,
    Status,
    Bindings,
}

pub(crate) fn router(
    password: Option<&'static str>,
    username: &'static str,
    policy: Option<PathBuf>,
) -> Router {
    router_with_runtime(password, username, policy, Arc::new(Runtime::default()))
}

pub(super) fn router_with_runtime(
    password: Option<&'static str>,
    username: &'static str,
    policy: Option<PathBuf>,
    runtime: Arc<Runtime>,
) -> Router {
    Router::new()
        .merge(routes(runtime.clone()))
        .merge(super::agent_apps_auth_logout::routes(runtime))
        .route("/v1/agent-apps/auth/accounts", get(read).layer(Extension(Resource::Accounts)))
        .route("/v1/agent-apps/auth/status", get(read).layer(Extension(Resource::Status)))
        .route("/v1/agent-apps/auth/bindings", get(read).layer(Extension(Resource::Bindings)))
        .route(
            "/v1/agent-apps/auth/bindings/{provider_id}",
            put(super::agent_apps_auth_binding::put).layer(DefaultBodyLimit::max(4 * 1024)),
        )
        .route(
            "/v1/agent-apps/auth/accounts/{account_id}/default",
            put(super::agent_apps_auth_default::put),
        )
        .route(
            "/v1/agent-apps/auth/accounts/{account_id}",
            delete(super::agent_apps_auth_remove::delete),
        )
        .layer(Extension(PolicyFile(policy)))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
        .layer(middleware::from_fn(move |request: Request, next: Next| async move {
            if password.is_none_or(str::is_empty) {
                return ReadError(AuthErrorCode::Unauthenticated).into_response();
            }
            crate::gateway::middleware::basic_auth_middleware_with_credentials(
                request, next, password, username,
            )
            .await
        }))
        .layer(middleware::from_fn(no_store))
}

async fn no_store(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    if !response.headers().contains_key(header::CACHE_CONTROL) {
        let code = match response.status() {
            StatusCode::UNAUTHORIZED => Some(AuthErrorCode::Unauthenticated),
            StatusCode::BAD_REQUEST => Some(AuthErrorCode::InvalidInput),
            _ => None,
        };
        if let Some(code) = code {
            let challenge = response.headers().get(header::WWW_AUTHENTICATE).cloned();
            response = ReadError(code).into_response();
            if let Some(challenge) = challenge {
                response.headers_mut().insert(header::WWW_AUTHENTICATE, challenge);
            }
        }
        response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
}

async fn read(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Extension(resource): Extension<Resource>,
    headers: HeaderMap,
    query: Result<Query<ScopeQuery>, QueryRejection>,
) -> Result<Json<ReadResult>, ReadError> {
    let Query(query) = query.map_err(|_| ReadError(AuthErrorCode::InvalidInput))?;
    vw_agent_apps::providers::validate_identity(&query.host_id)
        .map_err(|_| ReadError(AuthErrorCode::InvalidInput))?;
    if let Some(account_id) = &query.account_id {
        if resource != Resource::Bindings {
            return Err(ReadError(AuthErrorCode::InvalidInput));
        }
        vw_agent_apps::providers::validate_identity(account_id)
            .map_err(|_| ReadError(AuthErrorCode::InvalidInput))?;
    }
    let response = tokio::task::spawn_blocking(move || {
        let host = authorize_auth_account_read(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &query.host_id,
            query.service,
            resource == Resource::Bindings,
        )
        .map_err(ReadError::access)?;
        let scope = AuthAccountScope {
            tenant: &tenant,
            host: &host.id,
            service: query.service,
            principal: &host.principal,
        };
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| AuthErrorCode::StorageUnavailable)?,
        };
        let accounts = AuthAccountStore::new(database.clone(), &host.credential_root)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        query_resource(&accounts, &host, scope, database, resource, query.account_id.as_deref())
            .map_err(ReadError)
    })
    .await
    .map_err(|_| ReadError(AuthErrorCode::StorageUnavailable))??;
    Ok(Json(response))
}

fn query_resource(
    accounts: &AuthAccountStore,
    host: &AuthAccountHost,
    scope: AuthAccountScope<'_>,
    database: GlobalDatabase,
    resource: Resource,
    account_id: Option<&str>,
) -> Result<ReadResult, AuthErrorCode> {
    if resource == Resource::Bindings {
        if let Some(id) = account_id {
            require_binding_account(accounts, host, scope, id)?;
        }
        return Ok(ReadResult::Bindings {
            bindings: read_bindings(host, scope, database, account_id)?,
        });
    }
    // These services have no default. A capabilities query does not need their credentials.
    let accounts_view = if resource == Resource::Status
        && matches!(scope.service, AuthService::ClaudeOfficial | AuthService::DshAccount)
    {
        Vec::new()
    } else {
        read_accounts(accounts, scope, host)?
    };
    if resource == Resource::Accounts {
        return Ok(ReadResult::Accounts { accounts: accounts_view });
    }
    Ok(ReadResult::Status(AuthStatus {
        service: scope.service,
        default_account_id: accounts_view
            .into_iter()
            .find(|account| account.is_default)
            .map(|account| account.account_id),
        // Each mutation capability requires its own grant.
        capabilities: AuthCapabilities {
            login_methods: Vec::new(),
            can_set_default: host.can_set_default
                && super::agent_apps_auth_default::supports_default(
                    scope.service,
                    host.owns_native_slot,
                ),
            can_bind: false,
            can_revoke: cfg!(target_os = "macos")
                && scope.service == AuthService::DshAccount
                && host.owns_native_slot
                && host.can_revoke,
        },
    }))
}

fn require_binding_account(
    accounts: &AuthAccountStore,
    host: &AuthAccountHost,
    scope: AuthAccountScope<'_>,
    id: &str,
) -> Result<(), AuthErrorCode> {
    match scope.service {
        #[cfg(target_os = "macos")]
        AuthService::AgyOauth => require_agy_account(
            scope,
            AgyNativeTarget { home: &host.home, exclusive_owner: Some(scope) },
            id,
        ),
        #[cfg(target_os = "macos")]
        AuthService::DshAccount => {
            let transactions = native_transactions(host)?;
            DshAccount::require_account(&host.home, transactions.as_deref(), scope, id)
        }
        #[cfg(not(target_os = "macos"))]
        AuthService::AgyOauth | AuthService::DshAccount => {
            let _ = host;
            Err(AuthErrorCode::Unsupported)
        }
        _ => accounts.read_metadata(&scope, id).map(|_| ()),
    }
}

fn read_accounts(
    store: &AuthAccountStore,
    scope: AuthAccountScope<'_>,
    host: &AuthAccountHost,
) -> Result<Vec<AuthAccount>, AuthErrorCode> {
    if matches!(scope.service, AuthService::AgyOauth | AuthService::DshAccount) {
        return read_native_accounts(scope, host);
    }
    let mut accounts = store.read_managed_accounts(scope, &host.home)?;
    if scope.service == AuthService::KimiOauth && host.owns_native_slot {
        apply_kimi_default(store, scope, host, &mut accounts)?;
    }
    Ok(accounts)
}

#[cfg(target_os = "macos")]
fn read_native_accounts(
    scope: AuthAccountScope<'_>,
    host: &AuthAccountHost,
) -> Result<Vec<AuthAccount>, AuthErrorCode> {
    match scope.service {
        AuthService::AgyOauth => read_agy_accounts(
            scope,
            AgyNativeTarget { home: &host.home, exclusive_owner: Some(scope) },
        ),
        AuthService::DshAccount => {
            let transactions = native_transactions(host)?;
            Ok(tokio::runtime::Handle::current()
                .block_on(DshAccount::read_account(&host.home, transactions.as_deref(), scope))?
                .into_iter()
                .collect())
        }
        _ => Err(AuthErrorCode::Unsupported),
    }
}

#[cfg(not(target_os = "macos"))]
fn read_native_accounts(
    _scope: AuthAccountScope<'_>,
    _host: &AuthAccountHost,
) -> Result<Vec<AuthAccount>, AuthErrorCode> {
    Err(AuthErrorCode::Unsupported)
}

#[cfg(target_os = "macos")]
fn native_transactions(
    host: &AuthAccountHost,
) -> Result<Option<std::sync::Arc<ProviderTransactions>>, AuthErrorCode> {
    let id = ExecutionHostId::new(&host.id).map_err(|_| AuthErrorCode::StorageUnavailable)?;
    ProviderTransactions::open_existing(&host.credential_root, id)
        .map(|transactions| transactions.map(std::sync::Arc::new))
        .map_err(|_| AuthErrorCode::StorageUnavailable)
}

#[cfg(target_os = "macos")]
fn apply_kimi_default(
    store: &AuthAccountStore,
    scope: AuthAccountScope<'_>,
    host: &AuthAccountHost,
    accounts: &mut [AuthAccount],
) -> Result<(), AuthErrorCode> {
    use vw_agent_apps::providers::kimi;
    if accounts.is_empty() {
        return Ok(());
    }
    let Some(transactions) = native_transactions(host)? else {
        return Ok(());
    };
    let config = kimi::read(&host.home).map_err(|_| AuthErrorCode::StorageUnavailable)?;
    let providers =
        kimi::declarations(config.as_deref()).map_err(|_| AuthErrorCode::StorageUnavailable)?;
    let ids = accounts.iter().map(|account| account.account_id.clone()).collect::<Vec<_>>();
    let mut default = None;
    for (provider_id, provider) in &providers {
        if !kimi::official(&provider.declaration["provider"]) {
            continue;
        }
        let native =
            KimiNativeTarget { home: &host.home, transactions: &transactions, provider_id };
        if let Some(id) = store.read_kimi_default(scope, &native, &ids)?
            && default.replace(id).is_some()
        {
            return Err(AuthErrorCode::Conflict);
        }
    }
    if let Some(id) = default
        && let Some(account) = accounts.iter_mut().find(|account| account.account_id == id)
    {
        account.is_default = true;
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn apply_kimi_default(
    _store: &AuthAccountStore,
    _scope: AuthAccountScope<'_>,
    _host: &AuthAccountHost,
    _accounts: &mut [AuthAccount],
) -> Result<(), AuthErrorCode> {
    Err(AuthErrorCode::Unsupported)
}

fn read_bindings(
    host: &AuthAccountHost,
    scope: AuthAccountScope<'_>,
    database: GlobalDatabase,
    account_filter: Option<&str>,
) -> Result<Vec<Binding>, AuthErrorCode> {
    let app = match scope.service {
        AuthService::CodexOauth => AppId::Codex,
        AuthService::ClaudeOfficial => AppId::Claude,
        AuthService::AgyOauth => AppId::Agy,
        AuthService::KimiOauth => AppId::Kimi,
        _ => return Ok(Vec::new()),
    };
    #[cfg(target_os = "macos")]
    {
        let providers =
            AppProviderStore::new(database).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let transactions = if app == AppId::Kimi {
            let Some(transactions) = native_transactions(host)? else {
                return Ok(Vec::new());
            };
            Some(transactions)
        } else {
            None
        };
        let mut bindings = Vec::new();
        for provider in
            providers.list(scope.tenant, app).map_err(|_| AuthErrorCode::StorageUnavailable)?
        {
            let id = &provider.id;
            let result = provider_binding(&providers, host, scope, app, id, transactions.as_ref());
            match result {
                Ok(Some(account_id))
                    if account_filter.is_none_or(|filter| filter == account_id) =>
                {
                    bindings.push(Binding {
                        provider_id: provider.id,
                        service: scope.service,
                        account_id,
                    });
                }
                Ok(_)
                | Err(
                    AuthErrorCode::Forbidden | AuthErrorCode::NotFound | AuthErrorCode::Unsupported,
                ) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(bindings)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (host, app, database, account_filter);
        Err(AuthErrorCode::Unsupported)
    }
}

#[cfg(target_os = "macos")]
pub(super) fn require_provider_account(
    database: GlobalDatabase,
    host: &AuthAccountHost,
    scope: AuthAccountScope<'_>,
    app: AppId,
    provider: &str,
    account: &str,
) -> Result<(), AuthErrorCode> {
    let providers =
        AppProviderStore::new(database.clone()).map_err(|_| AuthErrorCode::StorageUnavailable)?;
    let transactions = if app == AppId::Kimi {
        Some(native_transactions(host)?.ok_or(AuthErrorCode::NotFound)?)
    } else {
        None
    };
    let bound = provider_binding(&providers, host, scope, app, provider, transactions.as_ref())?;
    if bound.as_deref() != Some(account) {
        return Err(AuthErrorCode::NotFound);
    }
    let accounts = AuthAccountStore::new(database, &host.credential_root)
        .map_err(|_| AuthErrorCode::StorageUnavailable)?;
    require_binding_account(&accounts, host, scope, account)
}

#[cfg(target_os = "macos")]
fn provider_binding(
    providers: &AppProviderStore,
    host: &AuthAccountHost,
    scope: AuthAccountScope<'_>,
    app: AppId,
    provider: &str,
    transactions: Option<&Arc<ProviderTransactions>>,
) -> Result<Option<String>, AuthErrorCode> {
    match app {
        AppId::Codex => providers.codex_account_binding(scope, provider),
        AppId::Claude => providers.claude_account_binding(scope, provider),
        AppId::Agy => providers.agy_account_binding(scope, provider),
        AppId::Kimi => providers.kimi_account_binding(
            scope,
            &host.credential_root,
            &KimiNativeTarget {
                home: &host.home,
                transactions: transactions.ok_or(AuthErrorCode::Unsupported)?,
                provider_id: provider,
            },
        ),
        _ => Err(AuthErrorCode::Unsupported),
    }
}

struct ReadError(AuthErrorCode);

impl ReadError {
    fn access(status: StatusCode) -> Self {
        Self(match status {
            StatusCode::UNAUTHORIZED => AuthErrorCode::Unauthenticated,
            StatusCode::FORBIDDEN => AuthErrorCode::Forbidden,
            StatusCode::NOT_FOUND => AuthErrorCode::NotFound,
            StatusCode::NOT_IMPLEMENTED => AuthErrorCode::ExecutionChannelUnavailable,
            _ => AuthErrorCode::ExecutionHostUnavailable,
        })
    }
}

impl From<AuthErrorCode> for ReadError {
    fn from(code: AuthErrorCode) -> Self {
        Self(code)
    }
}

impl IntoResponse for ReadError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            AuthErrorCode::InvalidInput => (StatusCode::BAD_REQUEST, "Invalid account query"),
            AuthErrorCode::Unauthenticated => (StatusCode::UNAUTHORIZED, "Authentication required"),
            AuthErrorCode::Forbidden => (StatusCode::FORBIDDEN, "Account query forbidden"),
            AuthErrorCode::NotFound => (StatusCode::NOT_FOUND, "Account resource not found"),
            AuthErrorCode::Unsupported | AuthErrorCode::ExecutionChannelUnavailable => {
                (StatusCode::NOT_IMPLEMENTED, "Account query unavailable on this host")
            }
            AuthErrorCode::StorageUnavailable
            | AuthErrorCode::ExecutionHostUnavailable
            | AuthErrorCode::KeychainUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, "Account state unavailable")
            }
            AuthErrorCode::UpstreamFailure => {
                (StatusCode::BAD_GATEWAY, "Account provider unavailable")
            }
            _ => (StatusCode::CONFLICT, "Account state changed or is unavailable"),
        };
        (
            status,
            [(header::CACHE_CONTROL, "no-store")],
            Json(AuthError {
                code: self.0,
                message: message.into(),
                correlation_id: uuid::Uuid::new_v4().to_string(),
            }),
        )
            .into_response()
    }
}

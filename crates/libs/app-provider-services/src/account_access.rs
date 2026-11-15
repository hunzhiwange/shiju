use agent_database::GlobalDatabase;
use agent_app_services::auth_accounts::{AuthAccountScope, AuthAccountStore};
use agent_app_services::auth_host::AuthAccountHost;
use vw_api_app_auth::{AuthErrorCode, AuthService};
#[cfg(target_os = "macos")]
use {crate::AppProviderStore, std::sync::Arc, vw_api_app_identity::AppId};
#[cfg(target_os = "macos")]
use vw_agent_apps::{auth::{agy_account::{AgyNativeTarget, require_account as require_agy_account}, dsh_account::DshAccount, kimi_account::KimiNativeTarget}, providers::live::commit::{ExecutionHostId, ProviderTransactions}};
pub fn require_binding_account(
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


#[cfg(target_os = "macos")]
pub fn native_transactions(
    host: &AuthAccountHost,
) -> Result<Option<std::sync::Arc<ProviderTransactions>>, AuthErrorCode> {
    let id = ExecutionHostId::new(&host.id).map_err(|_| AuthErrorCode::StorageUnavailable)?;
    ProviderTransactions::open_existing(&host.credential_root, id)
        .map(|transactions| transactions.map(std::sync::Arc::new))
        .map_err(|_| AuthErrorCode::StorageUnavailable)
}


#[cfg(target_os = "macos")]
pub fn require_provider_account(
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
pub fn provider_binding(
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

use super::{Execution, Runtime, StartSender, drive, started};
use crate::{
    agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountStore},
    global_database::run_sync_database,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::oneshot;
use vw_agent_apps::{
    apps::discovery::read::locate_cli,
    auth::{
        agy_account::AgyNativeTarget,
        agy_login::AgyLogin,
        claude_keychain::{ClaudeKeychain, MacOsKeychain},
        claude_login::{ClaudeLogin, ClaudeLoginError, ClaudePaste},
        credential_file::{AccountCredential, CredentialFileStore},
        dsh_account::DshAccount,
        flow::{AuthAccountScope, AuthFlowTiming, AuthFlows},
    },
    providers::{
        claude_login::LoginMaterial,
        live::commit::{ExecutionHostId, ProviderTransactions},
    },
};
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::{
        AuthAccount, AuthCredentialState, AuthErrorCode, AuthFlow, AuthFlowId, AuthService,
        AuthStartRequest, AuthStartResponse,
    },
};

const CLAUDE_LIFETIME: Duration = Duration::from_secs(600);

#[derive(Default)]
pub(super) struct ClaudePastes(Mutex<HashMap<String, ClaudePaste>>);

impl ClaudePastes {
    pub(super) fn get(&self, id: &AuthFlowId) -> Result<ClaudePaste, AuthErrorCode> {
        self.0
            .lock()
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?
            .get(&id.0)
            .cloned()
            .ok_or(AuthErrorCode::Conflict)
    }
}

pub(super) async fn start_agy(
    flows: &AuthFlows,
    execution: &Execution,
    input: AuthStartRequest,
    browser_local: bool,
    reply: &mut StartSender,
) -> Result<(), AuthErrorCode> {
    let scope = execution.scope();
    let target =
        AgyNativeTarget { home: &execution.host.environment.home, exclusive_owner: Some(scope) };
    let login = AgyLogin::start_on_host(flows, scope, input, browser_local, target)?;
    drive(flows, scope, login.response.clone(), reply, || login.poll(flows, scope)).await;
    Ok(())
}

pub(super) async fn start_dsh(
    flows: &AuthFlows,
    execution: &Execution,
    input: AuthStartRequest,
    browser_local: bool,
    reply: &mut StartSender,
) -> Result<(), AuthErrorCode> {
    let scope = execution.scope();
    let host = ExecutionHostId::new(scope.host).map_err(|_| AuthErrorCode::InvalidInput)?;
    let transactions = ProviderTransactions::new(&execution.host.credential_root, host)
        .map_err(|_| AuthErrorCode::StorageUnavailable)?;
    let account = DshAccount::new(&execution.host.environment.home, &transactions, scope)?;
    let login = account.start_login(flows, input, browser_local, "en-US", 0).await?;
    drive(flows, scope, login.response().clone(), reply, || login.poll(flows)).await;
    Ok(())
}

pub(super) async fn start_claude(
    runtime: &Runtime,
    execution: &Execution,
    accounts: &AuthAccountStore,
    input: AuthStartRequest,
    reply: &mut StartSender,
) -> Result<(), AuthErrorCode> {
    let scope = execution.scope();
    let environment = &execution.host.environment;
    let credentials = CredentialFileStore::new(&execution.host.credential_root)
        .map_err(|_| AuthErrorCode::StorageUnavailable)?;
    let account_id = uuid::Uuid::new_v4().simple().to_string();
    let (program, directory) = blocking(|| {
        MacOsKeychain
            .authorize_home(&environment.home)
            .map_err(|_| AuthErrorCode::KeychainUnavailable)?;
        let program = locate_cli(AppId::Claude, environment)
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        let directory = prepare_claude(&credentials, scope, &account_id)?;
        Ok((program, directory))
    })?;

    let expires_at = TimestampMs(now()?.0 + CLAUDE_LIFETIME.as_millis() as i64);
    let deadline = Instant::now() + CLAUDE_LIFETIME;
    let login =
        match ClaudeLogin::start(&program, &credentials, scope, &account_id, CLAUDE_LIFETIME).await
        {
            Ok(login) => login,
            Err(error) => {
                blocking(|| {
                    credentials.discard_unpublished_claude(scope, &account_id, &MacOsKeychain)
                })?;
                return Err(paste_error(error));
            }
        };

    let (cancel, cancelled) = oneshot::channel();
    let mut cancel = Some(cancel);
    let stopped = Arc::new(AtomicBool::new(false));
    let cleanup_stopped = Arc::clone(&stopped);
    let flows = &runtime.flows;
    let pending = flows.start_with_cleanup(
        scope,
        input,
        AuthFlowTiming { deadline, expires_at, retry_after_seconds: 1 },
        move || {
            if let Some(cancel) = cancel.take() {
                let _ = cancel.send(());
            }
            if cleanup_stopped.load(Ordering::Acquire) {
                Ok(())
            } else {
                Err(AuthErrorCode::ExecutionHostUnavailable)
            }
        },
    );
    let flow_id = match pending {
        Ok(AuthFlow::Pending { flow_id, .. }) => flow_id,
        Ok(_) => unreachable!("start registers pending flows"),
        Err(error) => {
            let _ = login.cancel().await;
            blocking(|| {
                credentials.discard_unpublished_claude(scope, &account_id, &MacOsKeychain)
            })?;
            return Err(error);
        }
    };

    let response = AuthStartResponse::Pending {
        flow_id: flow_id.clone(),
        expires_at,
        retry_after_seconds: 1,
        verification_url: login.authorization_url().into(),
        user_code: None,
        can_paste: Some(true),
    };
    let registered = runtime
        .claude
        .0
        .lock()
        .map(|mut handles| {
            handles.insert(flow_id.0.clone(), login.paste_handle());
        })
        .is_ok();

    let result = if registered && started(reply, response) {
        login.wait_until_cancelled(cancelled).await.map_err(paste_error)
    } else {
        let _ = login.cancel().await;
        Err(AuthErrorCode::ExecutionHostUnavailable)
    };
    if let Ok(mut handles) = runtime.claude.0.lock() {
        handles.remove(&flow_id.0);
    }
    let result = result.and_then(|()| blocking(|| verified_claude(&directory, &account_id)));
    let (result, metadata) = match result {
        Ok((account, metadata)) => (Ok(account), Some(metadata)),
        Err(error) => (Err(error), None),
    };

    let finished = blocking(|| {
        let snapshot =
            flows.complete(scope, &flow_id, Instant::now, result, |scope, _, account| {
                let metadata = metadata.as_ref().ok_or(AuthErrorCode::StorageUnavailable)?;
                accounts.insert_metadata(&scope, &account.account_id, metadata)
            });
        if !matches!(snapshot, Ok(AuthFlow::Approved { .. })) {
            match accounts.read_metadata(&scope, &account_id) {
                Err(AuthErrorCode::NotFound) => {
                    credentials.discard_unpublished_claude(scope, &account_id, &MacOsKeychain)?
                }
                Ok(_) => {} // Publication can succeed before a database error is reported.
                Err(error) => return Err(error),
            }
        }
        Ok(())
    });
    if finished.is_ok() {
        stopped.store(true, Ordering::Release);
    }
    let _ = blocking(|| flows.reap_expired(Instant::now()));
    if !registered {
        return Err(AuthErrorCode::ExecutionHostUnavailable);
    }
    finished
}

fn prepare_claude(
    credentials: &CredentialFileStore,
    scope: AuthAccountScope<'_>,
    id: &str,
) -> Result<PathBuf, AuthErrorCode> {
    let credentials =
        credentials.for_owner(scope.principal).map_err(|_| AuthErrorCode::StorageUnavailable)?;
    credentials
        .register_claude_directory(scope.tenant, scope.host, id)
        .map_err(|_| AuthErrorCode::StorageUnavailable)?;
    let AccountCredential::ClaudeDirectory(directory) = credentials
        .read(scope.tenant, scope.host, AppId::Claude, id)
        .map_err(|_| AuthErrorCode::StorageUnavailable)?
    else {
        return Err(AuthErrorCode::StorageUnavailable);
    };
    if MacOsKeychain
        .read(Some(directory.path()))
        .map_err(|_| AuthErrorCode::KeychainUnavailable)?
        .is_some()
    {
        // The native namespace is a truncated hash. Never adopt a colliding item.
        return Err(AuthErrorCode::Conflict);
    }
    Ok(directory.path().to_owned())
}

fn verified_claude(
    directory: &Path,
    account_id: &str,
) -> Result<(AuthAccount, AuthAccountMetadata), AuthErrorCode> {
    let material = LoginMaterial::from_account(directory, &MacOsKeychain)
        .map_err(|_| AuthErrorCode::StorageUnavailable)?;
    let identity = material
        .identity()
        .map_err(|_| AuthErrorCode::IdentityMismatch)?
        .ok_or(AuthErrorCode::ReauthRequired)?;
    let authenticated_at = now()?;
    let metadata = AuthAccountMetadata::new(&identity, authenticated_at)?;
    Ok((
        AuthAccount {
            account_id: account_id.into(),
            service: AuthService::ClaudeOfficial,
            label: "Claude".into(),
            credential_state: AuthCredentialState::Ready,
            is_default: false,
            authenticated_at: Some(authenticated_at),
            login_expires_at: None,
            github_domain: None,
            region: None,
        },
        metadata,
    ))
}

fn now() -> Result<TimestampMs, AuthErrorCode> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
        .map(TimestampMs)
        .ok_or(AuthErrorCode::ExecutionHostUnavailable)
}

fn blocking<T: Send>(
    op: impl FnOnce() -> Result<T, AuthErrorCode> + Send,
) -> Result<T, AuthErrorCode> {
    run_sync_database(|| Ok(op())).map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?
}

pub(super) fn paste_error(error: ClaudeLoginError) -> AuthErrorCode {
    match error {
        ClaudeLoginError::InvalidPaste => AuthErrorCode::InvalidInput,
        ClaudeLoginError::Cancelled | ClaudeLoginError::TimedOut | ClaudeLoginError::Inactive => {
            AuthErrorCode::Conflict
        }
        ClaudeLoginError::InvalidAccount | ClaudeLoginError::Unavailable => {
            AuthErrorCode::ExecutionHostUnavailable
        }
        ClaudeLoginError::LinkUnavailable | ClaudeLoginError::Exited => {
            AuthErrorCode::UpstreamFailure
        }
    }
}

//! Local gateway account verification and revocation for employee chat streams.
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{
        LazyLock,
        atomic::{AtomicU64, Ordering},
    },
};

use axum::{
    Extension,
    extract::ConnectInfo,
    http::{HeaderMap, StatusCode},
};
use gateway_session::stream_host::EmployeeAuthorization;
use gateway_shiju_account::{SHIJU_CREDENTIAL_HEADER, ShijuCredential};
use gateway_types::ApiError;
use sha2::{Digest, Sha256};
use shiju_client::account::ShijuAccountClient;
use tokio_util::sync::CancellationToken;
use vw_api_transport::{DEFAULT_USER_ID, TenantScope};

/// Installed by the server only on a direct loopback listener with tunneling disabled.
#[derive(Clone, Copy)]
pub(crate) struct LocalEmployeeGateway;

type Fingerprint = [u8; 32];

#[derive(Default)]
struct Account {
    current: Option<ActiveCredential>,
    revoked: HashSet<Fingerprint>,
    last_verified: u64,
}

struct ActiveCredential {
    fingerprint: Fingerprint,
    generation: u64,
    revoked: CancellationToken,
}

static ACCOUNTS: LazyLock<parking_lot::Mutex<HashMap<TenantScope, Account>>> =
    LazyLock::new(Default::default);
static NEXT_ATTEMPT: AtomicU64 = AtomicU64::new(1);

pub(super) fn allowed(
    local: Option<Extension<LocalEmployeeGateway>>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: &HeaderMap,
) -> bool {
    local.is_some()
        && peer.is_some_and(|Extension(ConnectInfo(peer))| peer.ip().is_loopback())
        && !headers.keys().any(|key| {
            key == axum::http::header::FORWARDED || key.as_str().starts_with("x-forwarded-")
        })
}

fn unauthorized() -> ApiError {
    ApiError {
        status: StatusCode::UNAUTHORIZED,
        message: "Employee ACP requires the current SHIJU account".into(),
    }
}

fn credential(tenant: &TenantScope, headers: &HeaderMap) -> Result<ShijuCredential, ApiError> {
    if tenant.user_id().as_ref() == DEFAULT_USER_ID {
        return Err(unauthorized());
    }
    let mut values = headers.get_all(SHIJU_CREDENTIAL_HEADER).iter();
    let value = values.next().and_then(|value| value.to_str().ok()).ok_or_else(unauthorized)?;
    if values.next().is_some() {
        return Err(unauthorized());
    }
    ShijuCredential::new(value).map_err(|_| unauthorized())
}

fn fingerprint(credential: &ShijuCredential) -> Fingerprint {
    Sha256::digest(credential.expose().as_bytes()).into()
}

#[cfg(test)]
tokio::task_local! {
    pub(super) static TEST_CLIENT: ShijuAccountClient;
}

pub(super) async fn authorize(
    tenant: &TenantScope,
    headers: &HeaderMap,
) -> Result<EmployeeAuthorization, ApiError> {
    let credential = credential(tenant, headers)?;
    #[cfg(test)]
    if let Ok(client) = TEST_CLIENT.try_with(Clone::clone) {
        return verify(tenant, credential, &client).await;
    }
    let client = ShijuAccountClient::for_employee_acp()
        .map_err(|_| ApiError::internal("SHIJU account verification unavailable"))?;
    verify(tenant, credential, &client).await
}

pub(super) async fn verify(
    tenant: &TenantScope,
    credential: ShijuCredential,
    client: &ShijuAccountClient,
) -> Result<EmployeeAuthorization, ApiError> {
    let attempt = NEXT_ATTEMPT.fetch_add(1, Ordering::Relaxed);
    let fingerprint = fingerprint(&credential);
    let user = match client.current_user(credential.expose()).await {
        Ok(user) => user,
        Err(error) if error.status().is_some_and(|status| matches!(status.as_u16(), 401 | 403)) => {
            revoke(tenant, fingerprint, false);
            return Err(unauthorized());
        }
        Err(_) => {
            return Err(ApiError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                message: "SHIJU account verification unavailable".into(),
            });
        }
    };
    if user.user_id != tenant.user_id().as_ref()
        || user.company_id != tenant.company_id().as_ref()
        || user.status != "enabled"
    {
        revoke(tenant, fingerprint, false);
        return Err(unauthorized());
    }

    let mut accounts = ACCOUNTS.lock();
    let account = accounts.entry(tenant.clone()).or_default();
    if account.revoked.contains(&fingerprint)
        || (attempt < account.last_verified
            && account.current.as_ref().is_none_or(|active| active.fingerprint != fingerprint))
    {
        return Err(unauthorized());
    }
    account.last_verified = account.last_verified.max(attempt);
    if account.current.as_ref().is_none_or(|current| current.fingerprint != fingerprint) {
        if let Some(previous) = account.current.take() {
            account.revoked.insert(previous.fingerprint);
            previous.revoked.cancel();
        }
        account.current = Some(ActiveCredential {
            fingerprint,
            generation: attempt,
            revoked: CancellationToken::new(),
        });
    }
    let active = account.current.as_ref().expect("verified credential is active");
    Ok(EmployeeAuthorization {
        credential: credential.expose().to_owned(),
        generation: active.generation,
        revoked: active.revoked.clone(),
        stop: CancellationToken::new(),
    })
}

fn revoke(tenant: &TenantScope, fingerprint: Fingerprint, explicit: bool) {
    let mut accounts = ACCOUNTS.lock();
    if !explicit
        && !accounts
            .get(tenant)
            .and_then(|account| account.current.as_ref())
            .is_some_and(|active| active.fingerprint == fingerprint)
    {
        return;
    }
    let account = accounts.entry(tenant.clone()).or_default();
    account.revoked.insert(fingerprint);
    if account.current.as_ref().is_some_and(|active| active.fingerprint == fingerprint) {
        if let Some(active) = account.current.take() {
            active.revoked.cancel();
        }
    }
}

/// The desktop calls this with the outgoing account's authenticated request context.
pub(super) async fn revoke_account(
    Extension(tenant): Extension<TenantScope>,
    local: Option<Extension<LocalEmployeeGateway>>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    if !allowed(local, peer, &headers) {
        return Err(ApiError {
            status: StatusCode::FORBIDDEN,
            message: "Employee ACP requires the local desktop gateway".into(),
        });
    }
    revoke(&tenant, fingerprint(&credential(&tenant, &headers)?), true);
    Ok(StatusCode::NO_CONTENT)
}

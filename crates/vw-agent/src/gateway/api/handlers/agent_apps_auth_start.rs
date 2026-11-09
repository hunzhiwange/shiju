//! Versioned authorization mutations on the management authentication chain.

use super::{
    agent_apps::{AuthStartHost, authorize_auth_flow_start},
    agent_apps_auth_read::PolicyFile,
};
use crate::{
    agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore},
    global_database::GlobalDatabase,
};
use axum::{
    Extension, Json, Router,
    extract::{
        ConnectInfo, DefaultBodyLimit, FromRequest, Path, Query, Request, rejection::QueryRejection,
    },
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use std::{
    future::Future,
    net::SocketAddr,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
use tokio::sync::oneshot;
use vw_agent_apps::{
    auth::{
        codex_browser::CodexBrowserStart, codex_device::CodexDeviceAuth, copilot::CopilotAuth,
        flow::AuthFlows, kimi::KimiDeviceAuth, xai::XaiDeviceAuth,
    },
    providers::validate_identity,
};
use vw_api_types::{
    agent_apps_auth::{
        AuthError, AuthErrorCode, AuthFlow, AuthFlowId, AuthLoginMethod, AuthPasteRequest,
        AuthService, AuthStartRequest, AuthStartResponse,
    },
    id::TenantScope,
};

#[cfg(target_os = "macos")]
mod native;

#[derive(Default)]
pub(super) struct Runtime {
    pub(super) flows: AuthFlows,
    #[cfg(all(test, target_os = "macos"))]
    pub(super) codex_device_oauth: Option<SocketAddr>,
    kimi: OnceLock<Result<KimiDeviceAuth, AuthErrorCode>>,
    #[cfg(target_os = "macos")]
    claude: native::ClaudeAttempts,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ScopeQuery {
    pub(super) host_id: String,
    pub(super) service: AuthService,
}

pub(super) struct Execution {
    tenant: TenantScope,
    host: AuthStartHost,
    service: AuthService,
}

impl Execution {
    pub(super) fn scope(&self) -> AuthAccountScope<'_> {
        AuthAccountScope {
            tenant: &self.tenant,
            principal: &self.host.principal,
            host: &self.host.id,
            service: self.service,
        }
    }

    fn accounts(
        &self,
        database: Option<GlobalDatabase>,
    ) -> Result<AuthAccountStore, AuthErrorCode> {
        let database = match database {
            Some(database) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| AuthErrorCode::StorageUnavailable)?,
        };
        AuthAccountStore::new(database, &self.host.credential_root)
            .map_err(|_| AuthErrorCode::StorageUnavailable)
    }
}

pub(super) fn routes(runtime: Arc<Runtime>) -> Router {
    Router::new()
        .route("/v1/agent-apps/auth/flows", post(start))
        .route(
            "/v1/agent-apps/auth/flows/{flow_id}",
            get(super::agent_apps_auth_poll::poll).delete(super::agent_apps_auth_cancel::cancel),
        )
        .route("/v1/agent-apps/auth/flows/{flow_id}/paste", post(paste))
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(Extension(runtime))
}

pub(super) async fn authorize(
    policy: PolicyFile,
    tenant: TenantScope,
    headers: HeaderMap,
    query: Result<Query<ScopeQuery>, QueryRejection>,
) -> Result<Execution, FlowError> {
    let Query(query) = query.map_err(|_| AuthErrorCode::InvalidInput)?;
    validate_identity(&query.host_id).map_err(|_| AuthErrorCode::InvalidInput)?;
    tokio::task::spawn_blocking(move || {
        let host = authorize_auth_flow_start(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &query.host_id,
            query.service,
        )
        .map_err(FlowError::access)?;
        Ok(Execution { tenant, host, service: query.service })
    })
    .await
    .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?
}

async fn start(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    Extension(runtime): Extension<Arc<Runtime>>,
    database: Option<Extension<GlobalDatabase>>,
    query: Result<Query<ScopeQuery>, QueryRejection>,
    request: Request,
) -> Result<(StatusCode, Json<AuthStartResponse>), FlowError> {
    let browser_peer = local_browser_peer(&request);
    let execution = authorize(policy, tenant, request.headers().clone(), query).await?;
    let Json(input) = Json::<AuthStartRequest>::from_request(request, &())
        .await
        .map_err(|_| AuthErrorCode::InvalidInput)?;
    input.validate_for(execution.service)?;
    let browser_local = browser_peer && execution.host.browser_local;
    if input.method == AuthLoginMethod::Browser
        && matches!(
            execution.service,
            AuthService::CodexOauth | AuthService::AgyOauth | AuthService::DshAccount
        )
        && !browser_local
    {
        return Err(AuthErrorCode::CallbackUnreachable.into());
    }
    #[cfg(not(target_os = "macos"))]
    if matches!(
        execution.service,
        AuthService::ClaudeOfficial | AuthService::AgyOauth | AuthService::DshAccount
    ) {
        return Err(AuthErrorCode::Unsupported.into());
    }
    let execution_service = execution.service;
    let (send, receive) = oneshot::channel();
    tokio::spawn(async move {
        let mut response = Some(send);
        let result = run(
            &runtime,
            &execution,
            database.map(|Extension(db)| db),
            input,
            browser_local,
            &mut response,
        )
        .await;
        if let Some(response) = response {
            let _ =
                response.send(Err(result.err().unwrap_or(AuthErrorCode::ExecutionHostUnavailable)));
        }
    });
    let started = receive
        .await
        .unwrap_or(Err(AuthErrorCode::ExecutionHostUnavailable))
        .map_err(|code| FlowError { code, start_service: Some(execution_service) })?;
    Ok((StatusCode::CREATED, Json(started)))
}

fn local_browser_peer(request: &Request) -> bool {
    let headers = request.headers();
    request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .is_some_and(|peer| peer.0.ip().is_loopback())
        && !headers
            .keys()
            .any(|key| key == header::FORWARDED || key.as_str().starts_with("x-forwarded-"))
}

type StartSender = Option<oneshot::Sender<Result<AuthStartResponse, AuthErrorCode>>>;

async fn run(
    runtime: &Runtime,
    execution: &Execution,
    database: Option<GlobalDatabase>,
    input: AuthStartRequest,
    browser_local: bool,
    reply: &mut StartSender,
) -> Result<(), AuthErrorCode> {
    let scope = execution.scope();
    let flows = &runtime.flows;
    let store = crate::global_database::run_sync_database(|| Ok(execution.accounts(database)))
        .map_err(|_| AuthErrorCode::StorageUnavailable)??;
    if let Some(target) = &input.target_account_id {
        validate_identity(target).map_err(|_| AuthErrorCode::InvalidInput)?;
        let metadata =
            crate::global_database::run_sync_database(|| Ok(store.read_metadata(&scope, target)))
                .map_err(|_| AuthErrorCode::StorageUnavailable)??;
        if scope.service == AuthService::KimiOauth
            && input.region.is_some_and(|region| Some(region) != metadata.region)
        {
            return Err(AuthErrorCode::InvalidInput);
        }
    }

    match (scope.service, input.method) {
        (AuthService::GithubCopilot, AuthLoginMethod::DeviceCode) => {
            let adapter = CopilotAuth::new()?;
            let authorities = execution
                .host
                .github_enterprise_authorities
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let flow = adapter.start(flows, scope, input, &authorities).await?;
            drive(flows, scope, flow.response.clone(), reply, || {
                store.poll_copilot(&adapter, flows, scope, &flow)
            })
            .await;
        }
        (AuthService::CodexOauth, AuthLoginMethod::DeviceCode) => {
            #[cfg(all(test, target_os = "macos"))]
            let adapter = match runtime.codex_device_oauth {
                Some(address) => CodexDeviceAuth::with_loopback_oauth(address),
                None => CodexDeviceAuth::new(),
            }?;
            #[cfg(not(all(test, target_os = "macos")))]
            let adapter = CodexDeviceAuth::new()?;
            let flow = adapter.start(flows, scope, input).await?;
            drive(flows, scope, flow.response.clone(), reply, || {
                store.poll_codex_device(&adapter, flows, scope, &flow)
            })
            .await;
        }
        (AuthService::CodexOauth, AuthLoginMethod::Browser) => {
            let flow = CodexBrowserStart::start(flows, scope, input, browser_local)?;
            drive(flows, scope, flow.response.clone(), reply, || {
                store.poll_codex_browser(flows, scope, &flow)
            })
            .await;
        }
        (AuthService::XaiOauth, AuthLoginMethod::DeviceCode) => {
            let adapter = XaiDeviceAuth::new()?;
            let flow = adapter.start(flows, scope, input).await?;
            drive(flows, scope, flow.response.clone(), reply, || {
                store.poll_xai_device(&adapter, flows, scope, &flow)
            })
            .await;
        }
        (AuthService::KimiOauth, AuthLoginMethod::DeviceCode) => {
            let adapter =
                runtime.kimi.get_or_init(KimiDeviceAuth::new).as_ref().map_err(|code| *code)?;
            let flow = store.start_kimi_device(adapter, flows, scope, input).await?;
            drive(flows, scope, flow.response.clone(), reply, || {
                store.poll_kimi_device(adapter, flows, scope, &flow)
            })
            .await;
        }
        #[cfg(target_os = "macos")]
        (AuthService::AgyOauth, AuthLoginMethod::Browser) => {
            native::start_agy(flows, execution, input, browser_local, reply).await?;
        }
        #[cfg(target_os = "macos")]
        (AuthService::DshAccount, AuthLoginMethod::Browser) => {
            native::start_dsh(flows, execution, input, browser_local, reply).await?;
        }
        #[cfg(target_os = "macos")]
        (AuthService::ClaudeOfficial, AuthLoginMethod::Browser | AuthLoginMethod::CopyLink) => {
            native::start_claude(runtime, execution, &store, input, reply).await?;
        }
        _ => return Err(AuthErrorCode::Unsupported),
    }
    Ok(())
}

fn started(reply: &mut StartSender, response: AuthStartResponse) -> bool {
    reply.take().is_some_and(|reply| reply.send(Ok(response)).is_ok())
}

async fn drive<F>(
    flows: &AuthFlows,
    scope: AuthAccountScope<'_>,
    response: AuthStartResponse,
    reply: &mut StartSender,
    mut poll: impl FnMut() -> F,
) where
    F: Future<Output = Result<AuthFlow, AuthErrorCode>>,
{
    let AuthStartResponse::Pending { flow_id, .. } = &response;
    let flow_id = flow_id.clone();
    if !started(reply, response) {
        let _ = crate::global_database::run_sync_database(|| {
            Ok(flows.cancel(scope, &flow_id, Instant::now))
        });
        return;
    }

    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        match poll().await {
            Ok(AuthFlow::Pending { .. }) => {}
            Ok(_) => break,
            Err(code) => {
                let _ = flows.complete(scope, &flow_id, Instant::now, Err(code), |_, _, _| Ok(()));
                break;
            }
        }
    }
    let _ = crate::global_database::run_sync_database(|| Ok(flows.reap_expired(Instant::now())));
}

async fn paste(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    Extension(runtime): Extension<Arc<Runtime>>,
    id: Result<Path<String>, axum::extract::rejection::PathRejection>,
    query: Result<Query<ScopeQuery>, QueryRejection>,
    request: Request,
) -> Result<StatusCode, FlowError> {
    let execution = authorize(policy, tenant, request.headers().clone(), query).await?;
    if execution.service != AuthService::ClaudeOfficial {
        return Err(AuthErrorCode::Unsupported.into());
    }
    let Path(id) = id.map_err(|_| AuthErrorCode::InvalidInput)?;
    let id = uuid::Uuid::parse_str(&id).map_err(|_| AuthErrorCode::NotFound)?;
    let id = AuthFlowId(id.to_string());
    let Json(input) = Json::<AuthPasteRequest>::from_request(request, &())
        .await
        .map_err(|_| AuthErrorCode::InvalidInput)?;
    runtime.flows.require_pending(execution.scope(), &id, Instant::now)?;
    #[cfg(target_os = "macos")]
    {
        let paste = runtime.claude.paste(&id)?;
        paste.submit(&input.pasted).await.map_err(native::paste_error)?;
        Ok(StatusCode::ACCEPTED)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = input;
        Err(AuthErrorCode::Unsupported.into())
    }
}

pub(super) struct FlowError {
    code: AuthErrorCode,
    start_service: Option<AuthService>,
}

impl From<AuthErrorCode> for FlowError {
    fn from(code: AuthErrorCode) -> Self {
        Self { code, start_service: None }
    }
}

impl FlowError {
    pub(super) fn access(status: StatusCode) -> Self {
        use AuthErrorCode::*;
        Self::from(match status {
            StatusCode::UNAUTHORIZED => Unauthenticated,
            StatusCode::FORBIDDEN => Forbidden,
            StatusCode::NOT_FOUND => NotFound,
            StatusCode::NOT_IMPLEMENTED => ExecutionChannelUnavailable,
            _ => ExecutionHostUnavailable,
        })
    }
}

impl IntoResponse for FlowError {
    fn into_response(self) -> Response {
        use AuthErrorCode::*;
        let (status, message) = match self.code {
            InvalidInput => (StatusCode::BAD_REQUEST, "Invalid authorization request"),
            Unauthenticated => (StatusCode::UNAUTHORIZED, "Authentication required"),
            Forbidden => (StatusCode::FORBIDDEN, "Authorization operation forbidden"),
            NotFound => (StatusCode::NOT_FOUND, "Authorization resource not found"),
            CallbackUnreachable => {
                (StatusCode::CONFLICT, "Browser cannot reach the execution host callback")
            }
            Unsupported | ExecutionChannelUnavailable => {
                (StatusCode::NOT_IMPLEMENTED, "Authorization unavailable on this host")
            }
            StorageUnavailable | ExecutionHostUnavailable | KeychainUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, "Authorization host resource unavailable")
            }
            RemoteRevocationUnconfirmed => (
                StatusCode::BAD_GATEWAY,
                "Local sign-out completed but remote revocation could not be confirmed",
            ),
            UpstreamFailure => (StatusCode::BAD_GATEWAY, "Authorization provider unavailable"),
            _ => (StatusCode::CONFLICT, "Authorization state changed or is unavailable"),
        };
        let correlation_id = uuid::Uuid::new_v4().to_string();
        if let Some(service) = self.start_service {
            tracing::warn!(
                operation = "start",
                ?service,
                code = ?self.code,
                %correlation_id,
                "Authorization request failed"
            );
        }
        (
            status,
            [(header::CACHE_CONTROL, "no-store")],
            Json(AuthError { code: self.code, message: message.into(), correlation_id }),
        )
            .into_response()
    }
}

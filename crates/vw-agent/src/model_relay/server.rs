use super::{RelayBinding, RelayIdentity};
use axum::{
    Router,
    body::{Body, Bytes, to_bytes},
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    net::TcpListener,
    sync::{OwnedSemaphorePermit, Semaphore},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

const BODY_LIMIT: usize = 4 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Only protocol and body cross into the provider transport. No client headers, URL or credentials.
pub struct RelayRequest {
    pub path: &'static str,
    pub body: Bytes,
}

/// Protocol transport boundary implemented by the low-level provider adapters.
/// Implementations must revalidate the bound provider/model and credentials on each call,
/// disable redirects, and never execute returned tool calls or enter session/prompt.
#[async_trait::async_trait]
pub trait RelayBackend: Send + Sync + 'static {
    async fn execute(
        &self,
        identity: &RelayIdentity,
        request: RelayRequest,
        cancel: CancellationToken,
    ) -> Result<Response, StatusCode>;
}

struct ServerState {
    binding: Arc<RelayBinding>,
    backend: Arc<dyn RelayBackend>,
    context: crate::project::instance::Context,
    capacity: Arc<Semaphore>,
    rate: parking_lot::Mutex<(Instant, u32)>,
}

/// A dedicated listener per capability makes tokens unusable against other sessions.
/// Dropping the owner revokes the capability; shutdown additionally joins all HTTP tasks.
pub struct RelayServer {
    address: SocketAddr,
    binding: Arc<RelayBinding>,
    task: Option<JoinHandle<std::io::Result<()>>>,
}
impl RelayServer {
    pub async fn start(
        binding: Arc<RelayBinding>,
        backend: Arc<dyn RelayBackend>,
        context: crate::project::instance::Context,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(
            context.project.id == binding.identity.project && !context.directory.is_empty(),
            "relay project mismatch"
        );
        anyhow::ensure!(
            binding.accepts(binding.bearer_token(), &binding.identity),
            "inactive relay capability"
        );
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let address = listener.local_addr()?;
        let state = Arc::new(ServerState {
            binding: binding.clone(),
            backend,
            context,
            capacity: Arc::new(Semaphore::new(4)),
            rate: parking_lot::Mutex::new((Instant::now(), 0)),
        });
        let router = Router::new()
            .route("/v1/chat/completions", post(handle))
            .route("/v1/messages", post(handle_messages))
            .route("/v1/responses", post(handle))
            .with_state(state);
        let cancel = binding.cancelled.clone();
        let expires = binding.expires;
        let task = tokio::spawn(async move {
            axum::serve(listener, router).with_graceful_shutdown(async move {
                tokio::select! { () = cancel.cancelled() => {}, () = tokio::time::sleep_until(expires.into()) => { cancel.cancel(); } }
            }).await
        });
        Ok(Self { address, binding, task: Some(task) })
    }
    pub fn address(&self) -> SocketAddr {
        self.address
    }
    pub async fn shutdown(mut self) -> anyhow::Result<()> {
        self.binding.revoke();
        if let Some(task) = self.task.take() {
            task.await??;
        }
        Ok(())
    }
}
impl Drop for RelayServer {
    fn drop(&mut self) {
        self.binding.revoke();
    }
}

struct RequestGuard {
    cancel: CancellationToken,
    _permit: OwnedSemaphorePermit,
}
impl Drop for RequestGuard {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

async fn handle_messages(state: State<Arc<ServerState>>, request: Request) -> Response {
    let response = handle(state, request).await;
    if response.status().is_success() {
        response
    } else {
        super::messages::error(response.status())
    }
}

async fn handle(State(state): State<Arc<ServerState>>, request: Request) -> Response {
    let Some(token) = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if !state.binding.accepts(token, &state.binding.identity) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    // Query strings and unexpected identity/auth headers are never forwarded.
    if request.uri().query().is_some() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let path = match request.uri().path() {
        "/v1/chat/completions" => "/v1/chat/completions",
        "/v1/messages" => "/v1/messages",
        "/v1/responses" => "/v1/responses",
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let Ok(permit) = state.capacity.clone().try_acquire_owned() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    {
        let mut rate = state.rate.lock();
        if rate.0.elapsed() >= Duration::from_secs(60) {
            *rate = (Instant::now(), 0);
        }
        if rate.1 >= 60 {
            return StatusCode::TOO_MANY_REQUESTS.into_response();
        }
        rate.1 += 1;
    }
    let cancel = state.binding.cancelled.child_token();
    let guard = RequestGuard { cancel: cancel.clone(), _permit: permit };
    let deadline = std::cmp::min(state.binding.expires, Instant::now() + REQUEST_TIMEOUT);
    let response = async {
        let body = to_bytes(request.into_body(), BODY_LIMIT)
            .await
            .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
        let json: serde_json::Value =
            serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
        // Address/auth/session override keys are not part of any accepted protocol.
        let Some(object) = json.as_object() else {
            return Err(StatusCode::BAD_REQUEST);
        };
        if [
            "base_url",
            "api_base",
            "api_key",
            "authorization",
            "headers",
            "url",
            "provider",
            "tenant",
            "session",
            "generation",
        ]
        .iter()
        .any(|key| object.contains_key(*key))
        {
            return Err(StatusCode::BAD_REQUEST);
        }
        if object.get("model").and_then(serde_json::Value::as_str)
            != Some(state.binding.identity.model.as_str())
        {
            return Err(StatusCode::FORBIDDEN);
        }
        crate::session::ui_store::with_tenant_scope(
            state.binding.identity.tenant.clone(),
            crate::project::instance::provide_context(state.context.clone(), async {
                crate::provider::provider::get_model(
                    &state.binding.identity.provider,
                    &state.binding.identity.model,
                )
                .await
                .map_err(|_| StatusCode::FORBIDDEN)?;
                state
                    .backend
                    .execute(&state.binding.identity, RelayRequest { path, body }, cancel.clone())
                    .await
            }),
        )
        .await
    };
    let response = tokio::select! {
        biased;
        () = cancel.cancelled() => return StatusCode::UNAUTHORIZED.into_response(),
        () = tokio::time::sleep_until(deadline.into()) => return StatusCode::GATEWAY_TIMEOUT.into_response(),
        response = response => response,
    };
    let response = match response {
        Ok(response) => response,
        Err(status) => return status.into_response(),
    };
    // Never expose redirects (including Location credentials) or arbitrary backend errors.
    if !response.status().is_success() {
        return StatusCode::BAD_GATEWAY.into_response();
    }
    let (mut parts, body) = response.into_parts();
    let content_type = parts.headers.get(axum::http::header::CONTENT_TYPE).cloned();
    parts.headers.clear();
    if let Some(value) = content_type {
        parts.headers.insert(axum::http::header::CONTENT_TYPE, value);
    }
    let stream = futures_util::stream::unfold(
        (body.into_data_stream(), guard, deadline),
        |(mut stream, guard, deadline)| async move {
            use futures_util::StreamExt;
            tokio::select! {
                biased;
                () = guard.cancel.cancelled() => None,
                () = tokio::time::sleep_until(deadline.into()) => None,
                next = stream.next() => next.map(|chunk| (chunk.map_err(|_| std::io::Error::other("relay stream failed")), (stream, guard, deadline))),
            }
        },
    );
    Response::from_parts(parts, Body::from_stream(stream))
}

/// Common upstream client for relay transports: credentials must never follow redirects.
pub fn upstream_http_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(REQUEST_TIMEOUT)
        .build()
}

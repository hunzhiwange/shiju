use super::{RelayBinding, RelayIdentity};
use axum::{
    Router,
    body::{Body, Bytes, to_bytes},
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
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
    control_capacity: Arc<Semaphore>,
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
            control_capacity: Arc::new(Semaphore::new(8)),
            rate: parking_lot::Mutex::new((Instant::now(), 0)),
        });
        let router = Router::new()
            .route("/", post(handle))
            .route("/v1/chat/completions", post(handle))
            .route("/v1/messages", post(handle_messages))
            .route("/v1/responses", post(handle_responses))
            .route("/v1beta/models", get(handle_gemini))
            .route("/v1beta/models/{*resource}", get(handle_gemini).post(handle_gemini))
            .with_state(state);
        #[cfg(test)]
        let router = router.layer(axum::middleware::from_fn(
            |request: axum::extract::Request, next: axum::middleware::Next| async move {
                let route = match request.uri().path() {
                    "/v1/messages" => "messages",
                    "/v1/messages/count_tokens" => "count_tokens",
                    "/v1/models" => "models",
                    _ => "other",
                };
                let response = next.run(request).await;
                record_acceptance_status(route, response.status().as_u16());
                response
            },
        ));
        let cancel = binding.cancelled.clone();
        let expires = binding.expires;
        let task = tokio::spawn(async move {
            axum::serve(listener, router).with_graceful_shutdown(async move {
                if let Some(expires) = expires {
                    tokio::select! {
                        () = cancel.cancelled() => {},
                        () = tokio::time::sleep_until(expires.into()) => cancel.cancel(),
                    }
                } else {
                    cancel.cancelled().await;
                }
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

async fn handle_gemini(state: State<Arc<ServerState>>, request: Request) -> Response {
    let response = handle(state, request).await;
    if response.status().is_success() { response } else { super::gemini::error(response.status()) }
}

async fn handle_responses(state: State<Arc<ServerState>>, request: Request) -> Response {
    let response = handle(state, request).await;
    if response.status().is_success() {
        response
    } else {
        super::responses::error(response.status())
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
    let gemini = request.uri().path().starts_with("/v1beta/models");
    let bearer = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    if request.headers().contains_key("authorization") && bearer.is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let api_key = if gemini {
        request.headers().get("x-goog-api-key").and_then(|v| v.to_str().ok())
    } else {
        None
    };
    let token = match (bearer, api_key) {
        (Some(t), None) | (None, Some(t)) => t,
        _ => return StatusCode::UNAUTHORIZED.into_response(),
    };
    if !state.binding.accepts(token, &state.binding.identity) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let kiro = state.binding.identity.session.ends_with(":kiro");
    let kiro_control = kiro && request.uri().path() == "/";
    let kiro_target = request
        .headers()
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let metadata = gemini && request.method() == axum::http::Method::GET;
    let path = if gemini {
        let resource = request.uri().path().strip_prefix("/v1beta/models/");
        let (name, action) = match resource {
            Some(r) => r.split_once(':').map_or((Some(r), None), |(n, a)| (Some(n), Some(a))),
            None => (None, None),
        };
        if name.is_some_and(|n| n != state.binding.identity.model) {
            return StatusCode::FORBIDDEN.into_response();
        }
        match (metadata, action, name) {
            (true, None, _) => {
                if name.is_some() {
                    "/v1beta/model"
                } else {
                    "/v1beta/models"
                }
            }
            (false, Some("generateContent"), Some(_)) => "/v1beta/generateContent",
            (false, Some("streamGenerateContent"), Some(_)) => "/v1beta/streamGenerateContent",
            _ => return StatusCode::NOT_FOUND.into_response(),
        }
    } else {
        match request.uri().path() {
            "/" if kiro_control => "/kiro/control",
            "/v1/chat/completions" => "/v1/chat/completions",
            "/v1/messages" => "/v1/messages",
            "/v1/responses" => "/v1/responses",
            _ => return StatusCode::NOT_FOUND.into_response(),
        }
    };
    if request.uri().query().is_some_and(|q| {
        !matches!(
            (path, q),
            ("/v1beta/streamGenerateContent", "alt=sse") | ("/v1/messages", "beta=true")
        )
    }) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    // Kiro initializes its local catalog/settings concurrently. Keep this bounded
    // local-only lane separate from the four upstream model request permits.
    let capacity = if kiro_control { &state.control_capacity } else { &state.capacity };
    let Ok(permit) = capacity.clone().try_acquire_owned() else {
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
    let deadline = Instant::now() + REQUEST_TIMEOUT;
    let deadline = state.binding.expires.map_or(deadline, |expires| expires.min(deadline));
    let response = async {
        let body = to_bytes(request.into_body(), BODY_LIMIT)
            .await
            .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
        let mut json: serde_json::Value = if metadata {
            serde_json::json!({})
        } else {
            serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?
        };
        if kiro_control {
            return crate::session::ui_store::with_tenant_scope(
                state.binding.identity.tenant.clone(),
                crate::project::instance::provide_context(state.context.clone(), async {
                    let model = crate::provider::provider::get_model(
                        &state.binding.identity.provider,
                        &state.binding.identity.model,
                    )
                    .await
                    .map_err(|_| StatusCode::FORBIDDEN)?;
                    if model.status != "active" {
                        return Err(StatusCode::FORBIDDEN);
                    }
                    super::kiro::control(&kiro_target, &json, &state.binding.identity.model)
                        .map(|value| axum::Json(value).into_response())
                }),
            )
            .await;
        }
        if kiro && path == "/v1/responses" {
            super::kiro::responses(&mut json)?;
        }
        if gemini {
            let object = json.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
            if object
                .get("model")
                .is_some_and(|v| v.as_str() != Some(state.binding.identity.model.as_str()))
            {
                return Err(StatusCode::FORBIDDEN);
            }
            object.insert("model".into(), state.binding.identity.model.clone().into());
            if object.contains_key("stream") {
                return Err(StatusCode::BAD_REQUEST);
            }
            if !metadata {
                object.insert("stream".into(), (path == "/v1beta/streamGenerateContent").into());
            }
        }
        let body = if gemini || (kiro && path == "/v1/responses") {
            Bytes::from(serde_json::to_vec(&json).map_err(|_| StatusCode::BAD_REQUEST)?)
        } else {
            body
        };
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

#[cfg(test)]
fn record_acceptance_status(route: &'static str, status: u16) {
    let Some(path) = std::env::var_os("VW_ACP_ACCEPTANCE_DIAGNOSTICS") else { return };
    static STATUS: std::sync::LazyLock<parking_lot::Mutex<Vec<serde_json::Value>>> =
        std::sync::LazyLock::new(|| parking_lot::Mutex::new(Vec::new()));
    let mut observations = STATUS.lock();
    if observations.len() < 128 {
        observations.push(serde_json::json!({"route":route,"status":status}));
    }
    if let Ok(bytes) = serde_json::to_vec(&*observations) {
        let path = std::path::PathBuf::from(path).with_extension("relay.json");
        let _ = std::fs::write(path, bytes);
    }
}

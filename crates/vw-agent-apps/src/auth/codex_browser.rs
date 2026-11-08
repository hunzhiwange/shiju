//! Host-owned Codex authorization-code flow with an IPv4 loopback callback.

use super::{
    codex_device::{CLIENT_ID, complete_account, exchange_code, oauth_client},
    copilot::{utc_now, valid_token},
    credential_file::AccountCredential,
    flow::{AuthAccountScope, AuthFlowTiming, AuthFlows},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, Request, StatusCode, Url};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::{ErrorKind, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use uuid::Uuid;
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{
        AuthAccount, AuthErrorCode, AuthFlow, AuthFlowId, AuthLoginMethod, AuthService,
        AuthStartRequest, AuthStartResponse,
    },
};

const CALLBACK_PORT: u16 = 1455;
const CALLBACK_PATH: &str = "/auth/callback";
const LOGIN_LIFETIME: Duration = Duration::from_secs(900);
const MAX_REQUEST_BYTES: usize = 8192;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
const IO_TIMEOUT: Duration = Duration::from_millis(100);

type CallbackResult = Result<String, AuthErrorCode>;

struct BrowserGrant {
    verifier: String,
    result: Option<CallbackResult>,
}

struct CallbackWorker {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl CallbackWorker {
    fn stop(&mut self) -> Result<(), AuthErrorCode> {
        if let Some(join) = self.join.take() {
            self.stop.store(true, Ordering::Relaxed);
            join.thread().unpark();
            join.join().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        }
        Ok(())
    }
}

impl Drop for CallbackWorker {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Only the start response leaves the host. The bound listener and verifier
/// belong to this attempt and are cleared on cancellation or expiry.
pub struct CodexBrowserStart {
    pub response: AuthStartResponse,
    flow_id: AuthFlowId,
    redirect_uri: String,
    client: Client,
    grant: Arc<Mutex<Option<BrowserGrant>>>,
    worker: Arc<Mutex<CallbackWorker>>,
    polling: tokio::sync::Mutex<()>,
}

impl CodexBrowserStart {
    /// `browser_on_execution_host` comes from trusted host routing. A browser
    /// on a different host cannot use this fixed localhost redirect.
    pub fn start(
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        browser_on_execution_host: bool,
    ) -> Result<Self, AuthErrorCode> {
        Self::start_on(
            flows,
            scope,
            request,
            browser_on_execution_host,
            CALLBACK_PORT,
            LOGIN_LIFETIME,
        )
    }

    pub(super) fn start_on(
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        browser_on_execution_host: bool,
        port: u16,
        lifetime: Duration,
    ) -> Result<Self, AuthErrorCode> {
        scope.validate()?;
        if scope.service != AuthService::CodexOauth
            || request.method != AuthLoginMethod::Browser
            || request.target_account_id.is_some()
        {
            return Err(AuthErrorCode::Unsupported);
        }
        request.validate_for(scope.service)?;
        if !browser_on_execution_host {
            return Err(AuthErrorCode::CallbackUnreachable);
        }

        let client = oauth_client()?;
        let deadline = Instant::now() + lifetime;
        let expires_at = TimestampMs(utc_now()?.0 + lifetime.as_millis() as i64);
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).map_err(|error| {
            if error.kind() == ErrorKind::AddrInUse {
                AuthErrorCode::Conflict
            } else {
                AuthErrorCode::ExecutionHostUnavailable
            }
        })?;
        listener.set_nonblocking(true).map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        let port =
            listener.local_addr().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?.port();
        let redirect_uri = format!("http://localhost:{port}{CALLBACK_PATH}");
        let state = random_secret();
        let verifier = random_secret();
        let verification_url = authorize_url(&redirect_uri, &state, &verifier);
        let grant = Arc::new(Mutex::new(Some(BrowserGrant { verifier, result: None })));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker_grant = Arc::clone(&grant);
        // A dedicated worker can be stopped and joined by the synchronous host
        // cleanup hook, including when cancellation runs on the async runtime.
        let join = thread::Builder::new()
            .name("codex-oauth-callback".into())
            .spawn(move || {
                serve_callbacks(listener, &state, &worker_grant, &worker_stop, deadline);
                if let Ok(mut grant) = worker_grant.lock() {
                    *grant = None;
                }
            })
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        let worker = Arc::new(Mutex::new(CallbackWorker { stop, join: Some(join) }));
        let cleanup = Arc::clone(&worker);
        let flow = flows.start_with_cleanup(
            scope,
            request,
            AuthFlowTiming { deadline, expires_at, retry_after_seconds: 1 },
            move || stop_worker(&cleanup),
        )?;
        let AuthFlow::Pending { flow_id, expires_at, retry_after_seconds } = flow else {
            unreachable!("registering a flow always returns its pending snapshot");
        };
        let response = AuthStartResponse::Pending {
            flow_id: flow_id.clone(),
            expires_at,
            retry_after_seconds,
            verification_url,
            user_code: None,
            can_paste: None,
        };
        Ok(Self {
            response,
            flow_id,
            redirect_uri,
            client,
            grant,
            worker,
            polling: tokio::sync::Mutex::new(()),
        })
    }

    pub async fn poll(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            &AuthAccount,
            &str,
            &AccountCredential,
        ) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        self.poll_with(flows, scope, |_| {}, persist).await
    }

    pub(super) async fn poll_with(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        mut route: impl FnMut(&mut Request),
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            &AuthAccount,
            &str,
            &AccountCredential,
        ) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        let snapshot = flows.snapshot(scope, &self.flow_id, Instant::now)?;
        if !matches!(snapshot, AuthFlow::Pending { .. }) {
            return Ok(snapshot);
        }
        let Ok(_poll) = self.polling.try_lock() else {
            return Ok(snapshot);
        };
        let callback = {
            let mut grant =
                self.grant.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
            match grant.as_ref() {
                Some(grant) if grant.result.is_none() => return Ok(snapshot),
                _ => grant.take(),
            }
        };
        stop_worker(&self.worker)?;

        let result = async {
            let BrowserGrant { verifier, result } =
                callback.ok_or(AuthErrorCode::UpstreamFailure)?;
            let code = result.ok_or(AuthErrorCode::UpstreamFailure)??;
            let tokens =
                exchange_code(&self.client, &code, &verifier, &self.redirect_uri, &mut route)
                    .await?;
            complete_account(scope, tokens)
        }
        .await;
        let completed = match result {
            Ok((account, (identity, credential))) => flows.complete(
                scope,
                &self.flow_id,
                Instant::now,
                Ok(account),
                |bound, _, account| persist(bound, account, &identity, &credential),
            ),
            Err(error) => {
                flows.complete(scope, &self.flow_id, Instant::now, Err(error), |_, _, _| Ok(()))
            }
        };
        match completed {
            Err(AuthErrorCode::Conflict) => {
                let snapshot = flows.snapshot(scope, &self.flow_id, Instant::now)?;
                if matches!(snapshot, AuthFlow::Pending { .. }) {
                    return Err(AuthErrorCode::Conflict);
                }
                Ok(snapshot)
            }
            result => result,
        }
    }
}

fn stop_worker(worker: &Mutex<CallbackWorker>) -> Result<(), AuthErrorCode> {
    worker.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?.stop()
}

fn random_secret() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn authorize_url(redirect_uri: &str, state: &str, verifier: &str) -> String {
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut url =
        Url::parse("https://auth.openai.com/oauth/authorize").expect("constant OAuth URL");
    url.query_pairs_mut().extend_pairs([
        ("response_type", "code"),
        ("client_id", CLIENT_ID),
        ("redirect_uri", redirect_uri),
        ("scope", "openid profile email offline_access api.connectors.read api.connectors.invoke"),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("state", state),
        ("id_token_add_organizations", "true"),
        ("codex_cli_simplified_flow", "true"),
        ("originator", "codex_cli_rs"),
    ]);
    url.into()
}

fn serve_callbacks(
    listener: TcpListener,
    state: &str,
    grant: &Mutex<Option<BrowserGrant>>,
    stop: &AtomicBool,
    deadline: Instant,
) {
    while !stop.load(Ordering::Relaxed) && Instant::now() < deadline {
        match listener.accept() {
            Ok((stream, peer)) if peer.ip().is_loopback() => {
                respond(stream, state, grant, stop, deadline);
            }
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::park_timeout(Duration::from_millis(20));
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
}

fn respond(
    mut stream: TcpStream,
    state: &str,
    grant: &Mutex<Option<BrowserGrant>>,
    stop: &AtomicBool,
    deadline: Instant,
) {
    if stream.set_nonblocking(false).is_err()
        || stream.set_read_timeout(Some(IO_TIMEOUT)).is_err()
        || stream.set_write_timeout(Some(IO_TIMEOUT)).is_err()
    {
        return;
    }
    let request_deadline = deadline.min(Instant::now() + REQUEST_TIMEOUT);
    let mut buffer = [0; MAX_REQUEST_BYTES];
    let mut filled = 0;
    let callback = loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        if Instant::now() >= request_deadline {
            break Err(StatusCode::REQUEST_TIMEOUT);
        }
        if filled == buffer.len() {
            break Err(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE);
        }
        match stream.read(&mut buffer[filled..]) {
            Ok(0) => return,
            Ok(read) => {
                filled += read;
                if buffer[..filled].windows(4).any(|window| window == b"\r\n\r\n") {
                    if Instant::now() >= request_deadline {
                        break Err(StatusCode::REQUEST_TIMEOUT);
                    }
                    break parse_callback(&buffer[..filled], state);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                ) => {}
            Err(_) => return,
        }
    };

    let status = match callback {
        Err(status) => status,
        Ok(result) => {
            let Ok(mut grant) = grant.lock() else {
                return;
            };
            if stop.load(Ordering::Relaxed) || Instant::now() >= deadline {
                StatusCode::GONE
            } else if let Some(grant) = grant.as_mut().filter(|grant| grant.result.is_none()) {
                grant.result = Some(result);
                StatusCode::OK
            } else {
                StatusCode::CONFLICT
            }
        }
    };
    let body = match status {
        StatusCode::OK => "Callback received. Return to ShiJu to check authorization.",
        StatusCode::CONFLICT => "This login callback has already been received.",
        _ => "Invalid login callback. Return to ShiJu and try again.",
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n{body}",
        body.len(),
    );
}

fn parse_callback(bytes: &[u8], expected_state: &str) -> Result<CallbackResult, StatusCode> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut request = httparse::Request::new(&mut headers);
    let bad_request = StatusCode::BAD_REQUEST;
    let size = match request.parse(bytes) {
        Ok(httparse::Status::Complete(size)) => size,
        Err(httparse::Error::TooManyHeaders) => {
            return Err(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE);
        }
        _ => return Err(bad_request),
    };
    if size != bytes.len()
        || request.headers.iter().any(|header| {
            header.name.eq_ignore_ascii_case("transfer-encoding")
                || header.name.eq_ignore_ascii_case("content-length") && header.value != b"0"
        })
    {
        return Err(bad_request);
    }
    if request.method != Some("GET") {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }
    let target = request.path.ok_or(bad_request)?;
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path != CALLBACK_PATH {
        return Err(StatusCode::NOT_FOUND);
    }
    if query.contains('#')
        || query.bytes().enumerate().any(|(offset, byte)| {
            byte == b'%'
                && !query
                    .as_bytes()
                    .get(offset + 1..offset + 3)
                    .is_some_and(|encoded| encoded.iter().all(u8::is_ascii_hexdigit))
        })
    {
        return Err(bad_request);
    }
    let url = Url::parse(&format!("http://localhost/?{query}")).map_err(|_| bad_request)?;
    let mut values = HashMap::new();
    for (key, value) in url.query_pairs() {
        if values.insert(key, value).is_some() {
            return Err(bad_request);
        }
    }
    if values.get("state").map(|value| value.as_ref()) != Some(expected_state) {
        return Err(bad_request);
    }
    match (values.remove("code"), values.remove("error")) {
        (Some(code), None) if valid_token(&code) => Ok(Ok(code.into_owned())),
        (None, Some(error)) if !error.is_empty() => Ok(Err(AuthErrorCode::AuthorizationDenied)),
        _ => Err(bad_request),
    }
}

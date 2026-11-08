//! Bounded, single-use OAuth callback handling on the execution host.

use super::copilot::valid_token;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{StatusCode, Url};
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
use vw_api_types::agent_apps_auth::AuthErrorCode;

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

pub(super) struct CallbackCode {
    pub code: String,
    pub verifier: String,
}

type TakenCallback = Result<CallbackCode, AuthErrorCode>;

pub(super) struct LoopbackCallback {
    grant: Arc<Mutex<Option<BrowserGrant>>>,
    worker: Arc<Mutex<CallbackWorker>>,
}

pub(super) struct CallbackChallenge {
    pub port: u16,
    pub state: String,
    pub challenge: String,
}

impl LoopbackCallback {
    pub(super) fn bind(
        port: u16,
        path: &'static str,
        worker_name: &'static str,
        deadline: Instant,
    ) -> Result<(Self, CallbackChallenge), AuthErrorCode> {
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
        let state = random_secret();
        let verifier = random_secret();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let parameters = CallbackChallenge { port, state: state.clone(), challenge };
        let grant = Arc::new(Mutex::new(Some(BrowserGrant { verifier, result: None })));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker_grant = Arc::clone(&grant);
        // A dedicated worker can be stopped and joined by the synchronous host
        // cleanup hook, including when cancellation runs on the async runtime.
        let join = thread::Builder::new()
            .name(worker_name.into())
            .spawn(move || {
                serve_callbacks(listener, path, &state, &worker_grant, &worker_stop, deadline);
                if let Ok(mut grant) = worker_grant.lock() {
                    *grant = None;
                }
            })
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        let worker = Arc::new(Mutex::new(CallbackWorker { stop, join: Some(join) }));
        Ok((Self { grant, worker }, parameters))
    }

    pub(super) fn cleanup(&self) -> impl FnMut() -> Result<(), AuthErrorCode> + Send + 'static {
        let worker = Arc::clone(&self.worker);
        move || stop_worker(&worker)
    }

    /// Take the accepted grant once and close the listener before token exchange.
    pub(super) fn take(&self) -> Result<Option<TakenCallback>, AuthErrorCode> {
        let callback = {
            let mut grant =
                self.grant.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
            match grant.as_ref() {
                Some(grant) if grant.result.is_none() => return Ok(None),
                _ => grant.take(),
            }
        };
        stop_worker(&self.worker)?;
        let result = match callback {
            Some(BrowserGrant { verifier, result: Some(result) }) => {
                result.map(|code| CallbackCode { code, verifier })
            }
            _ => Err(AuthErrorCode::UpstreamFailure),
        };
        Ok(Some(result))
    }
}

fn stop_worker(worker: &Mutex<CallbackWorker>) -> Result<(), AuthErrorCode> {
    worker.lock().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?.stop()
}

fn random_secret() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn serve_callbacks(
    listener: TcpListener,
    path: &str,
    state: &str,
    grant: &Mutex<Option<BrowserGrant>>,
    stop: &AtomicBool,
    deadline: Instant,
) {
    while !stop.load(Ordering::Relaxed) && Instant::now() < deadline {
        match listener.accept() {
            Ok((stream, peer)) if peer.ip().is_loopback() => {
                respond(stream, path, state, grant, stop, deadline);
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
    path: &str,
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
                    break parse_callback(&buffer[..filled], path, state);
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

fn parse_callback(
    bytes: &[u8],
    callback_path: &str,
    expected_state: &str,
) -> Result<CallbackResult, StatusCode> {
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
    if path != callback_path {
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

//! Local WSS boundary for Office add-ins. No gateway credentials cross this boundary.
//!
//! `approve_pairing` is a trusted desktop-only entry point: call it only after explicit
//! approval for the owner/surface/document. Pairing is not permission to execute;
//! the dispatcher must recheck ControlHost policy before and after `execute`.
//! Certificates must already be trusted by the Office WebView, with localhost SAN.
//! Store cert.pem/key.pem outside the plugin/repository in a user-owned 0700
//! directory (files 0600). Never disable certificate validation or install trust
//! silently. See https://learn.microsoft.com/en-us/office/dev/add-ins/develop/add-in-manifests.
//! Pair tokens are memory-only, single use, 60 seconds; connections last 5 minutes.
//! Reconnect/reload/Save As require a new desktop approval and document generation.

use super::sessions::{
    OfficeCapability, OfficeHandle, OfficeSession, OfficeSessions, OfficeSurface,
};
use anyhow::{Context, Result, bail, ensure};
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tokio_tungstenite::tungstenite::{Message, protocol::WebSocketConfig};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use vw_api_types::computer_control::{
    ComputerControlAction, ComputerControlTarget, ControlDocumentId, ControlSessionId,
};

const MAX_BYTES: usize = 64 * 1024;
const TIMEOUT: Duration = Duration::from_secs(15);
const LIFETIME: Duration = Duration::from_secs(300);

struct Pairing {
    owner: ControlSessionId,
    surface: OfficeSurface,
    document: ControlDocumentId,
    expires: Instant,
}

struct Pending {
    id: Uuid,
    reply: oneshot::Sender<Value>,
}

struct Connection {
    owner: ControlSessionId,
    expires: Instant,
    send: mpsc::Sender<Value>,
    cancel: CancellationToken,
    pending: Option<Pending>,
}

struct State {
    pairings: HashMap<String, Pairing>,
    sessions: OfficeSessions,
    connections: HashMap<Uuid, Connection>,
}

pub struct OfficeTransport {
    address: SocketAddr,
    state: Arc<Mutex<State>>,
    cancel: CancellationToken,
    task: Option<JoinHandle<()>>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Incoming {
    Pair {
        token: String,
        surface: String,
        document_id: String,
        title: String,
        capabilities: Vec<String>,
    },
    Heartbeat {
        sequence: u64,
    },
    Result {
        sequence: u64,
        request_id: String,
        value: Value,
    },
}

impl OfficeTransport {
    /// Starts a TLS-only listener on an OS-selected IPv4 loopback port. Only the
    /// exact HTTPS loopback task-pane origin is allowed, never global CORS.
    pub async fn bind(credentials: &Path, origin: &str) -> Result<Self> {
        let url = url::Url::parse(origin).context("invalid Office origin")?;
        ensure!(
            url.scheme() == "https"
                && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
                && url.origin().ascii_serialization() == origin,
            "Office requires an exact HTTPS loopback origin"
        );
        let tls = load_tls(credentials)?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let state = Arc::new(Mutex::new(State {
            pairings: HashMap::new(),
            sessions: OfficeSessions::new(Duration::from_secs(30))
                .map_err(|_| anyhow::anyhow!("invalid Office timeout"))?,
            connections: HashMap::new(),
        }));
        let cancel = CancellationToken::new();
        let task =
            tokio::spawn(serve(listener, tls, origin.to_owned(), state.clone(), cancel.clone()));
        Ok(Self { address, state, cancel, task: Some(task) })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// No remote endpoint can call this. Return this secret only to the approved
    /// task pane; do not log it, persist it or put it in a URL/query string.
    pub fn approve_pairing(
        &self,
        owner: ControlSessionId,
        surface: OfficeSurface,
        document: ControlDocumentId,
    ) -> Result<String> {
        ensure!(!self.cancel.is_cancelled(), "Office transport stopped");
        ensure!(
            !owner.0.trim().is_empty()
                && owner.0.len() <= 256
                && !document.0.trim().is_empty()
                && document.0.len() <= 256,
            "invalid Office identity"
        );
        let mut state = self.state.lock();
        state.pairings.retain(|_, grant| grant.expires > Instant::now());
        ensure!(state.pairings.len() < 32, "too many pending Office approvals");
        let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        state.pairings.insert(
            token.clone(),
            Pairing { owner, surface, document, expires: Instant::now() + Duration::from_secs(60) },
        );
        Ok(token)
    }

    pub fn documents(&self, owner: &ControlSessionId) -> Vec<OfficeSession> {
        self.state.lock().sessions.discover(owner, Instant::now())
    }

    pub fn revoke(&self, owner: &ControlSessionId) {
        let mut state = self.state.lock();
        state.pairings.retain(|_, grant| &grant.owner != owner);
        state.connections.retain(|_, connection| {
            if &connection.owner == owner {
                connection.cancel.cancel();
                false
            } else {
                true
            }
        });
        state.sessions.disconnect_owner(owner);
    }

    pub(crate) fn connected_documents(&self) -> Vec<OfficeSession> {
        let mut state = self.state.lock();
        let owners: Vec<_> = state.connections.values().map(|c| c.owner.clone()).collect();
        let mut documents = Vec::new();
        for owner in owners { documents.extend(state.sessions.discover(&owner, Instant::now())); }
        documents
    }

    /// Trusted emergency stop cancels all document connections and pending pairings.
    pub(crate) fn revoke_all(&self) {
        let owners: Vec<_> = self.state.lock().connections.values().map(|c| c.owner.clone()).collect();
        for owner in owners { self.revoke(&owner); }
        self.state.lock().pairings.clear();
    }

    /// Exact capability and identity checks surround each result. One command per
    /// document at a time; timeout invalidates the connection and every old result.
    pub async fn execute(
        &self,
        owner: &ControlSessionId,
        handle: &OfficeHandle,
        target: &ComputerControlTarget,
        action: &ComputerControlAction,
    ) -> Result<Value> {
        // A dropped request must invalidate its queue and old document handle.
        let mut execution = ExecutionLease {
            state: self.state.clone(), owner: owner.clone(), handle: handle.clone(), armed: false,
        };
        let id = Uuid::new_v4();
        let message = json!({"type":"command", "request_id":id.to_string(), "surface":surface_name(handle.surface), "document_id":handle.document_id.0, "generation":handle.generation.to_string(), "action":action});
        ensure!(serde_json::to_vec(&message)?.len() <= MAX_BYTES, "Office command too large");
        let (reply, result) = oneshot::channel();
        {
            let mut state = self.state.lock();
            state
                .sessions
                .authorize(owner, handle, target, action, Instant::now())
                .map_err(|_| anyhow::anyhow!("Office command not authorized"))?;
            let connection =
                state.connections.get_mut(&handle.generation).context("Office disconnected")?;
            ensure!(
                connection.expires > Instant::now() && !connection.cancel.is_cancelled(),
                "Office connection expired"
            );
            ensure!(connection.pending.is_none(), "Office document busy");
            connection.send.try_send(message).context("Office send queue unavailable")?;
            connection.pending = Some(Pending { id, reply });
            execution.armed = true;
        }
        let value = match tokio::time::timeout(TIMEOUT, result).await {
            Ok(Ok(value)) => value,
            _ => {
                let mut state = self.state.lock();
                if let Some(connection) = state.connections.remove(&handle.generation) {
                    connection.cancel.cancel();
                }
                let _ = state.sessions.disconnect(owner, handle);
                bail!("Office command disconnected or timed out");
            }
        };
        self.state
            .lock()
            .sessions
            .authorize(owner, handle, target, action, Instant::now())
            .map_err(|_| anyhow::anyhow!("Office result no longer authorized"))?;
        execution.armed = false;
        Ok(value)
    }

    pub async fn shutdown(mut self) {
        self.cancel.cancel();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for OfficeTransport {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

fn surface_name(surface: OfficeSurface) -> &'static str {
    match surface {
        OfficeSurface::Excel => "excel",
        OfficeSurface::Word => "word",
    }
}

fn capabilities(names: Vec<String>) -> Result<Vec<OfficeCapability>> {
    names
        .into_iter()
        .map(|name| match name.as_str() {
            "excel_read_range" => Ok(OfficeCapability::ExcelReadRange),
            "excel_write_range" => Ok(OfficeCapability::ExcelWriteRange),
            "word_read_document" => Ok(OfficeCapability::WordReadDocument),
            "word_find_text" => Ok(OfficeCapability::WordFindText),
            "word_replace_text" => Ok(OfficeCapability::WordReplaceText),
            "word_insert_text" => Ok(OfficeCapability::WordInsertText),
            _ => anyhow::bail!("unknown Office capability"),
        })
        .collect()
}

#[cfg(unix)]
fn load_tls(directory: &Path) -> Result<tokio_rustls::TlsAcceptor> {
    use std::io::Read;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .context("missing repository root")?;
    ensure!(
        !directory.canonicalize()?.starts_with(repository.canonicalize()?),
        "Office TLS credentials must be outside the repository and plugin package"
    );
    let dir =
        std::fs::symlink_metadata(directory).context("missing Office credentials directory")?;
    ensure!(
        directory.is_absolute()
            && dir.is_dir()
            && dir.uid() == unsafe { libc::geteuid() }
            && dir.mode() & 0o077 == 0,
        "Office credentials require a user-owned 0700 directory outside the package"
    );
    let read = |name: &str| -> Result<Vec<u8>> {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(directory.join(name))
            .context("missing Office TLS file")?;
        let meta = file.metadata()?;
        ensure!(
            meta.is_file()
                && meta.uid() == dir.uid()
                && meta.mode() & 0o077 == 0
                && meta.len() <= MAX_BYTES as u64,
            "Office TLS file requires user ownership, mode 0600 and bounded size"
        );
        let mut bytes = Vec::new();
        file.by_ref().take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
        ensure!(bytes.len() <= MAX_BYTES, "Office TLS file too large");
        Ok(bytes)
    };
    let certificates = CertificateDer::pem_slice_iter(&read("cert.pem")?)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let key = PrivateKeyDer::from_pem_slice(&read("key.pem")?)?;
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_no_client_auth()
    .with_single_cert(certificates, key)?;
    Ok(Arc::new(config).into())
}

#[cfg(not(unix))]
fn load_tls(_: &Path) -> Result<tokio_rustls::TlsAcceptor> {
    bail!("Office local transport currently requires Unix credential permissions")
}

async fn serve(
    listener: TcpListener,
    tls: tokio_rustls::TlsAcceptor,
    origin: String,
    state: Arc<Mutex<State>>,
    cancel: CancellationToken,
) {
    let mut jobs = tokio::task::JoinSet::new();
    let slots = Arc::new(tokio::sync::Semaphore::new(32));
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            Some(_) = jobs.join_next(), if !jobs.is_empty() => {},
            accepted = listener.accept() => {
                let Ok((socket, peer)) = accepted else { break };
                if !peer.ip().is_loopback() { continue; }
                let Ok(permit) = slots.clone().try_acquire_owned() else { continue };
                let (tls, origin, state, cancel) = (tls.clone(), origin.clone(), state.clone(), cancel.clone());
                jobs.spawn(async move {
                    let _permit = permit;
                    let _ = connection(socket, tls, &origin, state, cancel).await;
                });
            }
        }
    }
    jobs.abort_all();
    while jobs.join_next().await.is_some() {}
    let mut state = state.lock();
    for (_, connection) in state.connections.drain() {
        connection.cancel.cancel();
    }
    state.pairings.clear();
    state.sessions = OfficeSessions::new(Duration::from_secs(30)).expect("nonzero timeout");
}

// RAII cleanup also runs if the server aborts a connection during shutdown.
struct ExecutionLease {
    state: Arc<Mutex<State>>,
    owner: ControlSessionId,
    handle: OfficeHandle,
    armed: bool,
}
impl Drop for ExecutionLease {
    fn drop(&mut self) {
        if !self.armed { return; }
        let mut state = self.state.lock();
        if let Some(connection) = state.connections.remove(&self.handle.generation) {
            connection.cancel.cancel();
        }
        let _ = state.sessions.disconnect(&self.owner, &self.handle);
    }
}

struct Lease {
    state: Arc<Mutex<State>>,
    owner: ControlSessionId,
    handle: OfficeHandle,
}
impl Drop for Lease {
    fn drop(&mut self) {
        let mut state = self.state.lock();
        state.connections.remove(&self.handle.generation);
        let _ = state.sessions.disconnect(&self.owner, &self.handle);
    }
}

async fn connection(
    socket: TcpStream,
    tls: tokio_rustls::TlsAcceptor,
    origin: &str,
    state: Arc<Mutex<State>>,
    stop: CancellationToken,
) -> Result<()> {
    let tls = tokio::time::timeout(TIMEOUT, tls.accept(socket)).await??;
    let callback = |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                    response| {
        if request.uri().path() == "/office"
            && request.uri().query().is_none()
            && request.headers().get_all("origin").iter().count() == 1
            && request.headers().get("origin").and_then(|v| v.to_str().ok()) == Some(origin)
        {
            Ok(response)
        } else {
            let mut rejected = tokio_tungstenite::tungstenite::http::Response::new(Some(
                "Office origin denied".to_owned(),
            ));
            *rejected.status_mut() = tokio_tungstenite::tungstenite::http::StatusCode::FORBIDDEN;
            Err(rejected)
        }
    };
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_BYTES))
        .max_frame_size(Some(MAX_BYTES));
    let mut socket = tokio::time::timeout(
        TIMEOUT,
        tokio_tungstenite::accept_hdr_async_with_config(tls, callback, Some(config)),
    )
    .await??;
    let first =
        tokio::time::timeout(TIMEOUT, socket.next()).await?.context("Office pairing missing")??;
    let Incoming::Pair { token, surface, document_id, title, capabilities: names } = parse(first)?
    else {
        bail!("Office pairing required")
    };
    let (send, mut outgoing) = mpsc::channel(1);
    let cancel = stop.child_token();
    let lease_state = state.clone();
    let lease = {
        let mut state = state.lock();
        let grant = state.pairings.remove(&token).context("Office approval missing")?;
        ensure!(
            grant.expires > Instant::now()
                && surface == surface_name(grant.surface)
                && document_id == grant.document.0,
            "Office approval expired or identity changed"
        );
        let caps = capabilities(names)?;
        let handle = state
            .sessions
            .register(
                grant.owner.clone(),
                grant.surface,
                grant.document,
                title,
                caps,
                Instant::now(),
            )
            .map_err(|_| anyhow::anyhow!("invalid Office registration"))?;
        // Re-registering a document closes its previous socket and pending result.
        let active: Vec<_> = state
            .sessions
            .discover(&grant.owner, Instant::now())
            .into_iter()
            .map(|s| s.handle.generation)
            .collect();
        state.connections.retain(|id, connection| {
            if connection.owner == grant.owner && !active.contains(id) {
                connection.cancel.cancel();
                false
            } else {
                true
            }
        });
        state.connections.insert(
            handle.generation,
            Connection {
                owner: grant.owner.clone(),
                expires: Instant::now() + LIFETIME,
                send,
                cancel: cancel.clone(),
                pending: None,
            },
        );
        Lease { state: lease_state, owner: grant.owner, handle }
    };
    socket.send(Message::text(json!({"type":"paired", "surface":surface_name(lease.handle.surface), "document_id":lease.handle.document_id.0, "generation":lease.handle.generation.to_string()}).to_string())).await?;
    let mut sequence = 0_u64;
    let lifetime = tokio::time::sleep(LIFETIME);
    tokio::pin!(lifetime);
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            _ = &mut lifetime => break,
            message = outgoing.recv() => {
                let Some(message) = message else { break };
                // Revoke wins over an already queued command or blocked socket write.
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => break,
                    result = tokio::time::timeout(TIMEOUT, socket.send(Message::text(message.to_string()))) => { result??; }
                }
            }
            message = tokio::time::timeout(Duration::from_secs(30), socket.next()) => {
                let Some(message) = message? else { break };
                let message = message?;
                if message.is_close() { break; }
                let incoming = parse(message)?;
                {
                    let mut state = state.lock();
                    let connection = state.connections.get_mut(&lease.handle.generation).context("Office disconnected")?;
                    ensure!(connection.expires > Instant::now() && !connection.cancel.is_cancelled(), "Office connection expired");
                    let received = match &incoming { Incoming::Heartbeat { sequence } | Incoming::Result { sequence, .. } => *sequence, _ => bail!("Office already paired") };
                    ensure!(Some(received) == sequence.checked_add(1), "Office request replay or sequence gap");
                    sequence = received;
                    if let Incoming::Result { request_id, value, .. } = incoming {
                        let pending = connection.pending.take().context("unsolicited Office result")?;
                        ensure!(pending.id.to_string() == request_id, "Office result identity mismatch");
                        let _ = pending.reply.send(value);
                    }
                    state.sessions.heartbeat(&lease.owner, &lease.handle, Instant::now()).map_err(|_| anyhow::anyhow!("Office session expired"))?;
                }
                tokio::time::timeout(TIMEOUT, socket.send(Message::text(json!({"type":"heartbeat", "sequence":sequence}).to_string()))).await??;
            }
        }
    }
    Ok(())
}

fn parse(message: Message) -> Result<Incoming> {
    let Message::Text(text) = message else { bail!("Office requires text messages") };
    ensure!(text.len() <= MAX_BYTES, "Office message too large");
    serde_json::from_str(&text).context("invalid Office message")
}

#[cfg(all(test, unix))]
#[path = "transport_tests.rs"]
mod tests;

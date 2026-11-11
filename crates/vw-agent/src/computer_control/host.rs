//! User-local IPC boundary. Only trusted host code may issue tickets or mutate policy.
//! The launcher passes a connected Unix stream (no TCP listener) and delivers the
//! one-use ticket out of band. A client cannot select its OS user or grant permissions.
//! Backends must recheck platform focus/identity and honor cancellation before effects.

use super::policy::ComputerControlPolicy;
use crate::security::pairing::constant_time_eq;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::HashMap,
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::UnixStream,
    sync::mpsc,
    time::{Instant, timeout},
};
use tokio_util::sync::CancellationToken;
use vw_api_types::computer_control::*;

const MAX_FRAME: usize = 1024 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_LIFETIME: Duration = Duration::from_secs(300);

/// Deliberately has no Debug implementation: credentials must never enter logs.
pub struct SessionTicket {
    pub session_id: ControlSessionId,
    pub credential: String,
    pub permission_version: PermissionVersion,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case", deny_unknown_fields)]
pub enum HostMessage {
    Hello { session_id: ControlSessionId, credential: String },
    Action { sequence: u64, request: ComputerControlRequest },
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case", deny_unknown_fields)]
pub enum HostReply {
    Ready,
    Rejected { error: ControlErrorCode },
    Completed { request_id: ControlRequestId, outcome: ControlOutcome },
}

struct Session {
    credential: String,
    target: ComputerControlTarget,
    version: PermissionVersion,
    expires: Instant,
    next_sequence: u64,
    attached: bool,
    cancelled: CancellationToken,
}

pub struct ControlHost {
    local_user: String,
    uid: u32,
    policy: Arc<RwLock<ComputerControlPolicy>>,
    /// Set by trusted platform lock observation, never from a wire request.
    locked: Arc<AtomicBool>,
    sessions: Mutex<HashMap<ControlSessionId, Session>>,
}

impl ControlHost {
    pub fn new(policy: Arc<RwLock<ComputerControlPolicy>>, locked: Arc<AtomicBool>) -> Self {
        // SAFETY: geteuid has no preconditions and returns the current OS identity.
        let uid = unsafe { libc::geteuid() };
        Self {
            local_user: format!("uid:{uid}"),
            uid,
            policy,
            locked,
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// Called by the trusted local controller after selecting a concrete target.
    /// Tickets are memory-only, bounded-lived, and cannot authorize another target.
    pub fn issue(
        &self,
        target: ComputerControlTarget,
        lifetime: Duration,
    ) -> Result<SessionTicket, ControlErrorCode> {
        if lifetime.is_zero() || lifetime > MAX_LIFETIME {
            return Err(ControlErrorCode::InvalidRequest);
        }
        let policy = self.policy.read();
        let version = policy.version();
        policy.authorize(&self.local_user, &target, self.locked.load(Ordering::SeqCst), version)?;
        drop(policy); // Never hold policy and session locks in opposite orders.
        let mut secret = [0u8; 32];
        getrandom::fill(&mut secret).map_err(|_| ControlErrorCode::BackendFailure)?;
        let credential = hex::encode(secret);
        let session_id = ControlSessionId(uuid::Uuid::new_v4().to_string());
        let mut sessions = self.sessions.lock();
        sessions.retain(|_, session| session.attached || session.expires > Instant::now());
        sessions.insert(
            session_id.clone(),
            Session {
                credential: credential.clone(),
                target,
                version,
                expires: Instant::now() + lifetime,
                next_sequence: 1,
                attached: false,
                cancelled: CancellationToken::new(),
            },
        );
        Ok(SessionTicket { session_id, credential, permission_version: version })
    }

    /// Trusted stop entry point. A stopped session cannot be resumed with its ticket.
    pub fn stop(&self, id: &ControlSessionId) {
        if let Some(session) = self.sessions.lock().remove(id) {
            session.cancelled.cancel();
        }
    }

    #[cfg(test)]
    pub(super) fn session_count(&self) -> usize {
        self.sessions.lock().len()
    }

    fn attach(
        &self,
        id: &ControlSessionId,
        credential: &str,
    ) -> Result<(Instant, CancellationToken), ControlErrorCode> {
        let mut sessions = self.sessions.lock();
        if sessions.get(id).is_some_and(|session| session.expires <= Instant::now()) {
            sessions.remove(id);
            return Err(ControlErrorCode::PermissionDenied);
        }
        let session = sessions.get_mut(id).ok_or(ControlErrorCode::PermissionDenied)?;
        if session.attached
            || session.expires <= Instant::now()
            || !constant_time_eq(&session.credential, credential)
        {
            return Err(ControlErrorCode::PermissionDenied);
        }
        session.attached = true;
        session.credential.clear();
        Ok((session.expires, session.cancelled.clone()))
    }

    fn authorize(
        &self,
        id: &ControlSessionId,
        sequence: u64,
        request: &ComputerControlRequest,
    ) -> Result<(), ControlErrorCode> {
        if &request.session_id != id {
            return Err(ControlErrorCode::PermissionDenied);
        }
        let sessions = &mut *self.sessions.lock();
        let session = sessions.get_mut(id).ok_or(ControlErrorCode::Disconnected)?;
        if session.cancelled.is_cancelled() || session.expires <= Instant::now() {
            return Err(ControlErrorCode::Cancelled);
        }
        if request.target != session.target {
            return Err(ControlErrorCode::TargetChanged);
        }
        if request.permission_version != session.version {
            return Err(ControlErrorCode::StalePermission);
        }
        if sequence != session.next_sequence
            || sequence == u64::MAX
            || request.request_id.0.is_empty()
            || request.request_id.0.len() > 128
        {
            return Err(ControlErrorCode::InvalidRequest);
        }
        if !matches!(
            (&request.target, &request.action),
            (_, ComputerControlAction::Stop)
                | (
                    ComputerControlTarget::NativeApp { .. },
                    ComputerControlAction::NativeObserve { .. }
                        | ComputerControlAction::NativeInput { .. }
                )
                | (ComputerControlTarget::ChromeTab { .. }, ComputerControlAction::Chrome { .. })
                | (
                    ComputerControlTarget::ExcelWorkbook { .. },
                    ComputerControlAction::Excel { .. }
                )
                | (ComputerControlTarget::WordDocument { .. }, ComputerControlAction::Word { .. })
        ) {
            return Err(ControlErrorCode::InvalidRequest);
        }
        self.policy.read().authorize(
            &self.local_user,
            &session.target,
            self.locked.load(Ordering::SeqCst),
            session.version,
        )?;
        session.next_sequence += 1;
        Ok(())
    }

    /// One in-flight action per connection. While waiting, only an authenticated
    /// Stop may arrive; pipelining fails closed. Dropping/aborting serve drops the
    /// execution future and cancels the token, so backends must not detach effects.
    pub async fn serve<F, Fut>(&self, mut stream: UnixStream, execute: F) -> std::io::Result<()>
    where
        F: Fn(ComputerControlRequest, CancellationToken) -> Fut,
        Fut: Future<Output = Result<ControlOutput, ControlErrorCode>>,
    {
        if stream.peer_cred()?.uid() != self.uid {
            return write_frame(
                &mut stream,
                &HostReply::Rejected { error: ControlErrorCode::PermissionDenied },
            )
            .await;
        }
        let hello = timeout(IO_TIMEOUT, read_frame::<_, HostMessage>(&mut stream)).await;
        let (id, expires, cancel) = match hello {
            Ok(Ok(HostMessage::Hello { session_id, credential })) => {
                match self.attach(&session_id, &credential) {
                    Ok((expires, cancel)) => (session_id, expires, cancel),
                    Err(error) => {
                        return write_frame(&mut stream, &HostReply::Rejected { error }).await;
                    }
                }
            }
            _ => {
                return write_frame(
                    &mut stream,
                    &HostReply::Rejected { error: ControlErrorCode::InvalidRequest },
                )
                .await;
            }
        };
        let _session = SessionGuard { host: self, id: id.clone(), cancel: cancel.clone() };
        write_frame(&mut stream, &HostReply::Ready).await?;
        let (mut reader, mut writer) = stream.into_split();
        let (tx, mut rx) = mpsc::channel(1);
        let disconnect = CancellationToken::new();
        let disconnected = disconnect.clone();
        let _reader = ReaderGuard(tokio::spawn(async move {
            loop {
                match read_frame::<_, HostMessage>(&mut reader).await {
                    Ok(message) => {
                        if tx.send(Ok(message)).await.is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        if error.kind() == std::io::ErrorKind::UnexpectedEof {
                            disconnected.cancel();
                        }
                        let _ = tx.send(Err(error)).await;
                        break;
                    }
                }
            }
        }));
        loop {
            let message = tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                _ = disconnect.cancelled() => break,
                _ = tokio::time::sleep_until(expires) => break,
                message = rx.recv() => message,
            };
            let (sequence, request) = match message {
                Some(Ok(HostMessage::Action { sequence, request })) => (sequence, request),
                _ => {
                    write_frame(
                        &mut writer,
                        &HostReply::Rejected { error: ControlErrorCode::InvalidRequest },
                    )
                    .await?;
                    break;
                }
            };
            if let Err(error) = self.authorize(&id, sequence, &request) {
                write_frame(&mut writer, &HostReply::Rejected { error }).await?;
                break;
            }
            let request_id = request.request_id.clone();
            if matches!(request.action, ComputerControlAction::Stop) {
                cancel.cancel();
                write_frame(
                    &mut writer,
                    &HostReply::Completed {
                        request_id,
                        outcome: ControlOutcome::Succeeded { output: ControlOutput::Acknowledged },
                    },
                )
                .await?;
                break;
            }
            // Authorization above is synchronous and immediately precedes dispatch.
            let result = tokio::select! {
                biased;
                _ = cancel.cancelled() => Err(ControlErrorCode::Cancelled),
                _ = disconnect.cancelled() => Err(ControlErrorCode::Disconnected),
                _ = tokio::time::sleep_until(expires) => { cancel.cancel(); Err(ControlErrorCode::Cancelled) },
                message = rx.recv() => {
                    match message {
                        Some(Ok(HostMessage::Action { sequence, request })) => {
                            let validation = self.authorize(&id, sequence, &request);
                            let error = match validation {
                                Err(error) => error,
                                Ok(()) if matches!(request.action, ComputerControlAction::Stop) => ControlErrorCode::Cancelled,
                                Ok(()) => ControlErrorCode::InvalidRequest,
                            };
                            cancel.cancel();
                            if error != ControlErrorCode::Cancelled {
                                write_frame(&mut writer, &HostReply::Rejected { error }).await?;
                                break;
                            }
                            Err(error)
                        },
                        _ => { cancel.cancel(); Err(ControlErrorCode::Disconnected) }
                    }
                },
                result = execute(request, cancel.clone()) => result,
            };
            let outcome = match result {
                Ok(output) => ControlOutcome::Succeeded { output },
                Err(error) => ControlOutcome::Rejected { error },
            };
            write_frame(&mut writer, &HostReply::Completed { request_id, outcome }).await?;
            if cancel.is_cancelled() || disconnect.is_cancelled() {
                break;
            }
        }
        Ok(())
    }
}

struct SessionGuard<'a> {
    host: &'a ControlHost,
    id: ControlSessionId,
    cancel: CancellationToken,
}
impl Drop for SessionGuard<'_> {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.host.sessions.lock().remove(&self.id);
    }
}
struct ReaderGuard(tokio::task::JoinHandle<()>);
impl Drop for ReaderGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Length-delimited JSON; cap allocation before decoding and never log payloads.
pub async fn read_frame<R: AsyncRead + Unpin, T: DeserializeOwned>(
    reader: &mut R,
) -> std::io::Result<T> {
    let length = reader.read_u32().await? as usize;
    if length == 0 || length > MAX_FRAME {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid control frame length",
        ));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).await?;
    serde_json::from_slice(&bytes)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid control frame"))
}

pub async fn write_frame<W: AsyncWrite + Unpin, T: Serialize>(
    writer: &mut W,
    message: &T,
) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(message).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid control response")
    })?;
    if bytes.len() > MAX_FRAME {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "control response too large",
        ));
    }
    timeout(IO_TIMEOUT, async {
        writer.write_u32(bytes.len() as u32).await?;
        writer.write_all(&bytes).await?;
        writer.flush().await
    })
    .await
    .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "control write timed out"))?
}

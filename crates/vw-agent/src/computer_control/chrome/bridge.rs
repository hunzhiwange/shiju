//! Native messaging over a same-user Unix stream from the bounded Python relay.
//! Pairing proves transport readiness only. `serve` routes every action through
//! ControlHost's one-use ticket, live policy, target/version and cancellation checks.
use super::origin_policy::{DocumentBinding, canonical_origin};
use crate::computer_control::host::{ControlHost, read_frame, write_frame};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tokio::{
    net::UnixStream,
    sync::{mpsc, oneshot},
    time::timeout,
};
use vw_api_types::computer_control::*;

pub const EXTENSION_ID: &str = "ogfgcnpdhnmkpcbaclbglefaclanipmg";
const DEADLINE: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Hello {
    #[serde(rename = "type")]
    kind: String,
    version: u8,
    extension_id: String,
    challenge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChromeTarget {
    pub tab_id: u64,
    pub document_id: String,
    pub navigation_version: u64,
    pub url: String,
}

struct Command {
    action: &'static str,
    target: Option<ChromeTarget>,
    args: Value,
    reply: oneshot::Sender<Result<Value, ControlErrorCode>>,
}

pub struct ChromeBridge {
    session_id: ControlSessionId,
    binding: Mutex<DocumentBinding>,
    commands: mpsc::Sender<Command>,
    available: Arc<AtomicBool>,
    worker: tokio::task::JoinHandle<()>,
}

impl Drop for ChromeBridge {
    fn drop(&mut self) {
        self.worker.abort();
    }
}
struct Reader(tokio::task::JoinHandle<()>);
impl Drop for Reader {
    fn drop(&mut self) {
        self.0.abort();
    }
}
struct Availability(Arc<AtomicBool>);
impl Drop for Availability {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl ChromeBridge {
    pub async fn pair(
        mut stream: UnixStream,
        session_id: ControlSessionId,
    ) -> Result<Self, ControlErrorCode> {
        if stream.peer_cred().map_err(|_| ControlErrorCode::PermissionDenied)?.uid()
            // SAFETY: geteuid has no preconditions.
            != unsafe { libc::geteuid() }
            || session_id.0.is_empty()
            || session_id.0.len() > 128
        {
            return Err(ControlErrorCode::PermissionDenied);
        }
        let hello: Hello = timeout(DEADLINE, read_frame(&mut stream))
            .await
            .map_err(|_| ControlErrorCode::Disconnected)?
            .map_err(|_| ControlErrorCode::InvalidRequest)?;
        if hello.kind != "hello"
            || hello.version != 1
            || hello.extension_id != EXTENSION_ID
            || hello.challenge.is_empty()
            || hello.challenge.len() > 128
        {
            return Err(ControlErrorCode::PermissionDenied);
        }
        write_frame(
            &mut stream,
            &json!({"type":"paired", "version":1,
            "challenge":hello.challenge, "sessionId":session_id.0}),
        )
        .await
        .map_err(|_| ControlErrorCode::Disconnected)?;
        let (commands, mut incoming) = mpsc::channel::<Command>(1);
        let available = Arc::new(AtomicBool::new(true));
        let guard = Availability(available.clone());
        let available_worker = available.clone();
        let session = session_id.0.clone();
        let (mut reader, mut writer) = stream.into_split();
        let (frames, mut replies) = mpsc::channel(1);
        let reader = Reader(tokio::spawn(async move {
            loop {
                let frame = read_frame::<_, Value>(&mut reader).await;
                let failed = frame.is_err();
                if frames.send(frame).await.is_err() || failed {
                    break;
                }
            }
        }));
        let worker = tokio::spawn(async move {
            let _guard = guard;
            let _reader = reader;
            for sequence in 1..=4096u64 {
                let mut command = tokio::select! {
                    biased;
                    // Unsolicited frames and EOF both invalidate the connection.
                    _ = replies.recv() => break,
                    command = incoming.recv() => match command { Some(command) => command, None => break },
                };
                if command.reply.is_closed() {
                    break;
                }
                let id = format!("chrome-{sequence}");
                let request = json!({"type":"request", "version":1, "sessionId":session,
                    "id":id, "seq":sequence, "action":command.action,
                    "target":command.target, "args":command.args});
                let result = timeout(DEADLINE, async {
                    tokio::select! {
                        biased;
                        _ = command.reply.closed() => return Err(std::io::Error::from(std::io::ErrorKind::Interrupted)),
                        result = write_frame(&mut writer, &request) => result?,
                    }

                    tokio::select! {
                        biased;
                        _ = command.reply.closed() => Err(std::io::Error::from(std::io::ErrorKind::Interrupted)),
                        frame = replies.recv() => frame.unwrap_or_else(|| Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))),
                    }
                }).await;
                let reply = match result {
                    Ok(Ok(value))
                        if value["type"] == "response"
                            && value["version"] == 1
                            && value["sessionId"] == session
                            && value["id"] == id
                            && value["ok"].is_boolean() =>
                    {
                        value
                    }
                    _ => {
                        available_worker.store(false, Ordering::SeqCst);
                        let _ = command.reply.send(Err(ControlErrorCode::Disconnected));
                        break;
                    }
                };
                let result = if reply["ok"] == true {
                    reply.get("data").cloned().ok_or(ControlErrorCode::BackendFailure)
                } else {
                    Err(match reply["error"].as_str() {
                        Some("stale_target") => ControlErrorCode::TargetChanged,
                        Some("transfer_not_approved") => ControlErrorCode::PermissionDenied,
                        _ => ControlErrorCode::BackendFailure,
                    })
                };
                if command.reply.send(result).is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            session_id,
            binding: Mutex::new(DocumentBinding::default()),
            commands,
            available,
            worker,
        })
    }

    pub fn is_available(&self) -> bool {
        self.available.load(Ordering::SeqCst)
    }

    async fn exchange(
        &self,
        action: &'static str,
        target: Option<ChromeTarget>,
        args: Value,
    ) -> Result<Value, ControlErrorCode> {
        let (reply, receiver) = oneshot::channel();
        self.commands
            .send(Command { action, target, args, reply })
            .await
            .map_err(|_| ControlErrorCode::Disconnected)?;
        receiver.await.map_err(|_| ControlErrorCode::Disconnected)?
    }

    /// Enumerates only documents explicitly granted through the Chrome action.
    /// This is transport metadata, never a persistent permission grant.
    pub async fn enumerate(&self) -> Result<Value, ControlErrorCode> {
        self.exchange("enumerate", None, json!({})).await
    }

    pub(super) async fn execute(
        &self,
        request: ComputerControlRequest,
        host: &ControlHost,
    ) -> Result<ControlOutput, ControlErrorCode> {
        host.recheck(&request)?;
        if request.session_id != self.session_id {
            return Err(ControlErrorCode::PermissionDenied);
        }
        let ComputerControlTarget::ChromeTab { tab_id, ref origin } = request.target else {
            return Err(ControlErrorCode::InvalidRequest);
        };
        let ComputerControlAction::Chrome { ref command } = request.action else {
            return Err(ControlErrorCode::InvalidRequest);
        };
        let target = self.current_target(tab_id, origin).await?;
        host.recheck(&request)?;
        let (action, args) = match command {
            ChromeCommand::Snapshot => ("read", json!({})),
            ChromeCommand::Click { selector } if !selector.is_empty() && selector.len() <= 2048 => {
                ("click", json!({"selector":selector}))
            }
            ChromeCommand::Fill { selector, text }
                if !selector.is_empty() && selector.len() <= 2048 && text.len() <= 16000 =>
            {
                ("input", json!({"selector":selector,"text":text}))
            }
            ChromeCommand::Navigate { url } => {
                host.authorize_destination(&request, canonical_origin(url)?)?;
                ("navigate", json!({"url":url}))
            }
            ChromeCommand::Upload { selector, file_name, content_base64 } => {
                validate_upload(selector, file_name, content_base64)?;
                host.consume_transfer(&request)?;
                (
                    "upload",
                    json!({"selector":selector,"fileName":file_name,"contentBase64":content_base64,"approved":true}),
                )
            }
            ChromeCommand::Download { selector, url }
                if !selector.is_empty() && selector.len() <= 2048 =>
            {
                if canonical_origin(url)? != *origin {
                    return Err(ControlErrorCode::PermissionDenied);
                }
                host.consume_transfer(&request)?;
                ("download", json!({"selector":selector,"url":url,"approved":true}))
            }
            // Screenshots require the host-owned media resolver.
            ChromeCommand::Screenshot
            | ChromeCommand::Press { .. }
            | ChromeCommand::Scroll { .. } => return Err(ControlErrorCode::Unsupported),
            _ => return Err(ControlErrorCode::InvalidRequest),
        };
        host.recheck(&request)?;
        let result = self.exchange(action, Some(target), args).await;
        if action == "navigate" {
            self.binding.lock().invalidate();
        }
        let data = result?;
        host.recheck(&request)?;
        // Readback is discarded when an action navigated, redirected or lost its
        // document. Navigation deliberately invalidates the old lease instead.
        if action != "navigate" {
            self.current_target(tab_id, origin).await?;
        }
        host.recheck(&request)?;
        match action {
            "read" => {
                let text = data["text"]
                    .as_str()
                    .filter(|text| text.len() <= 256000)
                    .ok_or(ControlErrorCode::BackendFailure)?;
                Ok(ControlOutput::Observation { text: text.into(), screenshots: vec![] })
            }
            "click" if data["clicked"] == true => Ok(ControlOutput::Acknowledged),
            "input" if data["entered"] == true => {
                if host.restricted_session() {
                    let ChromeCommand::Fill { text, .. } = command else {
                        return Err(ControlErrorCode::InvalidRequest);
                    };
                    if data["value"].as_str() != Some(text.as_str()) {
                        return Err(ControlErrorCode::BackendFailure);
                    }
                }
                Ok(ControlOutput::Acknowledged)
            },
            "navigate" if data["navigated"] == true => Ok(ControlOutput::Acknowledged),
            "upload" if data["uploaded"] == true => Ok(ControlOutput::Acknowledged),
            "download" => {
                use base64::Engine;
                let content = data["contentBase64"]
                    .as_str()
                    .filter(|s| s.len() <= 349528)
                    .ok_or(ControlErrorCode::BackendFailure)?;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(content)
                    .map_err(|_| ControlErrorCode::BackendFailure)?;
                if bytes.len() > 262144 {
                    return Err(ControlErrorCode::BackendFailure);
                }
                Ok(ControlOutput::ChromeDownload { content_base64: content.into() })
            }
            _ => Err(ControlErrorCode::BackendFailure),
        }
    }

    async fn current_target(
        &self,
        tab_id: u64,
        origin: &str,
    ) -> Result<ChromeTarget, ControlErrorCode> {
        let targets: Vec<ChromeTarget> = serde_json::from_value(self.enumerate().await?)
            .map_err(|_| ControlErrorCode::BackendFailure)?;
        let Some(target) = targets.into_iter().find(|target| target.tab_id == tab_id) else {
            self.binding.lock().invalidate();
            return Err(ControlErrorCode::TargetChanged);
        };
        self.binding.lock().check(&target, origin)?;
        Ok(target)
    }

    /// The controller must send ControlHost's Hello with its out-of-band ticket.
    /// No credential is delivered to the extension or persisted by Python.
    /// Own the bridge so controller EOF, stop or abort also closes the native port.
    pub async fn serve(self, host: &ControlHost, controller: UnixStream) -> std::io::Result<()> {
        host.serve(controller, |request, _cancel| self.execute(request, host)).await
    }
}

/// No filesystem paths, MIME-dependent executable payloads or unbounded frames.
pub fn validate_upload(selector: &str, name: &str, content: &str) -> Result<(), ControlErrorCode> {
    use base64::Engine;
    if selector.is_empty()
        || selector.len() > 2048
        || name.is_empty()
        || name.len() > 128
        || !name.as_bytes()[0].is_ascii_alphanumeric()
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        || content.len() > 349528
    {
        return Err(ControlErrorCode::InvalidRequest);
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(content)
        .map_err(|_| ControlErrorCode::InvalidRequest)?;
    if bytes.len() > 262144 {
        return Err(ControlErrorCode::InvalidRequest);
    }
    Ok(())
}

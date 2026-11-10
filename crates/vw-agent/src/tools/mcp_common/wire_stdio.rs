//! Decode raw results before rmcp's typed model, retaining the existing owned-child lifecycle.
use futures_util::StreamExt;
use rmcp::{
    RoleClient,
    model::{ClientJsonRpcMessage, ServerJsonRpcMessage},
    transport::Transport,
};
use serde_json::Value;
use std::{io, process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::AsyncWriteExt,
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::Mutex,
};
use tokio_util::codec::{FramedRead, LinesCodec};

pub(super) fn decode(
    mut value: Value,
    responses: &crate::plugins::RawMcpResponse,
) -> Result<ServerJsonRpcMessage, serde_json::Error> {
    responses.capture(&value);
    crate::plugins::preserve_mcp_wire_result(&mut value);
    serde_json::from_value(value)
}

fn nonstandard_notification(value: &Value) -> bool {
    if value.get("id").is_some() {
        return false;
    }
    let Some(method) = value.get("method").and_then(Value::as_str) else { return false };
    // Preserve rmcp 0.8's non-MCP notification compatibility without its raw-frame debug log.
    // Standard malformed messages and unsupported requests still fail decoding.
    !matches!(
        method,
        "initialize"
            | "ping"
            | "prompts/get"
            | "prompts/list"
            | "resources/list"
            | "resources/read"
            | "resources/subscribe"
            | "resources/unsubscribe"
            | "resources/templates/list"
            | "tools/call"
            | "tools/list"
            | "completion/complete"
            | "logging/setLevel"
            | "roots/list"
            | "sampling/createMessage"
            | "notifications/cancelled"
            | "notifications/initialized"
            | "notifications/message"
            | "notifications/progress"
            | "notifications/prompts/list_changed"
            | "notifications/resources/list_changed"
            | "notifications/resources/updated"
            | "notifications/roots/list_changed"
            | "notifications/tools/list_changed"
    )
}

pub(super) struct StdioTransport {
    child: Option<Child>,
    input: Arc<Mutex<Option<ChildStdin>>>,
    output: FramedRead<ChildStdout, LinesCodec>,
    responses: Arc<crate::plugins::RawMcpResponse>,
}

impl StdioTransport {
    pub(super) fn new(
        mut command: Command,
        responses: Arc<crate::plugins::RawMcpResponse>,
        connection_check: bool,
        probe_child: Option<&mut Option<ProbeChild>>,
    ) -> io::Result<Self> {
        #[cfg(unix)]
        if probe_child.is_some() {
            command.process_group(0);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(if connection_check { Stdio::null() } else { Stdio::inherit() })
            .kill_on_drop(true)
            .spawn()?;
        let input = child.stdin.take();
        let output = child.stdout.take();
        // Transfer ownership before any await. The batch retains the child even if
        // cancellation drops the initialization future before rmcp returns a peer.
        let child = match probe_child {
            Some(owned) => {
                owned.replace(ProbeChild(child));
                None
            }
            None => Some(child),
        };
        let input = input.ok_or_else(|| io::Error::other("MCP stdin unavailable"))?;
        let output = output.ok_or_else(|| io::Error::other("MCP stdout unavailable"))?;
        Ok(Self {
            responses,
            child,
            input: Arc::new(Mutex::new(Some(input))),
            // Parse JSON here rather than using rmcp's codec, which logs malformed raw frames.
            output: FramedRead::new(output, LinesCodec::new()),
        })
    }
}

impl Transport<RoleClient> for StdioTransport {
    type Error = io::Error;

    fn send(
        &mut self,
        message: ClientJsonRpcMessage,
    ) -> impl Future<Output = io::Result<()>> + Send + 'static {
        let input = self.input.clone();
        async move {
            let mut bytes = serde_json::to_vec(&message).map_err(io::Error::other)?;
            bytes.push(b'\n');
            let mut input = input.lock().await;
            let input = input.as_mut().ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotConnected, "MCP transport is closed")
            })?;
            input.write_all(&bytes).await?;
            input.flush().await
        }
    }

    async fn receive(&mut self) -> Option<ServerJsonRpcMessage> {
        loop {
            match self.output.next().await? {
                Ok(line) if line.trim().is_empty() => continue,
                Ok(line) => match serde_json::from_str::<Value>(&line).and_then(|value| {
                    if nonstandard_notification(&value) {
                        return Ok(None);
                    }
                    decode(value, &self.responses).map(Some)
                }) {
                    Ok(None) => continue,
                    Ok(Some(message)) => return Some(message),
                    Err(_) => {
                        tracing::warn!("MCP server sent an invalid protocol message");
                        return None;
                    }
                },
                Err(_) => {
                    tracing::warn!("MCP server output could not be decoded");
                    return None;
                }
            }
        }
    }

    async fn close(&mut self) -> io::Result<()> {
        // Closing the owned pipe sends EOF; shutdown() alone does not drop a ChildStdin handle.
        self.input.lock().await.take();
        let Some(child) = &mut self.child else { return Ok(()) };
        match tokio::time::timeout(Duration::from_secs(3), child.wait()).await {
            Ok(status) => status.map(|_| ()),
            Err(_) => child.kill().await,
        }
    }
}

/// A dedicated stdio process and, on Unix, its private process group.
pub(crate) struct ProbeChild(Child);

impl ProbeChild {
    pub(crate) async fn kill_and_reap(mut self) {
        self.kill_group();
        let _ = self.0.kill().await;
    }

    fn kill_group(&self) {
        #[cfg(unix)]
        if let Some(pid) = self.0.id() {
            // SAFETY: the unreaped child owns the private group created with process_group(0).
            unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
        }
    }
}

impl Drop for ProbeChild {
    fn drop(&mut self) {
        self.kill_group();
    }
}

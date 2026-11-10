//! MCP Apps RPC admission. Identity comes from the host, never from page parameters.
use super::mcp_app_download::{Download, DownloadPrompt};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::{
    collections::{BTreeSet, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use vw_gateway_client::vw_api_types::mcp_app_network::McpAppNetworkPolicy;

pub struct McpAppBridge {
    session: String,
    server: String,
    origin: String,
    tools: BTreeSet<String>,
    redirect_domains: Vec<String>,
    resource_domains: Vec<String>,
    recent_links: VecDeque<Value>,
    recent_downloads: VecDeque<Value>,
    initialized: bool,
    ready: bool,
    closed: bool,
}

enum Admission {
    Reply(Value),
    OpenLink { id: Value, url: reqwest::Url },
    Download { id: Value, download: Download },
}

fn action_error(id: Value, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":-32000,"message":message}})
}

fn resolve_link(
    id: Value,
    url: &str,
    deadline: tokio::time::Instant,
    approved: Option<bool>,
    open: impl FnOnce(&str) -> bool,
) -> Value {
    // Check the clock even if a late approval won the timeout's final poll.
    if tokio::time::Instant::now() >= deadline || approved.is_none() {
        return action_error(id, "External link confirmation expired");
    }
    if approved == Some(false) {
        return action_error(id, "External link opening denied by user");
    }
    if open(url) {
        json!({"jsonrpc":"2.0","id":id,"result":{}})
    } else {
        action_error(id, "System browser could not open external link")
    }
}

fn parse_link(raw: &str) -> Option<reqwest::Url> {
    // A URL parser can normalize input such as backslashes and stripped controls.
    let authority = raw
        .strip_prefix("https://")?
        .split(['/', '?', '#'])
        .next()?;
    if raw.len() > 8192
        || authority.is_empty()
        || authority.contains(['@', '%'])
        || raw
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace() || ch == '\\')
    {
        return None;
    }
    for (index, byte) in raw.bytes().enumerate() {
        if byte == b'%'
            && u8::from_str_radix(raw.get(index + 1..index + 3)?, 16)
                .ok()?
                .is_ascii_control()
        {
            return None;
        }
    }
    let url = reqwest::Url::parse(raw).ok()?;
    url.host_str()?;
    Some(url)
}

impl McpAppBridge {
    pub fn new(
        session: String,
        server: String,
        origin: String,
        tools: BTreeSet<String>,
    ) -> Result<Self> {
        if session.is_empty() || server.is_empty() || origin.is_empty() {
            bail!("missing host identity");
        }
        Ok(Self {
            session,
            server,
            origin,
            tools,
            redirect_domains: Vec::new(),
            resource_domains: Vec::new(),
            recent_links: VecDeque::new(),
            recent_downloads: VecDeque::new(),
            initialized: false,
            ready: false,
            closed: false,
        })
    }

    /// The caller must resolve permission and execution using the bound session/server.
    pub fn receive<F>(
        &mut self,
        session: &str,
        server: &str,
        origin: &str,
        message: &str,
        execute: F,
    ) -> Result<Value>
    where
        F: FnMut(&str, &str, &str, &Value) -> Result<Value>,
    {
        match self.admit(session, server, origin, message, false, execute)? {
            Admission::Reply(response) => Ok(response),
            Admission::OpenLink { .. } | Admission::Download { .. } => {
                unreachable!("no host action handlers")
            }
        }
    }

    fn admit<F>(
        &mut self,
        session: &str,
        server: &str,
        origin: &str,
        message: &str,
        handles_host_actions: bool,
        mut execute: F,
    ) -> Result<Admission>
    where
        F: FnMut(&str, &str, &str, &Value) -> Result<Value>,
    {
        if self.closed || session != self.session || server != self.server || origin != self.origin
        {
            bail!("MCP app identity mismatch or closed bridge");
        }
        if message.len() > 1024 * 1024 {
            bail!("MCP app message too large");
        }
        let request: Value = serde_json::from_str(message)?;
        if request["jsonrpc"] != "2.0" || !request.is_object() {
            bail!("invalid JSON-RPC request");
        }
        let method = request["method"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing method"))?;
        if method == "ui/notifications/initialized" {
            if !self.initialized || self.ready || request.get("id").is_some() {
                bail!("invalid initialized notification");
            }
            self.ready = true;
            return Ok(Admission::Reply(Value::Null));
        }
        let id = request
            .get("id")
            .filter(|id| id.is_string() || id.is_i64() || id.is_u64())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("invalid request id"))?;
        let result = match method {
            "ui/initialize" => {
                if self.initialized {
                    bail!("app already initialized");
                }
                self.initialized = true;
                let mut capabilities = json!({});
                if handles_host_actions {
                    capabilities["downloadFile"] = json!({});
                    if !self.redirect_domains.is_empty() {
                        capabilities["openLinks"] = json!({});
                    }
                }
                json!({"protocolVersion":"2026-01-26","hostInfo":{"name":"ShiJu","version":env!("CARGO_PKG_VERSION")},"hostCapabilities":capabilities,"hostContext":{"platform":"desktop"}})
            }
            "ui/open-link" if handles_host_actions => {
                let denied = |id, reason| Ok(Admission::Reply(action_error(id, reason)));
                if !self.ready {
                    return denied(id, "MCP app is not ready");
                }
                let Some(url) = request["params"]["url"].as_str().and_then(parse_link) else {
                    return denied(id, "Invalid external link URL");
                };
                if !self
                    .redirect_domains
                    .contains(&url.origin().ascii_serialization())
                {
                    return denied(id, "External link origin is not allowed");
                }
                if self.recent_links.contains(&id) {
                    return denied(id, "Duplicate external link request");
                }
                // Bound replay tracking for a long-running app.
                if self.recent_links.len() == 32 {
                    self.recent_links.pop_front();
                }
                self.recent_links.push_back(id.clone());
                return Ok(Admission::OpenLink { id, url });
            }
            "ui/download-file" if handles_host_actions => {
                let denied = |id, reason| Ok(Admission::Reply(action_error(id, reason)));
                if !self.ready {
                    return denied(id, "MCP app is not ready");
                }
                if self.recent_downloads.contains(&id) {
                    return denied(id, "Duplicate download request");
                }
                let download = match Download::parse(&request["params"], &self.resource_domains) {
                    Ok(download) => download,
                    Err(reason) => return denied(id, reason),
                };
                if self.recent_downloads.len() == 32 {
                    self.recent_downloads.pop_front();
                }
                self.recent_downloads.push_back(id.clone());
                return Ok(Admission::Download { id, download });
            }
            "tools/call" => {
                if !self.ready {
                    bail!("app is not initialized");
                }
                let params = &request["params"];
                let name = params["name"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("missing tool name"))?;
                if !self.tools.contains(name) || !params["arguments"].is_object() {
                    bail!("tool or arguments denied");
                }
                execute(&self.session, &self.server, name, &params["arguments"])?
            }
            _ => bail!("unsupported MCP app method"),
        };
        Ok(Admission::Reply(
            json!({"jsonrpc":"2.0","id":id,"result":result}),
        ))
    }

    pub fn close(&mut self) {
        self.closed = true;
        self.tools.clear();
        self.recent_links.clear();
        self.recent_downloads.clear();
    }
}

/// A dedicated child has no browser commands or persisted browser profile.
pub struct McpAppProcess {
    child: std::process::Child,
    input: std::process::ChildStdin,
    events: std::sync::mpsc::Receiver<String>,
    document_active: Arc<AtomicBool>,
    bridge: McpAppBridge,
    session: String,
    server: String,
}

impl McpAppProcess {
    pub fn is_ready(&self) -> bool {
        self.bridge.ready && !self.bridge.closed
    }
    pub fn is_running(&mut self) -> bool {
        self.child.try_wait().is_ok_and(|status| status.is_none())
    }

    pub fn launch(
        helper: &std::path::Path,
        session: String,
        server: String,
        html: &str,
        tools: BTreeSet<String>,
    ) -> Result<Self> {
        Self::launch_with_policy(
            helper,
            session,
            server,
            html,
            tools,
            McpAppNetworkPolicy::default(),
        )
    }
    pub fn launch_with_policy(
        helper: &std::path::Path,
        session: String,
        server: String,
        html: &str,
        tools: BTreeSet<String>,
        csp: McpAppNetworkPolicy,
    ) -> Result<Self> {
        use std::io::{BufRead, Write};
        if html.len() > 4 * 1024 * 1024 {
            bail!("MCP app HTML too large");
        }
        let mut bridge = McpAppBridge::new(session.clone(), server.clone(), "null".into(), tools)?;
        bridge.redirect_domains.clone_from(&csp.redirect_domains);
        bridge.resource_domains.clone_from(&csp.resource_domains);
        let mut child = std::process::Command::new(helper)
            .arg("--mcp-app-stdin")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        let mut input = child.stdin.take().expect("piped stdin");
        let output = child.stdout.take().expect("piped stdout");
        if let Err(error) = writeln!(
            input,
            "{}",
            json!({"html":html,"csp":csp,"nonce":format!("{:032x}",rand::random::<u128>())})
        )
        .and_then(|_| input.flush())
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
        let (send, events) = std::sync::mpsc::sync_channel(32);
        let document_active = Arc::new(AtomicBool::new(true));
        let reader_active = document_active.clone();
        std::thread::spawn(move || {
            let mut output = std::io::BufReader::new(output);
            loop {
                let mut line = String::new();
                match std::io::Read::take(&mut output, 1024 * 1024 + 1).read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                if line.len() > 1024 * 1024 {
                    break;
                }
                if line.trim_end() == "mcp-app-closed" {
                    break;
                }
                if let Some(message) = line.strip_prefix("mcp-app ") {
                    // Never block the child pipe on an unresponsive UI consumer.
                    if send.try_send(message.to_owned()).is_err() {
                        break;
                    }
                }
            }
            reader_active.store(false, Ordering::Release);
        });
        Ok(Self {
            child,
            input,
            events,
            document_active,
            bridge,
            session,
            server,
        })
    }

    /// Execution is supplied by the owning session's existing approval path, never the page.
    pub fn poll<F>(&mut self, mut execute: F) -> Result<usize>
    where
        F: FnMut(&str, &str, &str, &Value) -> Result<Value>,
    {
        use std::io::Write;
        let mut count = 0;
        for raw in self.events.try_iter() {
            let envelope: Value = serde_json::from_str(&raw)?;
            let origin = envelope["origin"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("missing origin"))?;
            let request = &envelope["message"];
            let response = match self.bridge.receive(
                &self.session,
                &self.server,
                origin,
                &request.to_string(),
                &mut execute,
            ) {
                Ok(response) => response,
                Err(_) => {
                    json!({"jsonrpc":"2.0","id":request.get("id").cloned().unwrap_or(Value::Null),"error":{"code":-32600,"message":"MCP app request denied"}})
                }
            };
            if response.is_null() {
                continue;
            }
            writeln!(self.input, "{response}")?;
            self.input.flush()?;
            count += 1;
        }
        Ok(count)
    }

    /// Route tool calls through the session binding and user-facing actions through host dialogs.
    pub(crate) async fn poll_gateway<F, D>(
        &mut self,
        client: &vw_gateway_client::GatewayClient,
        binding: &str,
        mut confirm: impl FnMut(String) -> F,
        mut open: impl FnMut(&str) -> bool,
        mut choose_path: impl FnMut(DownloadPrompt) -> D,
        cancelled: impl Fn() -> bool,
    ) -> Result<usize>
    where
        F: std::future::Future<Output = bool>,
        D: std::future::Future<Output = Option<std::path::PathBuf>>,
    {
        use std::io::Write;
        let mut count = 0;
        while let Ok(raw) = self.events.try_recv() {
            if !self.document_active.load(Ordering::Acquire) || cancelled() {
                break;
            }
            let envelope: Value = serde_json::from_str(&raw)?;
            let origin = envelope["origin"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("missing origin"))?;
            let request = &envelope["message"];
            let mut call = None;
            let admitted = self.bridge.admit(
                &self.session,
                &self.server,
                origin,
                &request.to_string(),
                true,
                |_, _, name, args| {
                    call = Some((name.to_owned(), args.clone()));
                    Ok(Value::Null)
                },
            );
            let response = match (admitted, call) {
                (Ok(Admission::Reply(mut response)), Some((name, arguments))) => {
                    match client
                        .mcp_app_call(&self.session, &self.server, binding, &name, &arguments)
                        .await
                    {
                        Ok(result) => {
                            response["result"] = result;
                            response
                        }
                        Err(_) => {
                            json!({"jsonrpc":"2.0","id":request["id"],"error":{"code":-32600,"message":"MCP app call denied"}})
                        }
                    }
                }
                (Ok(Admission::Reply(response)), None) => response,
                (Ok(Admission::OpenLink { id, url }), _) => {
                    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
                    let decision = tokio::time::timeout_at(
                        deadline,
                        confirm(url.origin().ascii_serialization()),
                    )
                    .await;
                    if !self.is_running()
                        || !self.document_active.load(Ordering::Acquire)
                        || cancelled()
                    {
                        break;
                    }
                    resolve_link(id, url.as_str(), deadline, decision.ok(), &mut open)
                }
                (Ok(Admission::Download { id, download }), _) => {
                    let active = || self.document_active.load(Ordering::Acquire) && !cancelled();
                    let revoked = async {
                        while active() {
                            tokio::time::sleep(Duration::from_millis(25)).await;
                        }
                    };
                    let result = tokio::select! {
                        () = revoked => break,
                        result = download.save(&mut choose_path, active) => result,
                    };
                    if !active() || !self.is_running() {
                        break;
                    }
                    match result {
                        Ok(()) => json!({"jsonrpc":"2.0","id":id,"result":{}}),
                        Err(reason) => action_error(id, reason),
                    }
                }
                (Err(_), _) => {
                    json!({"jsonrpc":"2.0","id":request["id"],"error":{"code":-32600,"message":"MCP app request denied"}})
                }
            };
            if !response.is_null() {
                writeln!(self.input, "{response}")?;
                self.input.flush()?;
            }
            count += 1;
        }
        Ok(count)
    }

    /// Deliver only host-selected tool data after the handshake.
    pub fn tool_result(&mut self, arguments: Value, result: Value) -> Result<()> {
        use std::io::Write;
        if !self.bridge.ready || self.bridge.closed {
            bail!("MCP app is not ready");
        }
        for notification in [
            json!({"jsonrpc":"2.0","method":"ui/notifications/tool-input","params":{"arguments":arguments}}),
            json!({"jsonrpc":"2.0","method":"ui/notifications/tool-result","params":result}),
        ] {
            writeln!(self.input, "{notification}")?;
        }
        self.input.flush()?;
        Ok(())
    }

    pub fn close(&mut self) -> Result<()> {
        use std::io::Write;
        self.bridge.close();
        self.document_active.store(false, Ordering::Release);
        if self.child.try_wait()?.is_none() {
            // Give the page a bounded opportunity to release its resources; never execute new calls during teardown.
            let _ = writeln!(self.input, "{}", json!({"jsonrpc":"2.0","id":"host-teardown","method":"ui/resource-teardown","params":{}})).and_then(|_| self.input.flush());
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(200);
            while let Some(remaining) = deadline.checked_duration_since(std::time::Instant::now()) {
                let Ok(raw) = self.events.recv_timeout(remaining) else {
                    break;
                };
                if serde_json::from_str::<Value>(&raw).is_ok_and(|value| {
                    value["message"]["id"] == "host-teardown"
                        && value["message"].get("result").is_some()
                }) {
                    break;
                }
            }
            if self.child.try_wait()?.is_none() {
                self.child.kill()?;
            }
        }
        self.child.wait()?;
        Ok(())
    }
}

impl Drop for McpAppProcess {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
#[path = "mcp_app_bridge_tests.rs"]
mod tests;

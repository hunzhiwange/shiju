//! MCP Apps RPC admission. Identity comes from the host, never from page parameters.
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub struct McpAppBridge {
    session: String,
    server: String,
    origin: String,
    tools: BTreeSet<String>,
    initialized: bool,
    ready: bool,
    closed: bool,
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
        Ok(Self { session, server, origin, tools, initialized: false, ready: false, closed: false })
    }

    /// The caller must resolve permission and execution using the bound session/server.
    pub fn receive<F>(
        &mut self,
        session: &str,
        server: &str,
        origin: &str,
        message: &str,
        mut execute: F,
    ) -> Result<Value>
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
        let method = request["method"].as_str().ok_or_else(|| anyhow::anyhow!("missing method"))?;
        if method == "ui/notifications/initialized" {
            if !self.initialized || self.ready || request.get("id").is_some() {
                bail!("invalid initialized notification");
            }
            self.ready = true;
            return Ok(Value::Null);
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
                json!({"protocolVersion":"2026-01-26","hostInfo":{"name":"ShiJu","version":env!("CARGO_PKG_VERSION")},"hostCapabilities":{},"hostContext":{"platform":"desktop"}})
            }
            "tools/call" => {
                if !self.ready {
                    bail!("app is not initialized");
                }
                let params = &request["params"];
                let name =
                    params["name"].as_str().ok_or_else(|| anyhow::anyhow!("missing tool name"))?;
                if !self.tools.contains(name) || !params["arguments"].is_object() {
                    bail!("tool or arguments denied");
                }
                execute(&self.session, &self.server, name, &params["arguments"])?
            }
            _ => bail!("unsupported MCP app method"),
        };
        Ok(json!({"jsonrpc":"2.0","id":id,"result":result}))
    }

    pub fn close(&mut self) {
        self.closed = true;
        self.tools.clear();
    }
}

/// A dedicated child has no browser commands or persisted browser profile.
pub struct McpAppProcess {
    child: std::process::Child,
    input: std::process::ChildStdin,
    events: std::sync::mpsc::Receiver<String>,
    bridge: McpAppBridge,
    session: String,
    server: String,
}

impl McpAppProcess {
    pub fn launch(
        helper: &std::path::Path,
        session: String,
        server: String,
        html: &str,
        tools: BTreeSet<String>,
    ) -> Result<Self> {
        use std::io::{BufRead, Write};
        if html.len() > 4 * 1024 * 1024 {
            bail!("MCP app HTML too large");
        }
        let bridge = McpAppBridge::new(session.clone(), server.clone(), "null".into(), tools)?;
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
            json!({"html":html,"nonce":format!("{:032x}",rand::random::<u128>())})
        )
        .and_then(|_| input.flush())
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
        let (send, events) = std::sync::mpsc::sync_channel(32);
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
                if let Some(message) = line.strip_prefix("mcp-app ") {
                    // Never block the child pipe on an unresponsive UI consumer.
                    if send.try_send(message.to_owned()).is_err() {
                        break;
                    }
                }
            }
        });
        Ok(Self { child, input, events, bridge, session, server })
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
            let origin =
                envelope["origin"].as_str().ok_or_else(|| anyhow::anyhow!("missing origin"))?;
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
        self.bridge.close();
        if self.child.try_wait()?.is_none() {
            self.child.kill()?;
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

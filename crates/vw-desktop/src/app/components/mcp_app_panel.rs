//! A session owns each isolated application process and revokes it on disposal.
use crate::app::mcp_app_bridge::McpAppProcess;
use anyhow::{Result, bail};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
};

#[derive(Debug, Clone)]
pub struct McpAppPanel(Arc<PanelLease>);
#[derive(Debug)]
struct PanelLease {
    cancelled: Arc<AtomicBool>,
    status: Arc<AtomicU8>,
}
impl Drop for PanelLease {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

impl McpAppPanel {
    pub fn validate(app: &Value) -> Result<(&str, &str, &str, &str)> {
        let field = |name| {
            app[name]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow::anyhow!("missing MCP app {name}"))
        };
        let server = field("server")?;
        let tool = field("tool")?;
        let binding = field("execution_binding")?;
        let uri = field("resource_uri")?;
        let contents = app["resource"]["contents"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing app contents"))?;
        if !uri.starts_with("ui://")
            || contents.len() != 1
            || contents[0]["uri"] != uri
            || contents[0]["mimeType"] != "text/html;profile=mcp-app"
        {
            bail!("invalid MCP app resource association");
        }
        let html = contents[0]["text"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 4 * 1024 * 1024)
            .ok_or_else(|| anyhow::anyhow!("invalid MCP app HTML"))?;
        Self::network_policy(app)?;
        Ok((server, tool, binding, html))
    }

    pub fn network_policy(app: &Value) -> Result<Value> {
        let csp = &app["resource"]["contents"][0]["_meta"]["ui"]["csp"];
        if csp.is_null() {
            return Ok(serde_json::json!({}));
        }
        let object = csp.as_object().ok_or_else(|| anyhow::anyhow!("invalid app CSP"))?;
        for (key, value) in object {
            if !matches!(
                key.as_str(),
                "resourceDomains" | "connectDomains" | "baseUriDomains" | "redirectDomains"
            ) {
                bail!("unsupported app CSP directive");
            }
            let domains = value
                .as_array()
                .filter(|v| v.len() <= 32)
                .ok_or_else(|| anyhow::anyhow!("invalid app CSP domains"))?;
            for domain in domains {
                let origin =
                    domain.as_str().ok_or_else(|| anyhow::anyhow!("invalid app origin"))?;
                let authority = origin
                    .strip_prefix("https://")
                    .ok_or_else(|| anyhow::anyhow!("app origins require HTTPS"))?;
                if authority.is_empty()
                    || authority.len() > 253
                    || !authority
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b':'))
                    || !authority.contains('.')
                    || authority.starts_with('.')
                    || authority.ends_with('.')
                    || authority.split(':').count() > 2
                {
                    bail!("app CSP requires exact HTTPS origins");
                }
                if let Some((_, port)) = authority.split_once(':')
                    && port.parse::<u16>().ok().is_none_or(|p| p == 0)
                {
                    bail!("invalid app origin port");
                }
            }
        }
        Ok(csp.clone())
    }

    pub fn launch(
        helper: &Path,
        session: String,
        app: &Value,
        arguments: Value,
        result: Value,
        client: vw_gateway_client::GatewayClient,
    ) -> Result<Self> {
        let (server, tool, binding, html) = Self::validate(app)?;
        let binding = binding.to_owned();
        let mut process = McpAppProcess::launch_with_policy(
            helper,
            session,
            server.to_owned(),
            html,
            BTreeSet::from([tool.to_owned()]),
            Self::network_policy(app)?,
        )?;
        let cancelled = Arc::new(AtomicBool::new(false));
        let status = Arc::new(AtomicU8::new(0));
        let lease =
            Self(Arc::new(PanelLease { cancelled: cancelled.clone(), status: status.clone() }));
        std::thread::Builder::new().name("mcp-app-panel".into()).spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
            if let Ok(runtime) = runtime {
                runtime.block_on(async {
                    let mut delivered = false;
                    let started = std::time::Instant::now();
                    loop {
                        if cancelled.load(Ordering::Acquire) {
                            break;
                        }
                        if !process.is_running() {
                            if !delivered {
                                status.store(3, Ordering::Release);
                            }
                            break;
                        }
                        let poll = process.poll_gateway(&client, &binding);
                        let stop = async {
                            while !cancelled.load(Ordering::Acquire) {
                                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                            }
                        };
                        let outcome =
                            tokio::select! { result = poll => result, () = stop => break };
                        if outcome.is_err() {
                            status.store(3, Ordering::Release);
                            break;
                        }
                        if !process.is_ready()
                            && started.elapsed() > std::time::Duration::from_secs(15)
                        {
                            status.store(3, Ordering::Release);
                            break;
                        }
                        if !delivered && process.is_ready() {
                            if process.tool_result(arguments.clone(), result.clone()).is_err() {
                                status.store(3, Ordering::Release);
                                break;
                            }
                            delivered = true;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                    }
                });
            } else {
                status.store(3, Ordering::Release);
            }
            if status.load(Ordering::Acquire) != 3 {
                status.store(2, Ordering::Release);
            }
            let _ = process.close();
        })?;
        Ok(lease)
    }
    pub fn failed(&self) -> bool {
        self.0.status.load(Ordering::Acquire) == 3
    }
    pub fn finished(&self) -> bool {
        self.0.status.load(Ordering::Acquire) >= 2
    }
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
    }
}

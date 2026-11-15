//! MCP handshake, transport lifecycle and server requests.
use agent_client_protocol::{HttpHeader, McpServer, McpServerStdio};
use anyhow::Context;
use reqwest_012::header::{HeaderMap, HeaderName, HeaderValue};
use rmcp::model::{
    ClientInfo, CreateElicitationRequestParam, CreateElicitationResult, CreateMessageRequestParam,
    CreateMessageResult, ErrorData, ListRootsResult,
};
use rmcp::service::{NotificationContext, RequestContext, RoleClient};
use rmcp::transport::{
    SseClientTransport, StreamableHttpClientTransport, sse_client::SseClientConfig,
    streamable_http_client::StreamableHttpClientTransportConfig,
};
use serde_json::{Value, json};
use std::{future::Future, time::Duration};
use tokio::process::Command;

use crate::server_name;
mod wire_check;
mod wire_http;
mod wire_stdio;
pub use wire_http::ConnectionCheckCleanup;
pub use wire_stdio::ProbeChild;

/// Host actions used by an existing connection; identity and cancellation stay explicit.
pub trait ClientHost: Send + Sync {
    fn runtime_stdio_cwd(&self, server: &McpServerStdio) -> anyhow::Result<Option<std::path::PathBuf>>;
    fn http_headers<'a>(&'a self, server: &'a agent_client_protocol::McpServerHttp, owner: Option<&'a crate::FormOwner>)
        -> std::pin::Pin<Box<dyn Future<Output = anyhow::Result<Option<HeaderMap>>> + Send + 'a>>;
    fn request_elicitation(&self, owner: crate::FormOwner, server: String,
        request: CreateElicitationRequestParam, cancelled: tokio_util::sync::CancellationToken,
        closed: tokio_util::sync::CancellationToken, wire_schema: Option<Value>)
        -> std::pin::Pin<Box<dyn Future<Output = Result<CreateElicitationResult, ErrorData>> + Send + '_>>;
}

struct UnsupportedMcpCapability {
    scope: &'static str,
    capability: &'static str,
    reason: &'static str,
}

const UNSUPPORTED_MCP_CAPABILITIES: &[UnsupportedMcpCapability] = &[
    UnsupportedMcpCapability {
        scope: "client",
        capability: "sampling",
        reason: "vw-agent MCP client does not broker model sampling requests from servers.",
    },
    UnsupportedMcpCapability {
        scope: "client",
        capability: "elicitation",
        reason: "vw-agent MCP client does not collect interactive user input for servers.",
    },
    UnsupportedMcpCapability {
        scope: "client",
        capability: "roots",
        reason: "vw-agent MCP client does not expose workspace roots through the MCP roots API.",
    },
    UnsupportedMcpCapability {
        scope: "server",
        capability: "resources.subscribe",
        reason: "vw-agent MCP client performs one-shot resource reads and does not keep resource subscriptions open.",
    },
];

pub struct NoopClient {
    info: ClientInfo,
    form_owner: Option<crate::FormOwner>,
    form_server: String,
    host: Option<std::sync::Arc<dyn ClientHost>>,
    closed: tokio_util::sync::CancellationToken,
    pub catalogs: std::sync::Arc<crate::McpCatalogs>,
    pub raw_responses: std::sync::Arc<crate::RawMcpResponse>,
}

impl Default for NoopClient {
    fn default() -> Self {
        Self::with_cancellation(tokio_util::sync::CancellationToken::new())
    }
}
impl NoopClient {
    fn with_cancellation(closed: tokio_util::sync::CancellationToken) -> Self {
        Self {
            host: None,
            form_owner: None,
            form_server: String::new(),
            closed: closed.clone(),
            raw_responses: Default::default(),
            catalogs: std::sync::Arc::new(crate::McpCatalogs::new(closed)),
            info: crate::capabilities::client_info(),
        }
    }
}

pub fn unsupported_mcp_capabilities_json() -> Value {
    Value::Array(
        UNSUPPORTED_MCP_CAPABILITIES
            .iter()
            .map(|capability| {
                json!({
                    "scope": capability.scope,
                    "capability": capability.capability,
                    "reason": capability.reason,
                })
            })
            .collect(),
    )
}

fn unsupported_client_capability_error(capability: &str) -> ErrorData {
    ErrorData::invalid_request(
        format!("MCP client capability '{capability}' is not supported by vw-agent"),
        Some(json!({ "capability": capability })),
    )
}

impl rmcp::ClientHandler for NoopClient {
    fn get_info(&self) -> ClientInfo {
        self.info.clone()
    }

    fn create_message(
        &self,
        _params: CreateMessageRequestParam,
        _context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<CreateMessageResult, ErrorData>> + Send + '_ {
        std::future::ready(Err(unsupported_client_capability_error("sampling")))
    }

    fn list_roots(
        &self,
        _context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<ListRootsResult, ErrorData>> + Send + '_ {
        std::future::ready(Err(unsupported_client_capability_error("roots")))
    }

    fn create_elicitation(
        &self,
        request: CreateElicitationRequestParam,
        context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<CreateElicitationResult, ErrorData>> + Send + '_ {
        async move {
            let schema = self.raw_responses.take_form(&context.id);
            let owner = self
                .form_owner
                .clone()
                .ok_or_else(|| unsupported_client_capability_error("elicitation"))?;
            self.host.as_ref()
                .ok_or_else(|| unsupported_client_capability_error("elicitation"))?
                .request_elicitation(
                owner,
                self.form_server.clone(),
                request,
                context.ct,
                self.closed.clone(),
                Some(schema.ok_or_else(|| {
                    ErrorData::invalid_params("MCP form wire schema is unavailable", None)
                })?),
            )
            .await
        }
    }

    fn on_resource_list_changed(
        &self,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        if context
            .peer
            .peer_info()
            .and_then(|i| i.capabilities.resources.as_ref())
            .and_then(|c| c.list_changed)
            == Some(true)
        {
            self.catalogs.resources_changed();
        } else {
            tracing::debug!("ignoring unadvertised MCP resources.listChanged notification");
        }
        std::future::ready(())
    }

    fn on_tool_list_changed(
        &self,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        if context
            .peer
            .peer_info()
            .and_then(|i| i.capabilities.tools.as_ref())
            .and_then(|c| c.list_changed)
            == Some(true)
        {
            self.catalogs.tools_changed();
        } else {
            tracing::debug!("ignoring unadvertised MCP tools.listChanged notification");
        }
        std::future::ready(())
    }

    fn on_prompt_list_changed(
        &self,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        if context
            .peer
            .peer_info()
            .and_then(|i| i.capabilities.prompts.as_ref())
            .and_then(|c| c.list_changed)
            == Some(true)
        {
            self.catalogs.prompts_changed();
        } else {
            tracing::debug!("ignoring unadvertised MCP prompts.listChanged notification");
        }
        std::future::ready(())
    }
}

fn remote_header_map(headers: &[HttpHeader]) -> anyhow::Result<HeaderMap> {
    let mut header_map = HeaderMap::new();
    for header in headers {
        let name = HeaderName::from_bytes(header.name.as_bytes())
            .with_context(|| format!("invalid MCP remote header name '{}'", header.name))?;
        let value = HeaderValue::from_str(&header.value)
            .with_context(|| format!("invalid MCP remote header value for '{}'", header.name))?;
        header_map.append(name, value);
    }
    Ok(header_map)
}

fn remote_http_client(
    headers: &[HttpHeader],
    connect_timeout: Duration,
) -> anyhow::Result<reqwest_012::Client> {
    reqwest_012::Client::builder()
        .default_headers(remote_header_map(headers)?)
        .connect_timeout(connect_timeout)
        .build()
        .context("failed to build MCP remote HTTP client")
}

fn connection_check_http_client(
    headers: &[HttpHeader],
    timeout: Duration,
) -> anyhow::Result<reqwest_012::Client> {
    let mut headers = remote_header_map(headers)?;
    for (_, value) in &mut headers {
        value.set_sensitive(true);
    }
    reqwest_012::Client::builder()
        .default_headers(headers)
        .connect_timeout(timeout)
        .read_timeout(timeout)
        .retry(reqwest_012::retry::never())
        .redirect(reqwest_012::redirect::Policy::none())
        .build()
        .context("failed to build MCP connection check HTTP client")
}

async fn with_connect_timeout<T>(
    server_name: &str,
    timeout: Duration,
    operation: impl Future<Output = anyhow::Result<T>>,
) -> anyhow::Result<T> {
    match tokio::time::timeout(timeout, operation).await {
        Ok(result) => result,
        Err(_) => anyhow::bail!(
            "timed out connecting to MCP server '{}' after {} ms",
            server_name,
            timeout.as_millis()
        ),
    }
}

pub type RunningMcpPeer = rmcp::service::RunningService<rmcp::RoleClient, NoopClient>;

/// Open a durable peer using the caller's trusted host actions and interaction owner.
pub async fn open_server_peer(
    host: std::sync::Arc<dyn ClientHost>,
    server: McpServer,
    timeout: Duration,
    closed: tokio_util::sync::CancellationToken,
    form_owner: Option<crate::FormOwner>,
) -> anyhow::Result<RunningMcpPeer> {
    open_server_peer_inner(host, server, timeout, closed, form_owner, None, None, None).await
}

/// Open an explicit catalog probe with no session interaction privileges.
#[cfg(not(target_arch = "wasm32"))]
pub async fn open_connection_check_peer(
    host: std::sync::Arc<dyn ClientHost>,
    server: McpServer,
    timeout: Duration,
    http_io: tokio_util::sync::CancellationToken,
    child: Option<&mut Option<ProbeChild>>,
    cleanup: Option<ConnectionCheckCleanup>,
) -> anyhow::Result<RunningMcpPeer> {
    open_server_peer_inner(
        host, server, timeout, tokio_util::sync::CancellationToken::new(), None,
        Some(http_io), child, cleanup,
    ).await
}

async fn open_server_peer_inner(
    host: std::sync::Arc<dyn ClientHost>,
    server: McpServer,
    timeout: Duration,
    closed: tokio_util::sync::CancellationToken,
    form_owner: Option<crate::FormOwner>,
    check_http_io: Option<tokio_util::sync::CancellationToken>,
    child: Option<&mut Option<wire_stdio::ProbeChild>>,
    cleanup: Option<wire_http::ConnectionCheckCleanup>,
) -> anyhow::Result<RunningMcpPeer> {
    let connection_check = check_http_io.is_some();
    let mut peer_client = NoopClient::with_cancellation(closed);
    if form_owner.is_some() {
        peer_client.info.capabilities.elicitation =
            Some(rmcp::model::ElicitationCapability { schema_validation: Some(true) });
    }
    peer_client.host = Some(host.clone());
    peer_client.form_owner = form_owner;
    peer_client.form_server = server_name(&server).to_owned();
    let responses = peer_client.raw_responses.clone();
    let server = &server;
    let running = match server {
        McpServer::Stdio(server) => {
            let mut command = Command::new(&server.command);
            if connection_check && !server.env.iter().any(|env| env.name == "PATH") {
                shell_command::apply_augmented_path_tokio_command(&mut command);
            }
            if let Some(cwd) = host.runtime_stdio_cwd(server)? {
                command.current_dir(cwd);
            }
            let args = server.args.clone();
            let env = server
                .env
                .iter()
                .map(|env| (env.name.clone(), env.value.clone()))
                .collect::<Vec<_>>();
            command.args(args);
            for (key, value) in &env {
                command.env(key, value);
            }
            let transport =
                wire_stdio::StdioTransport::new(command, responses, connection_check, child)?;
            with_connect_timeout(
                &server.name,
                timeout,
                wire_check::serve(peer_client, transport, connection_check),
            )
            .await
            .with_context(|| format!("failed to initialize MCP peer '{}'", server.name))?
        }
        McpServer::Sse(server) => {
            let client = if connection_check {
                connection_check_http_client(&server.headers, timeout)
            } else {
                remote_http_client(&server.headers, timeout)
            }
            .with_context(|| format!("failed to build SSE MCP client for '{}'", server.name))?;
            let mut client = wire_http::HttpClient::new(client, responses);
            let mut config =
                SseClientConfig { sse_endpoint: server.url.clone().into(), ..Default::default() };
            if let Some(http_io) = check_http_io {
                client = client
                    .with_check_origin(reqwest_012::Url::parse(&server.url)?.origin())
                    .with_check_cancellation(http_io);
                config.retry_policy =
                    std::sync::Arc::new(rmcp::transport::common::client_side_sse::NeverRetry);
            }
            let transport = with_connect_timeout(&server.name, timeout, async move {
                SseClientTransport::start_with_client(client, config)
                    .await
                    .map_err(anyhow::Error::from)
            })
            .await
            .with_context(|| format!("failed to start SSE MCP transport for '{}'", server.name))?;
            with_connect_timeout(
                &server.name,
                timeout,
                wire_check::serve(peer_client, transport, connection_check),
            )
            .await
            .with_context(|| format!("failed to initialize MCP peer '{}'", server.name))?
        }
        McpServer::Http(server) => {
            let scoped_headers = host.http_headers(server, peer_client.form_owner.as_ref()).await?;
            let client = if connection_check {
                connection_check_http_client(&server.headers, timeout)
            } else if let Some(headers) = scoped_headers {
                reqwest_012::Client::builder()
                    .default_headers(headers)
                    .connect_timeout(timeout)
                    // A redirect must not forward custom credential headers to another endpoint.
                    .redirect(reqwest_012::redirect::Policy::none())
                    .build()
                    .context("failed to build source-bound plugin MCP HTTP client")
            } else {
                remote_http_client(&server.headers, timeout)
            }
            .with_context(|| format!("failed to build HTTP MCP client for '{}'", server.name))?;
            let mut config = StreamableHttpClientTransportConfig::with_uri(server.url.clone());
            if connection_check {
                config.retry_config =
                    std::sync::Arc::new(rmcp::transport::common::client_side_sse::NeverRetry);
            }
            let mut client = wire_http::HttpClient::new(client, responses);
            if let Some(http_io) = check_http_io {
                client = client
                    .with_check_origin(reqwest_012::Url::parse(&server.url)?.origin())
                    .with_check_cancellation(http_io)
                    .with_check_cleanup(cleanup);
            }
            let transport = StreamableHttpClientTransport::with_client(client, config);
            with_connect_timeout(
                &server.name,
                timeout,
                wire_check::serve(peer_client, transport, connection_check),
            )
            .await
            .with_context(|| format!("failed to initialize MCP peer '{}'", server.name))?
        }
        _ => anyhow::bail!("unsupported MCP server transport"),
    };
    // rmcp 0.8 accepts arbitrary initialize versions. Retire an unsupported peer before
    // catalog discovery or tool execution can use it.
    if let Err(error) = crate::capabilities::validate_server_protocol(running.peer_info()) {
        let _ = running.cancel().await;
        return Err(error);
    }
    Ok(running)
}


#[cfg(test)]
mod tests;

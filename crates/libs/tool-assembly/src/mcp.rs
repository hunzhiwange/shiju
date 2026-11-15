//! Dynamic MCP discovery, policy filtering and factory registration.
use anyhow::Context;
use std::sync::Arc;
use tool_types::Tool;
use tool_mcp::McpToolHost;
use plugin_mcp::mcp_connections::McpSessionConnections;

pub async fn register_dynamic_mcp_tools<H: McpToolHost<Connections = McpSessionConnections>>(
    tool_arcs: &mut Vec<Arc<dyn Tool>>,
    config: anyhow::Result<acp_config::ResolvedAcpxConfig>,
    connections: Arc<McpSessionConnections>,
) {
    let config = match config {
        Ok(config) => config,
        Err(error) => {
            let _ = connections.synchronize(&[]);
            tracing::warn!("MCP toolset: failed to load ACP config for dynamic tools: {error}");
            return;
        }
    };

    let enabled = config
        .mcp_servers
        .iter()
        .filter(|server| mcp_client::management_for_server(&config, server).enabled)
        .cloned()
        .collect::<Vec<_>>();
    if connections.synchronize(&enabled).is_err() {
        connections.record_required_failure("configuration");
        return;
    }

    let mut has_resources_or_prompts = false;
    for server in &config.mcp_servers {
        let server_label = mcp_client::server_name(server).to_string();
        let management = mcp_client::management_for_server(&config, server);
        if !management.enabled {
            tracing::debug!(
                server = server_label.as_str(),
                "MCP toolset: skipped disabled MCP server"
            );
            continue;
        }
        let existing_ids = tool_arcs.iter().map(|tool| tool.spec().id).collect::<Vec<_>>();
        let existing_refs = existing_ids.iter().map(String::as_str).collect::<Vec<_>>();

        match discover_dynamic_mcp_tools::<H>(
            server.clone(),
            management.clone(),
            &existing_refs,
            connections.clone(),
        ).await {
            Ok(discovered) => {
                connections.register_tool_source(server.clone(), management.clone());
                has_resources_or_prompts |= discovered.has_resources_or_prompts;
                for tool in discovered.tools {
                    tracing::debug!(
                        server = server_label.as_str(),
                        tool_id = tool.name(),
                        "MCP toolset: registered dynamic tool"
                    );
                    tool_arcs.push(Arc::new(tool));
                }
            }
            Err(error) => {
                if management.required
                    || plugin_mcp::mcp_connection_settings::settings(server)
                        .is_ok_and(|settings| settings.required)
                {
                    connections.record_required_failure(&server_label);
                    tracing::error!(
                        server = server_label.as_str(),
                        "MCP toolset: required server failed dynamic tool discovery: {error}"
                    );
                } else {
                    tracing::warn!(
                        server = server_label.as_str(),
                        "MCP toolset: failed to discover dynamic tools: {error}"
                    );
                }
            }
        }
    }
    if has_resources_or_prompts {
        tool_arcs.push(Arc::new(tool_mcp::mcp_resource::McpResourceTool::<H>::new(config, connections)));
    }
}

pub struct DiscoveredMcpTools<H: McpToolHost> {
    pub tools: Vec<tool_mcp::mcp_tool::McpTool<H>>,
    pub has_resources_or_prompts: bool,
}

pub async fn discover_dynamic_mcp_tools<H: McpToolHost<Connections = McpSessionConnections>>(
    server: agent_client_protocol::McpServer,
    management: acp_config::McpServerManagement,
    existing_tool_ids: &[&str],
    connections: Arc<McpSessionConnections>,
) -> anyhow::Result<DiscoveredMcpTools<H>> {
    let server_label = mcp_client::server_name(&server).to_string();
    let existing_tool_ids =
        existing_tool_ids.iter().map(|id| (*id).to_string()).collect::<Vec<_>>();
    let configured_server = server.clone();

    let timeout = mcp_client::management_timeout(&management);

    let tool_connections = connections.clone();
    connections
        .with_client(&server, timeout, move |client| {
            let server_label = server_label.clone();
            let existing_tool_ids = existing_tool_ids.clone();
            let configured_server = configured_server.clone();
            let management = management.clone();
            Box::pin(async move {
                let peer = client.peer();
                let has_resources_or_prompts = peer.peer_info().is_some_and(|info| {
                    info.capabilities.resources.is_some() || info.capabilities.prompts.is_some()
                });
                if peer.peer_info().is_some_and(|info| info.capabilities.tools.is_none()) {
                    return Ok(DiscoveredMcpTools { tools: Vec::new(), has_resources_or_prompts });
                }
                let tools = client
                    .service()
                    .catalogs
                    .tools(peer)
                    .await
                    .context("failed to list MCP tools")?;
                let server_instructions =
                    peer.peer_info().and_then(|info| info.instructions.as_deref());
                let mut allowed_tools = Vec::new();
                for tool in tools.iter().cloned() {
                    let tool_id = tool_mcp::mcp_tool::mcp_tool_id(&server_label, tool.name.as_ref())?;
                    if mcp_client::mcp_tool_allowed(
                        &management,
                        tool.name.as_ref(),
                        &tool_id,
                    ) && plugin_mcp::mcp_connection_settings::tool_allowed(
                        &configured_server,
                        tool.name.as_ref(),
                    )? {
                        allowed_tools.push(tool);
                    }
                }
                let existing_refs =
                    existing_tool_ids.iter().map(String::as_str).collect::<Vec<_>>();
                let mut specs = tool_mcp::mcp_tool::mcp_tools_to_specs(
                    &server_label,
                    server_instructions,
                    &allowed_tools,
                    &existing_refs,
                )?;

                if plugin_mcp::mcp_connection_settings::settings(&configured_server)?
                    .supports_parallel_tool_calls
                    == Some(false)
                {
                    for spec in &mut specs {
                        spec.concurrency_safe = false;
                    }
                }
                let tools = allowed_tools
                    .into_iter()
                    .zip(specs.into_iter())
                    .map(|(tool, spec)| {
                        tool_mcp::mcp_tool::McpTool::<H>::from_session_server(
                            configured_server.clone(),
                            tool.name.to_string(),
                            spec,
                            tool_connections.clone(),
                            mcp_client::management_timeout(&management),
                        )
                    })
                    .collect();
                Ok(DiscoveredMcpTools { tools, has_resources_or_prompts })
            })
        })
        .await
}

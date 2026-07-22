//! MCP server inspection helpers shared by gateway and tools.

use super::mcp_common::{
    management_for_server, management_timeout, mcp_tool_allowed, server_name, server_transport,
    with_server_peer_timeout,
};
use anyhow::Context;
use serde_json::Value;
use vw_api_types::mcp::{McpConnectionStatusDto, McpInspectResponse, McpServerInspectDto};
use vw_api_types::tools::ToolSpecDto;

#[derive(Debug, Default)]
struct ServerInspectPayload {
    tools: Vec<ToolSpecDto>,
    resources: Vec<Value>,
    resource_templates: Vec<Value>,
    prompts: Vec<Value>,
    tools_error: Option<String>,
    resources_error: Option<String>,
    resource_templates_error: Option<String>,
    prompts_error: Option<String>,
}

pub async fn inspect_mcp_servers(config: &vw_acp::ResolvedAcpxConfig) -> McpInspectResponse {
    let mut servers = Vec::with_capacity(config.mcp_servers.len());

    for server in &config.mcp_servers {
        let name = server_name(server).to_string();
        let management = management_for_server(config, server);
        let mut entry = McpServerInspectDto {
            name: name.clone(),
            transport: server_transport(server).to_string(),
            enabled: management.enabled,
            required: management.required,
            connection_status: McpConnectionStatusDto::Disabled,
            timeout_ms: management.timeout_ms,
            allowed_tools: management.allowed_tools.clone(),
            denied_tools: management.denied_tools.clone(),
            tool_count: None,
            resource_count: None,
            resource_template_count: None,
            prompt_count: None,
            tools: Vec::new(),
            resources: Vec::new(),
            resource_templates: Vec::new(),
            prompts: Vec::new(),
            tools_error: None,
            resources_error: None,
            resource_templates_error: None,
            prompts_error: None,
            error: None,
        };

        if management.enabled {
            match inspect_server(server, &management).await {
                Ok(payload) => {
                    entry.connection_status = McpConnectionStatusDto::Connected;
                    entry.tool_count = Some(payload.tools.len());
                    entry.resource_count = Some(payload.resources.len());
                    entry.resource_template_count = Some(payload.resource_templates.len());
                    entry.prompt_count = Some(payload.prompts.len());
                    entry.tools = payload.tools;
                    entry.resources = payload.resources;
                    entry.resource_templates = payload.resource_templates;
                    entry.prompts = payload.prompts;
                    entry.tools_error = payload.tools_error;
                    entry.resources_error = payload.resources_error;
                    entry.resource_templates_error = payload.resource_templates_error;
                    entry.prompts_error = payload.prompts_error;
                }
                Err(error) => {
                    entry.connection_status = McpConnectionStatusDto::Error;
                    entry.error = Some(summarize_error(&error));
                }
            }
        }

        servers.push(entry);
    }

    McpInspectResponse { servers }
}

pub async fn inspect_mcp_servers_from_workspace(
    workspace_dir: impl AsRef<std::path::Path>,
) -> anyhow::Result<McpInspectResponse> {
    let config = super::mcp_common::load_resolved_acp_config_from(workspace_dir).await?;
    Ok(inspect_mcp_servers(&config).await)
}

async fn inspect_server(
    server: &agent_client_protocol::McpServer,
    management: &vw_acp::McpServerManagement,
) -> anyhow::Result<ServerInspectPayload> {
    let server_label = server_name(server).to_string();
    let management = management.clone();

    with_server_peer_timeout(server, management_timeout(&management), move |peer| {
        let server_label = server_label.clone();
        let management = management.clone();
        Box::pin(async move {
            let mut payload = ServerInspectPayload::default();
            match peer.list_all_tools().await.context("failed to list MCP tools") {
                Ok(tools) => {
                    let mut allowed_tools = Vec::new();
                    for tool in tools {
                        let tool_id =
                            super::mcp_tool::mcp_tool_id(&server_label, tool.name.as_ref())?;
                        if mcp_tool_allowed(&management, tool.name.as_ref(), &tool_id) {
                            allowed_tools.push(tool);
                        }
                    }

                    let server_instructions =
                        peer.peer_info().and_then(|info| info.instructions.as_deref());
                    let specs = super::mcp_tool::mcp_tools_to_specs(
                        &server_label,
                        server_instructions,
                        &allowed_tools,
                        &[],
                    )?;
                    payload.tools = specs.into_iter().map(|spec| spec.to_dto()).collect();
                }
                Err(error) => payload.tools_error = Some(summarize_error(&error)),
            }

            match peer.list_all_resources().await.context("failed to list MCP resources") {
                Ok(resources) => match serde_json::to_value(resources) {
                    Ok(Value::Array(resources)) => payload.resources = resources,
                    Ok(_) => {
                        payload.resources_error =
                            Some("failed to serialize MCP resources: expected array".to_string())
                    }
                    Err(error) => {
                        payload.resources_error =
                            Some(format!("failed to serialize MCP resources: {error}"))
                    }
                },
                Err(error) => payload.resources_error = Some(summarize_error(&error)),
            }

            match peer
                .list_all_resource_templates()
                .await
                .context("failed to list MCP resource templates")
            {
                Ok(resource_templates) => match serde_json::to_value(resource_templates) {
                    Ok(Value::Array(resource_templates)) => {
                        payload.resource_templates = resource_templates;
                    }
                    Ok(_) => {
                        payload.resource_templates_error = Some(
                            "failed to serialize MCP resource templates: expected array"
                                .to_string(),
                        );
                    }
                    Err(error) => {
                        payload.resource_templates_error =
                            Some(format!("failed to serialize MCP resource templates: {error}"));
                    }
                },
                Err(error) => payload.resource_templates_error = Some(summarize_error(&error)),
            }

            match peer.list_all_prompts().await.context("failed to list MCP prompts") {
                Ok(prompts) => match serde_json::to_value(prompts) {
                    Ok(Value::Array(prompts)) => payload.prompts = prompts,
                    Ok(_) => {
                        payload.prompts_error =
                            Some("failed to serialize MCP prompts: expected array".to_string())
                    }
                    Err(error) => {
                        payload.prompts_error =
                            Some(format!("failed to serialize MCP prompts: {error}"))
                    }
                },
                Err(error) => payload.prompts_error = Some(summarize_error(&error)),
            }

            Ok(payload)
        })
    })
    .await
}

fn summarize_error(error: &anyhow::Error) -> String {
    error.to_string()
}

#[cfg(test)]
#[path = "mcp_inspect_tests.rs"]
mod mcp_inspect_tests;

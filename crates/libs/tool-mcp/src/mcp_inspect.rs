//! 网关与工具共用的 MCP 服务器检查逻辑。
//!
//! 各类能力独立探测并分别记录错误，避免单项失败遮蔽其余可用信息。

use crate::McpToolHost;
use mcp_client::{
    management_for_server, management_timeout, mcp_tool_allowed, server_name, server_transport,
};
use anyhow::Context;
use rmcp::{ServiceError, model::ErrorCode};
use serde_json::Value;
use vw_api_mcp_inspect::{McpConnectionStatusDto, McpInspectResponse, McpServerInspectDto};
use vw_api_tools::ToolSpecDto;

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

/// 检查已解析配置中的全部 MCP 服务器并汇总各能力状态。
///
/// 单项探测失败只记录在对应服务器字段中，不中断其他服务器或能力检查。
pub async fn inspect_mcp_servers<H: McpToolHost>(config: &acp_config::ResolvedAcpxConfig) -> McpInspectResponse {
    let mut servers = Vec::with_capacity(config.mcp_servers.len());

    for server in &config.mcp_servers {
        let name = server_name(server).to_string();
        let management = management_for_server(config, server);
        let mut entry = McpServerInspectDto {
            name: name.clone(),
            transport: server_transport(server).to_string(),
            enabled: management.enabled,
            required: management.required
                || H::connection_required(server).unwrap_or(false),
            connection_status: McpConnectionStatusDto::Disabled,
            workspace_directory: None,
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
            match H::missing_executable(server) {
                Ok(Some((dependency, detail))) => {
                    entry.connection_status = McpConnectionStatusDto::Error;
                    entry.error = Some(format!("{}: {}", dependency, detail));
                    servers.push(entry);
                    continue;
                }
                Err(_) => {
                    entry.connection_status = McpConnectionStatusDto::Error;
                    entry.error = Some("plugin MCP dependency context is invalid".into());
                    servers.push(entry);
                    continue;
                }
                Ok(None) => {}
            }
            match inspect_server::<H>(server, &management).await {
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

/// 从工作区解析 MCP 配置后执行检查。
pub async fn inspect_mcp_servers_from_workspace<H: McpToolHost>(
    workspace_dir: impl AsRef<std::path::Path>,
) -> anyhow::Result<McpInspectResponse> {
    let config = H::load_config(Some(workspace_dir.as_ref())).await?;
    Ok(inspect_mcp_servers::<H>(&config).await)
}

async fn inspect_server<H: McpToolHost>(
    server: &agent_client_protocol::McpServer,
    management: &acp_config::McpServerManagement,
) -> anyhow::Result<ServerInspectPayload> {
    let configured_server = server.clone();
    let server_label = server_name(server).to_string();
    let management = management.clone();

    H::with_client(server, management_timeout(&management), move |client| {
        let server_label = server_label.clone();
        let configured_server = configured_server.clone();
        let management = management.clone();
        Box::pin(async move {
            let peer = client.peer();
            let catalogs = &client.service().catalogs;
            let info = peer.peer_info().context("MCP server capabilities unavailable")?;
            let mut payload = ServerInspectPayload::default();
            match catalogs.tools(peer).await.context("failed to list MCP tools") {
                Ok(tools) => {
                    let mut allowed_tools = Vec::new();
                    for tool in tools.iter().cloned() {
                        let tool_id =
                            super::mcp_tool::mcp_tool_id(&server_label, tool.name.as_ref())?;
                        if mcp_tool_allowed(&management, tool.name.as_ref(), &tool_id)
                            && H::tool_allowed(
                                &configured_server,
                                tool.name.as_ref(),
                            )?
                        {
                            allowed_tools.push(tool);
                        }
                    }

                    let specs = super::mcp_tool::mcp_tools_to_specs(
                        &server_label,
                        info.instructions.as_deref(),
                        &allowed_tools,
                        &[],
                    )?;
                    payload.tools = specs.into_iter().map(|spec| spec.to_dto()).collect();
                }
                Err(error) => payload.tools_error = Some(summarize_error(&error)),
            }

            if let Some(resources) = &info.capabilities.resources {
                let cacheable = resources.list_changed == Some(true);
                match catalogs
                    .resources(cacheable, || async {
                        peer.list_all_resources()
                            .await?
                            .into_iter()
                            .map(|item| serde_json::to_value(item).map_err(Into::into))
                            .collect()
                    })
                    .await
                    .context("failed to list MCP resources")
                {
                    Ok(resources) => payload.resources = resources.as_ref().clone(),
                    Err(error) => payload.resources_error = Some(summarize_error(&error)),
                }

                match catalogs
                    .templates(cacheable, || async {
                        let templates = match peer.list_all_resource_templates().await {
                            // Some servers advertise resources but omit the template list method.
                            Err(ServiceError::McpError(error))
                                if error.code == ErrorCode::METHOD_NOT_FOUND =>
                            {
                                return Ok(Vec::new());
                            }
                            result => result?,
                        };
                        templates
                            .into_iter()
                            .map(|item| serde_json::to_value(item).map_err(Into::into))
                            .collect()
                    })
                    .await
                    .context("failed to list MCP resource templates")
                {
                    Ok(templates) => payload.resource_templates = templates.as_ref().clone(),
                    Err(error) => payload.resource_templates_error = Some(summarize_error(&error)),
                }
            }

            if let Some(prompts) = &info.capabilities.prompts {
                match catalogs
                    .prompts(prompts.list_changed == Some(true), || async {
                        peer.list_all_prompts()
                            .await?
                            .into_iter()
                            .map(|item| serde_json::to_value(item).map_err(Into::into))
                            .collect()
                    })
                    .await
                    .context("failed to list MCP prompts")
                {
                    Ok(prompts) => payload.prompts = prompts.as_ref().clone(),
                    Err(error) => payload.prompts_error = Some(summarize_error(&error)),
                }
            }

            Ok(payload)
        })
    })
    .await
}

#[doc(hidden)]
pub fn summarize_error(error: &anyhow::Error) -> String {
    error.to_string()
}

#[cfg(test)]
#[path = "mcp_inspect_error_tests.rs"]
mod mcp_inspect_error_tests;

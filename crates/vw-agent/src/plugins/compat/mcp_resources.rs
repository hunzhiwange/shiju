//! Raw resource/prompt responses and their session-owned protocol operations.
use crate::tools::{
    ToolCallResult,
    mcp_common::{
        RunningMcpPeer, find_server_by_name, management_for_server, management_timeout, server_name,
    },
};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};

pub use mcp_client::protocol::RawResponse;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    server: String,
    action: String,
    uri: Option<String>,
    name: Option<String>,
    arguments: Option<std::collections::BTreeMap<String, String>>,
}
pub use plugin_mcp::mcp_resources::{request, validate_contents, list_entries, meta, string, uri};

fn validate_prompt(value: &Value) -> Result<()> {
    meta(value)?;
    if value.get("description").is_some_and(|v| !v.is_null() && !v.is_string()) {
        bail!("MCP prompt description must be a string");
    }
    for message in value
        .get("messages")
        .and_then(Value::as_array)
        .context("MCP prompt messages must be an array")?
    {
        if !matches!(message.get("role").and_then(Value::as_str), Some("user" | "assistant")) {
            bail!("MCP prompt role must be user or assistant");
        }
        meta(message)?;
        super::mcp_content::validate_content(
            message.get("content").context("MCP prompt content is required")?,
        )?;
    }
    Ok(())
}
fn visible(mut value: Value) -> Value {
    match &mut value {
        Value::Object(object) => {
            object.remove("_meta");
            for v in object.values_mut() {
                *v = visible(v.take());
            }
        }
        Value::Array(array) => {
            for v in array {
                *v = visible(v.take());
            }
        }
        _ => {}
    }
    value
}
pub(crate) async fn call(
    config: &acp_config::ResolvedAcpxConfig,
    connections: &super::mcp_connections::McpSessionConnections,
    input: Value,
) -> Result<ToolCallResult> {
    for key in ["uri", "name"] {
        if input.get(key).is_some_and(|value| !value.is_string()) {
            bail!("invalid MCP resource string argument");
        }
    }
    if input.get("arguments").is_some_and(|value| !value.is_object()) {
        bail!("MCP prompt arguments must be an object");
    }
    let args: Args = serde_json::from_value(input)
        .map_err(|_| anyhow::anyhow!("invalid MCP resource arguments"))?;
    let method = match args.action.as_str() {
        "list_resources" => "resources/list",
        "list_resource_templates" => "resources/templates/list",
        "read_resource" => "resources/read",
        "list_prompts" => "prompts/list",
        "get_prompt" => "prompts/get",
        "subscribe" | "unsubscribe" => bail!("MCP resource subscriptions are not supported"),
        _ => bail!("unsupported MCP resource action"),
    };
    match method {
        "resources/read" => {
            uri(args.uri.as_deref().context("MCP resource URI is required")?)?;
            if args.name.is_some() || args.arguments.is_some() {
                bail!("resource reads do not accept prompt arguments");
            }
        }
        "prompts/get" => {
            if args.name.as_deref().is_none_or(|name| name.trim().is_empty()) {
                bail!("MCP prompt name is required");
            }
            if args.uri.is_some() {
                bail!("prompt requests do not accept resource URIs");
            }
        }
        _ => {
            if args.uri.is_some() || args.name.is_some() || args.arguments.is_some() {
                bail!("catalog requests do not accept read/get parameters");
            }
        }
    }
    let server = find_server_by_name(&config.mcp_servers, &args.server)?;
    let management = management_for_server(config, server);
    if !management.enabled {
        bail!("MCP server is disabled");
    }
    let source = super::mcp_connection_settings::settings(server)?.source_root;
    let name = server_name(server).to_owned();
    let raw = connections
        .with_client(server, management_timeout(&management), move |client| {
            Box::pin(async move {
                let capabilities = &client
                    .peer()
                    .peer_info()
                    .context("MCP peer was not initialized")?
                    .capabilities;
                if method.starts_with("resources/") && capabilities.resources.is_none() {
                    bail!("MCP server does not advertise resources");
                }
                if method.starts_with("prompts/") && capabilities.prompts.is_none() {
                    bail!("MCP server does not advertise prompts");
                }
                match method {
                    "resources/list" => Ok(json!({"resources":client.service().catalogs.resources(capabilities.resources.as_ref().and_then(|c| c.list_changed) == Some(true), || list_entries(client, "resources/list", "resources")).await?.as_ref()})),
                    "resources/templates/list" => Ok(json!({"resourceTemplates":client.service().catalogs.templates(capabilities.resources.as_ref().and_then(|c| c.list_changed) == Some(true), || list_entries(client, "resources/templates/list", "resourceTemplates")).await?.as_ref()})),
                    "prompts/list" => Ok(json!({"prompts":client.service().catalogs.prompts(capabilities.prompts.as_ref().and_then(|c| c.list_changed) == Some(true), || list_entries(client, "prompts/list", "prompts")).await?.as_ref()})),
                    "resources/read" => {
                        let result = request(client, method, json!({"uri":args.uri})).await?;
                        validate_contents(&result)?;
                        Ok(result)
                    }
                    "prompts/get" => {
                        let mut params = json!({"name":args.name});
                        if let Some(arguments) = args.arguments {
                            params["arguments"] = json!(arguments);
                        }
                        let result = request(client, method, params).await?;
                        validate_prompt(&result)?;
                        Ok(result)
                    }
                    _ => unreachable!(),
                }
            })
        })
        .await?;
    Ok(ToolCallResult {
        data: json!({"server":name,"source_root":source,"mcp_result":raw}),
        model_result: json!({"server":name,"result":visible(raw)}),
        ..Default::default()
    })
}

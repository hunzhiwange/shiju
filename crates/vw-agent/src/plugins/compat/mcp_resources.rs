//! Raw resource/prompt responses and their session-owned protocol operations.
use crate::tools::{
    ToolCallResult,
    mcp_common::{
        RunningMcpPeer, find_server_by_name, management_for_server, management_timeout, server_name,
    },
};
use anyhow::{Context, Result, bail};
use rmcp::{
    model::{ClientRequest, RequestId},
    service::PeerRequestOptions,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Mutex;

/// A single response slot is sufficient: McpSessionConnections serializes each peer operation.
/// The response id must match; unsolicited/overlapping responses fail closed, never cross-route.
#[derive(Default)]
pub(crate) struct RawResponse(Mutex<Option<(Value, Value)>>);
impl RawResponse {
    pub(crate) fn capture(&self, message: &Value) {
        if message.get("method").is_some() {
            return;
        }
        if let (Some(id), Some(result)) = (message.get("id"), message.get("result")) {
            if ["resources", "resourceTemplates", "contents", "prompts", "messages"]
                .iter()
                .any(|key| result.get(key).is_some())
            {
                *self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner) =
                    Some((id.clone(), result.clone()));
            }
        }
    }
    fn take(&self, id: &RequestId) -> Result<Value> {
        let expected = serde_json::to_value(id)?;
        match self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take() {
            Some((actual, result)) if actual == expected => Ok(result),
            _ => bail!("MCP resource response is missing or does not match its request"),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    server: String,
    action: String,
    uri: Option<String>,
    name: Option<String>,
    arguments: Option<std::collections::BTreeMap<String, String>>,
}
async fn request(client: &RunningMcpPeer, method: &str, params: Value) -> Result<Value> {
    let req: ClientRequest = serde_json::from_value(json!({"method":method,"params":params}))?;
    let handle = client
        .peer()
        .send_cancellable_request(req, PeerRequestOptions::no_options())
        .await
        .map_err(|_| anyhow::anyhow!("MCP resource request could not be sent"))?;
    let id = handle.id.clone();
    handle.await_response().await.map_err(|error| match error {
        rmcp::service::ServiceError::McpError(error) => {
            anyhow::anyhow!("MCP {method} failed with protocol error {}", error.code.0)
        }
        rmcp::service::ServiceError::TransportClosed => {
            anyhow::anyhow!("MCP {method} transport closed")
        }
        rmcp::service::ServiceError::Cancelled { .. } => {
            anyhow::anyhow!("MCP {method} was cancelled")
        }
        rmcp::service::ServiceError::Timeout { .. } => anyhow::anyhow!("MCP {method} timed out"),
        _ => anyhow::anyhow!("MCP {method} request failed"),
    })?;
    client.service().raw_responses.take(&id)
}
fn meta(value: &Value) -> Result<()> {
    if value.get("_meta").is_some_and(|v| !v.is_null() && !v.is_object()) {
        bail!("MCP resource metadata must be an object");
    }
    Ok(())
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .with_context(|| format!("MCP resource field {key} must be a string"))
}
fn uri(value: &str) -> Result<()> {
    if value.is_empty() || value.chars().any(char::is_control) || url::Url::parse(value).is_err() {
        bail!("MCP resource URI must be an absolute URI");
    }
    Ok(())
}
fn validate_contents(value: &Value) -> Result<()> {
    let contents = value
        .get("contents")
        .and_then(Value::as_array)
        .context("MCP resource contents must be an array")?;
    for entry in contents {
        meta(entry)?;
        uri(string(entry, "uri")?)?;
        if entry.get("mimeType").is_some_and(|v| !v.is_null() && !v.is_string()) {
            bail!("MCP resource MIME must be a string");
        }
        match (entry.get("text"), entry.get("blob")) {
            (Some(Value::String(_)), None) => {}
            (None, Some(Value::String(blob))) => {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD
                    .decode(blob)
                    .map_err(|_| anyhow::anyhow!("MCP resource blob is not base64"))?;
            }
            _ => bail!("MCP resource requires exactly one text or blob payload"),
        }
    }
    meta(value)
}
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
pub(super) async fn list_entries(
    client: &RunningMcpPeer,
    method: &str,
    field: &str,
) -> Result<Vec<Value>> {
    super::mcp_catalog::collect(method, |cursor| async move {
        let result = request(client, method, json!({"cursor":cursor})).await?;
        meta(&result)?;
        let entries = result
            .get(field)
            .and_then(Value::as_array)
            .context("MCP catalog must be an array")?
            .clone();
        for entry in &entries {
            meta(entry)?;
            string(entry, "name")?;
            match field {
                "resources" => uri(string(entry, "uri")?)?,
                "resourceTemplates" => {
                    string(entry, "uriTemplate")?;
                }
                "prompts" => {
                    if let Some(args) = entry.get("arguments").filter(|v| !v.is_null()) {
                        for arg in
                            args.as_array().context("MCP prompt arguments must be an array")?
                        {
                            string(arg, "name")?;
                            if arg.get("required").is_some_and(|v| !v.is_null() && !v.is_boolean())
                            {
                                bail!("MCP prompt required must be boolean");
                            }
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        let cursor = match result.get("nextCursor") {
            None | Some(Value::Null) => None,
            Some(Value::String(v)) => Some(v.clone()),
            _ => bail!("MCP cursor must be a string"),
        };
        Ok((entries, cursor))
    })
    .await
}
pub(crate) async fn call(
    config: &vw_acp::ResolvedAcpxConfig,
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
                    "resources/list" => Ok(json!({"resources":client.service().catalogs.resources(client).await?.as_ref()})),
                    "resources/templates/list" => Ok(json!({"resourceTemplates":client.service().catalogs.templates(client).await?.as_ref()})),
                    "prompts/list" => Ok(json!({"prompts":client.service().catalogs.prompts(client).await?.as_ref()})),
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

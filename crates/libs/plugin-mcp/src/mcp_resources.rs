//! Resource protocol operations; session admission remains caller-owned.
use mcp_client::RunningMcpPeer;
use anyhow::{Context, Result, bail};
use rmcp::{model::ClientRequest, service::PeerRequestOptions};
use serde_json::{Value, json};

pub async fn request(client: &RunningMcpPeer, method: &str, params: Value) -> Result<Value> {
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
pub fn meta(value: &Value) -> Result<()> {
    if value.get("_meta").is_some_and(|v| !v.is_null() && !v.is_object()) {
        bail!("MCP resource metadata must be an object");
    }
    Ok(())
}
pub fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .with_context(|| format!("MCP resource field {key} must be a string"))
}
pub fn uri(value: &str) -> Result<()> {
    if value.is_empty() || value.chars().any(char::is_control) || url::Url::parse(value).is_err() {
        bail!("MCP resource URI must be an absolute URI");
    }
    Ok(())
}
pub fn validate_contents(value: &Value) -> Result<()> {
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
pub async fn list_entries(
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

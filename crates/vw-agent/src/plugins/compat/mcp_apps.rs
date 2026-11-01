//! Public MCP Apps associations are host data, never rendering authority.
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub(crate) const BINDING: &str = "vibewindow/internal-app-binding";

pub(crate) fn association(server: &str, tool: &str, meta: Option<&Value>) -> Result<Option<Value>> {
    let Some(meta) = meta.filter(|value| !value.is_null()) else { return Ok(None) };
    let object =
        meta.as_object().ok_or_else(|| anyhow::anyhow!("MCP tool metadata must be an object"))?;
    let modern = match object.get("ui") {
        Some(ui) => ui
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("MCP tool ui metadata must be an object"))?
            .get("resourceUri"),
        None => None,
    };
    let legacy = object.get("ui/resourceUri");
    if modern.is_some() && legacy.is_some() && modern != legacy {
        bail!("MCP tool UI resource declarations conflict");
    }
    let Some(uri) = modern.or(legacy) else { return Ok(None) };
    let uri =
        uri.as_str().ok_or_else(|| anyhow::anyhow!("MCP UI resource URI must be a string"))?;
    if !uri.starts_with("ui://")
        || uri.len() <= 5
        || uri.len() > 4096
        || uri.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        bail!("MCP UI resource requires a nonempty ui:// URI");
    }
    Ok(Some(
        json!({"server":server,"tool":tool,"resource_uri":uri,"tool_meta":meta,"renderable":false}),
    ))
}

/// Read through the same serialized connection; metadata is retained, never enforced here.
pub(crate) async fn resource(
    client: &crate::tools::mcp_common::RunningMcpPeer,
    binding: &mut Value,
) -> Result<()> {
    let uri =
        binding["resource_uri"].as_str().ok_or_else(|| anyhow::anyhow!("missing MCP UI URI"))?;
    let raw = super::mcp_resources::request(client, "resources/read", json!({"uri":uri})).await?;
    validate_resource(uri, &raw)?;
    binding["resource"] = raw;
    Ok(())
}

pub(super) fn validate_resource(uri: &str, raw: &Value) -> Result<()> {
    super::mcp_resources::validate_contents(raw)?;
    let contents =
        raw["contents"].as_array().ok_or_else(|| anyhow::anyhow!("missing MCP UI contents"))?;
    if contents.len() != 1
        || contents[0]["uri"] != uri
        || contents[0]["mimeType"] != "text/html;profile=mcp-app"
    {
        bail!("MCP UI resource URI or MIME does not match its declaration");
    }
    Ok(())
}

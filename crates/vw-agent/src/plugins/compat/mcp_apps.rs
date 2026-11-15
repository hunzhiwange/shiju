//! Public MCP Apps resource access through the host connection.
use anyhow::{Result, bail};
use serde_json::{Value, json};
pub use plugin_mcp::mcp_apps::{BINDING, association};

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

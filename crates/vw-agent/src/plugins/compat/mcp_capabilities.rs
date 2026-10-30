//! Frozen initialize version and explicit limits of the native MCP client.
use anyhow::{Result, bail};
use rmcp::model::{ClientCapabilities, ClientInfo, ProtocolVersion, ServerInfo};

pub(crate) fn client_info() -> ClientInfo {
    ClientInfo {
        protocol_version: ProtocolVersion::V_2025_06_18,
        // No roots, sampling, form or UI handler is installed on this connection.
        // Plugin names and remote extension declarations cannot enable a host capability.
        capabilities: ClientCapabilities::default(),
        client_info: Default::default(),
    }
}

pub(crate) fn validate_server_protocol(info: Option<&ServerInfo>) -> Result<()> {
    let Some(info) = info else {
        bail!("MCP server did not provide negotiated protocol information");
    };
    if ![
        ProtocolVersion::V_2024_11_05,
        ProtocolVersion::V_2025_03_26,
        ProtocolVersion::V_2025_06_18,
    ]
    .contains(&info.protocol_version)
    {
        // The version is remote input: do not reflect arbitrary payloads into diagnostics.
        bail!("MCP server negotiated an unsupported protocol version");
    }
    Ok(())
}

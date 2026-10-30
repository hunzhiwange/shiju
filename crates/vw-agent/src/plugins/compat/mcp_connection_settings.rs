//! 已验证的生命周期声明；包只能声明设置，不能提供宿主连接或执行环境。
use super::mcp_legacy::{PluginMcpDeclaration, PluginMcpParseOutcome};
use agent_client_protocol::McpServer;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{collections::HashMap, path::PathBuf, time::Duration};

const KEY: &str = "shiju/plugin-connection-context";

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConnectionSettings {
    pub startup_timeout_ms: Option<u64>,
    pub tool_timeout_ms: Option<u64>,
    pub required: bool,
    pub catalog_readiness: bool,
    pub source_root: Option<PathBuf>,
}

fn positive_duration(value: Value, field: &str, seconds: bool) -> Result<u64> {
    let duration = if seconds {
        value.as_f64().and_then(|v| Duration::try_from_secs_f64(v).ok())
    } else {
        value.as_u64().map(Duration::from_millis)
    }
    .filter(|v| !v.is_zero() && v.as_millis() <= u64::MAX as u128)
    .with_context(|| format!("plugin MCP requires positive {field}"))?;
    let millis = duration.as_millis() as u64;
    if millis == 0 {
        bail!("plugin MCP requires positive {field} of at least one millisecond");
    }
    Ok(millis)
}

pub(crate) fn prepare(
    outcome: &mut PluginMcpParseOutcome,
    root: &std::path::Path,
) -> Result<HashMap<String, ConnectionSettings>> {
    let mut settings = HashMap::new();
    for declaration in &mut outcome.servers {
        match declaration {
            PluginMcpDeclaration::Legacy { name, config }
            | PluginMcpDeclaration::Portable { name, config } => {
                let mut entry = ConnectionSettings {
                    source_root: Some(root.canonicalize()?),
                    ..Default::default()
                };
                let seconds = config.remove("startup_timeout_sec").filter(|v| !v.is_null());
                let millis = config.remove("startup_timeout_ms").filter(|v| !v.is_null());
                entry.startup_timeout_ms = match (seconds, millis) {
                    (Some(v), _) => Some(positive_duration(v, "startup_timeout_sec", true)?),
                    (_, Some(v)) => Some(positive_duration(v, "startup_timeout_ms", false)?),
                    _ => None,
                };
                if let Some(v) = config.remove("tool_timeout_sec").filter(|v| !v.is_null()) {
                    entry.tool_timeout_ms = Some(positive_duration(v, "tool_timeout_sec", true)?);
                }
                if let Some(v) = config.remove("required").filter(|v| !v.is_null()) {
                    entry.required = v.as_bool().context("plugin MCP required must be boolean")?;
                }
                if let Some(v) = config.remove("startup_readiness").filter(|v| !v.is_null()) {
                    entry.catalog_readiness = match v.as_str() {
                        Some("connection") => false,
                        Some("catalog") => true,
                        _ => bail!("plugin MCP startup_readiness must be connection or catalog"),
                    };
                }
                if let Some(v) = config.remove("environment_id").filter(|v| !v.is_null())
                    && v.as_str() != Some("local")
                {
                    bail!("remote plugin MCP execution is not supported by local runtime");
                }
                settings.insert(name.clone(), entry);
            }
            PluginMcpDeclaration::Acp(server) => {
                if meta(server).is_some_and(|meta| meta.contains_key(KEY)) {
                    bail!("plugin cannot supply host connection context");
                }
                settings.insert(
                    crate::tools::mcp_common::server_name(server).to_owned(),
                    ConnectionSettings {
                        source_root: Some(root.canonicalize()?),
                        ..Default::default()
                    },
                );
            }
        }
    }
    Ok(settings)
}

fn meta(server: &McpServer) -> Option<&Map<String, Value>> {
    match server {
        McpServer::Stdio(server) => server.meta.as_ref(),
        McpServer::Http(server) => server.meta.as_ref(),
        McpServer::Sse(server) => server.meta.as_ref(),
        _ => None,
    }
}

pub(crate) fn bind(server: &mut McpServer, settings: ConnectionSettings) -> Result<()> {
    let meta = match server {
        McpServer::Stdio(server) => server.meta.get_or_insert_default(),
        McpServer::Http(server) => server.meta.get_or_insert_default(),
        McpServer::Sse(server) => server.meta.get_or_insert_default(),
        _ => bail!("unsupported plugin MCP transport"),
    };
    meta.insert(KEY.into(), serde_json::to_value(settings)?);
    Ok(())
}

pub(crate) fn settings(server: &McpServer) -> Result<ConnectionSettings> {
    let settings: ConnectionSettings = meta(server)
        .and_then(|m| m.get(KEY))
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .context("invalid plugin MCP connection settings")?
        .unwrap_or_default();
    if settings.startup_timeout_ms == Some(0) || settings.tool_timeout_ms == Some(0) {
        bail!("plugin MCP lifecycle timeouts must be positive");
    }
    Ok(settings)
}

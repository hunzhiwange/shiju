//! Merge global server declarations and explicit management overrides.

use std::collections::HashSet;
use serde_json::Value;
use crate::server_name;

pub fn merge_global_mcp_server_values(
    config: &mut acp_config::ResolvedAcpxConfig,
    raw_servers: &[Value],
    source_path: &str,
) -> anyhow::Result<()> {
    if raw_servers.is_empty() {
        return Ok(());
    }

    let parsed = acp_config::parse_mcp_server_configs(&Value::Array(raw_servers.to_vec()), source_path)
        .map_err(anyhow::Error::from)?;
    let mut merged_names =
        config.mcp_servers.iter().map(server_name).map(ToString::to_string).collect::<HashSet<_>>();

    for acp_config::McpServerConfig { server, management } in parsed {
        let name = server_name(&server);
        if !merged_names.insert(name.to_string()) {
            tracing::warn!(
                server = name,
                "skipping global MCP server because ACP config already defines it"
            );
            continue;
        }
        config.mcp_servers.push(server);
        config.mcp_server_management.push(management);
    }

    Ok(())
}

#[derive(Debug, Default)]
struct McpServerManagementOverride {
    name: String,
    enabled: Option<bool>,
    required: Option<bool>,
    timeout_ms: Option<Option<u64>>,
    allowed_tools: Option<Vec<String>>,
    denied_tools: Option<Vec<String>>,
}

fn parse_override_bool(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> anyhow::Result<Option<bool>> {
    match object.get(key) {
        None => Ok(None),
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| anyhow::anyhow!("Invalid {path}.{key}: expected boolean")),
    }
}

fn parse_override_timeout(
    object: &serde_json::Map<String, Value>,
    path: &str,
) -> anyhow::Result<Option<Option<u64>>> {
    let Some(value) = object.get("timeout") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(Some(None));
    }
    let Some(seconds) = value.as_f64() else {
        anyhow::bail!("Invalid {path}.timeout: expected positive seconds or null");
    };
    if !seconds.is_finite() || seconds <= 0.0 {
        anyhow::bail!("Invalid {path}.timeout: expected positive seconds or null");
    }
    Ok(Some(Some((seconds * 1_000.0).round() as u64)))
}

fn parse_override_tools(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> anyhow::Result<Option<Vec<String>>> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    let Some(array) = value.as_array() else {
        anyhow::bail!("Invalid {path}.{key}: expected array");
    };
    let mut tools = Vec::with_capacity(array.len());
    for (index, value) in array.iter().enumerate() {
        match value.as_str().map(str::trim).filter(|text| !text.is_empty()) {
            Some(tool) => tools.push(tool.to_string()),
            None => anyhow::bail!("Invalid {path}.{key}[{index}]: expected non-empty string"),
        }
    }
    Ok(Some(tools))
}

fn parse_mcp_server_management_override(
    value: &Value,
    source_path: &str,
    index: usize,
) -> anyhow::Result<McpServerManagementOverride> {
    let path = format!("mcpServerOverrides[{index}] in {source_path}");
    let object =
        value.as_object().ok_or_else(|| anyhow::anyhow!("Invalid {path}: expected object"))?;
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Invalid {path}.name: expected non-empty string"))?
        .to_string();

    Ok(McpServerManagementOverride {
        name,
        enabled: parse_override_bool(object, "enabled", &path)?,
        required: parse_override_bool(object, "required", &path)?,
        timeout_ms: parse_override_timeout(object, &path)?,
        allowed_tools: parse_override_tools(object, "allowedTools", &path)?,
        denied_tools: parse_override_tools(object, "deniedTools", &path)?,
    })
}

fn apply_mcp_management_override(
    management: &mut acp_config::McpServerManagement,
    override_entry: McpServerManagementOverride,
) {
    if let Some(enabled) = override_entry.enabled {
        management.enabled = enabled;
    }
    if let Some(required) = override_entry.required {
        management.required = required;
    }
    if let Some(timeout_ms) = override_entry.timeout_ms {
        management.timeout_ms = timeout_ms;
    }
    if let Some(allowed_tools) = override_entry.allowed_tools {
        management.allowed_tools = allowed_tools;
    }
    if let Some(denied_tools) = override_entry.denied_tools {
        management.denied_tools = denied_tools;
    }
}

pub fn apply_mcp_server_override_values(
    config: &mut acp_config::ResolvedAcpxConfig,
    raw_overrides: &[Value],
    source_path: &str,
) -> anyhow::Result<()> {
    if raw_overrides.is_empty() {
        return Ok(());
    }

    let server_names =
        config.mcp_servers.iter().map(server_name).map(ToString::to_string).collect::<HashSet<_>>();
    for (index, value) in raw_overrides.iter().enumerate() {
        let override_entry = parse_mcp_server_management_override(value, source_path, index)?;
        if !server_names.contains(&override_entry.name) {
            tracing::warn!(
                server = override_entry.name.as_str(),
                "skipping MCP server override because no runtime server has this name"
            );
            continue;
        }

        let mut management = config
            .mcp_server_management
            .iter()
            .find(|entry| entry.name == override_entry.name)
            .cloned()
            .unwrap_or_else(|| {
                acp_config::McpServerManagement::default_for_server(override_entry.name.clone())
            });
        apply_mcp_management_override(&mut management, override_entry);

        if let Some(existing) =
            config.mcp_server_management.iter_mut().find(|entry| entry.name == management.name)
        {
            *existing = management;
        } else {
            config.mcp_server_management.push(management);
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

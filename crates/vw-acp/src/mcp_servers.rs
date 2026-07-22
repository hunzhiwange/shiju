//! MCP 服务器配置字段的解析与校验。

use std::path::PathBuf;

use agent_client_protocol::{
    EnvVariable, HttpHeader, McpServer, McpServerHttp, McpServerSse, McpServerStdio, Meta,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const DEFAULT_MCP_SERVER_ENABLED: bool = true;
const DEFAULT_MCP_SERVER_REQUIRED: bool = false;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ParseMcpServersError {
    message: String,
}

impl ParseMcpServersError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerManagement {
    pub name: String,
    pub enabled: bool,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub denied_tools: Vec<String>,
}

impl McpServerManagement {
    pub fn default_for_server(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: DEFAULT_MCP_SERVER_ENABLED,
            required: DEFAULT_MCP_SERVER_REQUIRED,
            timeout_ms: None,
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerConfig {
    pub server: McpServer,
    pub management: McpServerManagement,
}

fn parse_non_empty_string(
    value: Option<&Value>,
    path: &str,
) -> Result<String, ParseMcpServersError> {
    match value.and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        _ => Err(ParseMcpServersError::new(format!("Invalid {path}: expected non-empty string"))),
    }
}

fn parse_bool(
    value: Option<&Value>,
    path: &str,
    default: bool,
) -> Result<bool, ParseMcpServersError> {
    match value {
        None => Ok(default),
        Some(value) => value
            .as_bool()
            .ok_or_else(|| ParseMcpServersError::new(format!("Invalid {path}: expected boolean"))),
    }
}

fn parse_timeout_ms(
    value: Option<&Value>,
    path: &str,
) -> Result<Option<u64>, ParseMcpServersError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(seconds) = value.as_f64() else {
        return Err(ParseMcpServersError::new(format!(
            "Invalid {path}: expected positive seconds or null"
        )));
    };
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err(ParseMcpServersError::new(format!(
            "Invalid {path}: expected positive seconds or null"
        )));
    }
    let timeout_ms = (seconds * 1_000.0).round() as u64;
    if timeout_ms == 0 {
        return Err(ParseMcpServersError::new(format!(
            "Invalid {path}: expected positive seconds or null"
        )));
    }
    Ok(Some(timeout_ms))
}

fn parse_tool_names(
    value: Option<&Value>,
    path: &str,
) -> Result<Vec<String>, ParseMcpServersError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    let tools = value
        .as_array()
        .ok_or_else(|| ParseMcpServersError::new(format!("Invalid {path}: expected array")))?;

    tools
        .iter()
        .enumerate()
        .map(|(index, raw_tool)| {
            parse_non_empty_string(Some(raw_tool), &format!("{path}[{index}]"))
        })
        .collect()
}

fn parse_headers(
    value: Option<&Value>,
    path: &str,
) -> Result<Vec<HttpHeader>, ParseMcpServersError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    let headers = value
        .as_array()
        .ok_or_else(|| ParseMcpServersError::new(format!("Invalid {path}: expected array")))?;

    headers
        .iter()
        .enumerate()
        .map(|(index, raw_header)| {
            let header = raw_header.as_object().ok_or_else(|| {
                ParseMcpServersError::new(format!("Invalid {path}[{index}]: expected object"))
            })?;
            let name =
                parse_non_empty_string(header.get("name"), &format!("{path}[{index}].name"))?;
            let value =
                parse_non_empty_string(header.get("value"), &format!("{path}[{index}].value"))?;
            Ok(HttpHeader::new(name, value))
        })
        .collect()
}

fn parse_args(value: Option<&Value>, path: &str) -> Result<Vec<String>, ParseMcpServersError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    let args = value
        .as_array()
        .ok_or_else(|| ParseMcpServersError::new(format!("Invalid {path}: expected array")))?;

    args.iter()
        .enumerate()
        .map(|(index, raw_arg)| {
            raw_arg.as_str().map(ToOwned::to_owned).ok_or_else(|| {
                ParseMcpServersError::new(format!("Invalid {path}[{index}]: expected string"))
            })
        })
        .collect()
}

fn parse_env(value: Option<&Value>, path: &str) -> Result<Vec<EnvVariable>, ParseMcpServersError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    let env = value
        .as_array()
        .ok_or_else(|| ParseMcpServersError::new(format!("Invalid {path}: expected array")))?;

    env.iter()
        .enumerate()
        .map(|(index, raw_entry)| {
            let entry = raw_entry.as_object().ok_or_else(|| {
                ParseMcpServersError::new(format!("Invalid {path}[{index}]: expected object"))
            })?;
            let name = parse_non_empty_string(entry.get("name"), &format!("{path}[{index}].name"))?;
            let value =
                parse_non_empty_string(entry.get("value"), &format!("{path}[{index}].value"))?;
            Ok(EnvVariable::new(name, value))
        })
        .collect()
}

fn parse_meta(value: Option<&Value>, path: &str) -> Result<Option<Meta>, ParseMcpServersError> {
    match value {
        None => Ok(None),
        Some(Value::Null) => Ok(None),
        Some(Value::Object(meta)) => Ok(Some(meta.clone())),
        Some(_) => {
            Err(ParseMcpServersError::new(format!("Invalid {path}: expected object or null")))
        }
    }
}

fn server_config_name(server: &McpServer) -> &str {
    match server {
        McpServer::Http(server) => &server.name,
        McpServer::Sse(server) => &server.name,
        McpServer::Stdio(server) => &server.name,
        _ => "unknown",
    }
}

fn parse_management(
    server_name: &str,
    raw_server: &serde_json::Map<String, Value>,
    path: &str,
) -> Result<McpServerManagement, ParseMcpServersError> {
    Ok(McpServerManagement {
        name: server_name.to_string(),
        enabled: parse_bool(
            raw_server.get("enabled"),
            &format!("{path}.enabled"),
            DEFAULT_MCP_SERVER_ENABLED,
        )?,
        required: parse_bool(
            raw_server.get("required"),
            &format!("{path}.required"),
            DEFAULT_MCP_SERVER_REQUIRED,
        )?,
        timeout_ms: parse_timeout_ms(raw_server.get("timeout"), &format!("{path}.timeout"))?,
        allowed_tools: parse_tool_names(
            raw_server.get("allowedTools"),
            &format!("{path}.allowedTools"),
        )?,
        denied_tools: parse_tool_names(
            raw_server.get("deniedTools"),
            &format!("{path}.deniedTools"),
        )?,
    })
}

fn parse_server(raw_server: &Value, path: &str) -> Result<McpServer, ParseMcpServersError> {
    let server = raw_server
        .as_object()
        .ok_or_else(|| ParseMcpServersError::new(format!("Invalid {path}: expected object")))?;

    let name = parse_non_empty_string(server.get("name"), &format!("{path}.name"))?;
    let meta = parse_meta(server.get("_meta"), &format!("{path}._meta"))?;
    let server_type = match server.get("type") {
        None => "stdio".to_string(),
        Some(value) => parse_non_empty_string(Some(value), &format!("{path}.type"))?,
    };

    match server_type.as_str() {
        "http" => {
            let url = parse_non_empty_string(server.get("url"), &format!("{path}.url"))?;
            let headers = parse_headers(server.get("headers"), &format!("{path}.headers"))?;
            let server = McpServerHttp::new(name, url).headers(headers);
            let server = match meta {
                Some(meta) => server.meta(meta),
                None => server,
            };
            Ok(McpServer::Http(server))
        }
        "sse" => {
            let url = parse_non_empty_string(server.get("url"), &format!("{path}.url"))?;
            let headers = parse_headers(server.get("headers"), &format!("{path}.headers"))?;
            let server = McpServerSse::new(name, url).headers(headers);
            let server = match meta {
                Some(meta) => server.meta(meta),
                None => server,
            };
            Ok(McpServer::Sse(server))
        }
        "stdio" => {
            let command =
                parse_non_empty_string(server.get("command"), &format!("{path}.command"))?;
            let args = parse_args(server.get("args"), &format!("{path}.args"))?;
            let env = parse_env(server.get("env"), &format!("{path}.env"))?;
            let server = McpServerStdio::new(name, PathBuf::from(command)).args(args).env(env);
            let server = match meta {
                Some(meta) => server.meta(meta),
                None => server,
            };
            Ok(McpServer::Stdio(server))
        }
        _ => Err(ParseMcpServersError::new(format!(
            "Invalid {path}.type: expected http, sse, or stdio"
        ))),
    }
}

fn parse_server_config(
    raw_server: &Value,
    path: &str,
) -> Result<McpServerConfig, ParseMcpServersError> {
    let server = parse_server(raw_server, path)?;
    let raw_server = raw_server
        .as_object()
        .ok_or_else(|| ParseMcpServersError::new(format!("Invalid {path}: expected object")))?;
    let management = parse_management(server_config_name(&server), raw_server, path)?;
    Ok(McpServerConfig { server, management })
}

pub fn parse_mcp_servers(
    value: &Value,
    source_path: &str,
) -> Result<Vec<McpServer>, ParseMcpServersError> {
    parse_mcp_servers_with_field_name(value, source_path, "mcpServers")
}

pub fn parse_mcp_servers_with_field_name(
    value: &Value,
    source_path: &str,
    field_name: &str,
) -> Result<Vec<McpServer>, ParseMcpServersError> {
    Ok(parse_mcp_server_configs_with_field_name(value, source_path, field_name)?
        .into_iter()
        .map(|config| config.server)
        .collect())
}

pub fn parse_mcp_server_configs(
    value: &Value,
    source_path: &str,
) -> Result<Vec<McpServerConfig>, ParseMcpServersError> {
    parse_mcp_server_configs_with_field_name(value, source_path, "mcpServers")
}

pub fn parse_mcp_server_configs_with_field_name(
    value: &Value,
    source_path: &str,
    field_name: &str,
) -> Result<Vec<McpServerConfig>, ParseMcpServersError> {
    let field_path = format!("{field_name} in {source_path}");
    let servers = value.as_array().ok_or_else(|| {
        ParseMcpServersError::new(format!("Invalid {field_path}: expected array"))
    })?;

    servers
        .iter()
        .enumerate()
        .map(|(index, raw_server)| {
            parse_server_config(raw_server, &format!("{field_name}[{index}] in {source_path}"))
        })
        .collect()
}

pub fn parse_optional_mcp_servers(
    value: Option<&Value>,
    source_path: &str,
) -> Result<Option<Vec<McpServer>>, ParseMcpServersError> {
    parse_optional_mcp_servers_with_field_name(value, source_path, "mcpServers")
}

pub fn parse_optional_mcp_servers_with_field_name(
    value: Option<&Value>,
    source_path: &str,
    field_name: &str,
) -> Result<Option<Vec<McpServer>>, ParseMcpServersError> {
    value.map(|value| parse_mcp_servers_with_field_name(value, source_path, field_name)).transpose()
}

#[cfg(test)]
#[path = "mcp_servers_tests.rs"]
mod mcp_servers_tests;

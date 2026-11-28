//! Edit the existing global configuration through the gateway and reuse catalog distribution.
use super::Draft;
use crate::configuration::{editor_value, loaded_mcp_servers, validate_mcp_servers};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use vw_api_mcp_management::{McpConfigValue, McpConnectionDefinition};
use vw_gateway_client::GatewayClient;

#[derive(Clone)]
pub struct GlobalDraft {
    pub original: Option<Value>,
    pub include_global: bool,
    pub enabled: bool,
    pub required: bool,
    pub timeout: String,
}

pub fn open(
    draft: &mut Draft,
    page: &crate::state::State,
    index: Option<usize>,
) -> Result<(), String> {
    let mut global = GlobalDraft {
        original: None,
        include_global: true,
        enabled: true,
        required: false,
        timeout: "30".into(),
    };
    if let Some(index) = index {
        let (servers, _) = editor_value(&page.editor).map_err(|error| error.to_string())?;
        let server = servers.get(index).ok_or("目录已变化，请刷新")?;
        let name = server["name"].as_str().ok_or("目录已变化，请刷新")?;
        let values = |key: &str| -> Result<BTreeMap<String, McpConfigValue>, String> {
            let mut values = BTreeMap::new();
            if let Some(entries) = server[key].as_array() {
                for entry in entries {
                    let name =
                        entry["name"].as_str().ok_or("连接定义无效，请检查字段及凭据引用")?;
                    let value =
                        entry["value"].as_str().ok_or("连接定义无效，请检查字段及凭据引用")?;
                    if values
                        .insert(name.into(), McpConfigValue::Literal { value: value.into() })
                        .is_some()
                    {
                        return Err("字段名已存在".into());
                    }
                }
            }
            Ok(values)
        };
        let url = || server["url"].as_str().unwrap_or_default().to_owned();
        let connection = match server["type"].as_str().unwrap_or("stdio") {
            "http" => {
                McpConnectionDefinition::StreamableHttp { url: url(), headers: values("headers")? }
            }
            "sse" => McpConnectionDefinition::Sse { url: url(), headers: values("headers")? },
            _ => McpConnectionDefinition::Stdio {
                command: server["command"].as_str().unwrap_or_default().to_owned(),
                args: server["args"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
                env: values("env")?,
                cwd: None,
            },
        };
        draft.id = name.into();
        draft.name = name.into();
        draft.set_connection(connection);
        global.original = Some(server.clone());
        global.enabled = server["enabled"].as_bool().unwrap_or(true);
        global.required = server["required"].as_bool().unwrap_or(false);
        global.timeout = server
            .get("timeout")
            .filter(|value| !value.is_null())
            .map(Value::to_string)
            .unwrap_or_default();
    }
    draft.global = Some(global);
    Ok(())
}

pub fn server(draft: &Draft, global: &GlobalDraft) -> Result<Value, String> {
    let name = draft.id.trim();
    if name.is_empty() {
        return Err("请填写服务器名称".into());
    }
    let mut server =
        global.original.clone().and_then(|value| value.as_object().cloned()).unwrap_or_default();
    for key in ["command", "args", "env", "url", "headers", "timeout"] {
        server.remove(key);
    }
    server.insert("name".into(), json!(name));
    server.insert("enabled".into(), json!(global.enabled));
    server.insert("required".into(), json!(global.required));
    if !global.timeout.trim().is_empty() {
        let timeout = global
            .timeout
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or("超时须为正数")?;
        server.insert("timeout".into(), json!(timeout));
    }
    let values = |values: BTreeMap<String, McpConfigValue>| -> Result<Value, String> {
        values
            .into_iter()
            .map(|(name, value)| match value {
                McpConfigValue::Literal { value } => Ok(json!({ "name": name, "value": value })),
                McpConfigValue::Credential { .. } => {
                    Err("凭据引用和工作目录仅支持保存到共享 MCP 目录".into())
                }
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array)
    };
    match draft.connection()? {
        McpConnectionDefinition::Stdio { command, args, env, cwd: None } => {
            server.insert("type".into(), json!("stdio"));
            server.insert("command".into(), json!(command));
            server.insert("args".into(), json!(args));
            server.insert("env".into(), values(env)?);
        }
        McpConnectionDefinition::Stdio { cwd: Some(_), .. } => {
            return Err("凭据引用和工作目录仅支持保存到共享 MCP 目录".into());
        }
        connection => {
            let (kind, url, headers) = match connection {
                McpConnectionDefinition::StreamableHttp { url, headers } => ("http", url, headers),
                McpConnectionDefinition::Sse { url, headers } => ("sse", url, headers),
                _ => unreachable!("stdio was handled"),
            };
            server.insert("type".into(), json!(kind));
            server.insert("url".into(), json!(url));
            server.insert("headers".into(), values(headers)?);
        }
    }
    let server = Value::Object(server);
    validate_mcp_servers(&json!([&server])).map_err(|error| error.to_string())?;
    Ok(server)
}

pub async fn persist(
    client: &GatewayClient,
    original: Option<&Value>,
    server: Option<&Value>,
) -> Result<(), String> {
    let client = gateway_config::GatewayConfigClient::new(client.transport().clone());
    let patch = match (original, server) {
        (Some(previous), Some(replacement)) => {
            json!({ "mcpServerToReplace": { "previous": previous, "replacement": replacement } })
        }
        (None, Some(server)) => json!({ "mcpServerToAdd": server }),
        (Some(original), None) => {
            // A row can come from an unsaved JSON draft. Publish only a persisted definition.
            let config =
                client.global_config_get().await.map_err(|_| "网关不可用，请刷新后重试")?;
            let servers = loaded_mcp_servers(config);
            let mut matches = servers.as_array().into_iter().flatten().filter(|item| {
                item["name"].as_str().map(str::trim) == original["name"].as_str().map(str::trim)
            });
            if matches.next() != Some(original) || matches.next().is_some() {
                return Err("全局配置已变化，请刷新后重试".into());
            }
            return Ok(());
        }
        (None, None) => return Ok(()),
    };
    client
        .global_config_patch(&patch)
        .await
        .map_err(|_| "全局 MCP 保存失败，请检查权限或刷新配置后重试".into())
}

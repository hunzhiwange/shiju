//! Import and export global MCP definitions without starting a connection.

use crate::configuration::{editor_value, pretty, validate_mcp_servers};
use crate::state::{McpSettingsTab, State};
use serde_json::{Map, Value, json};
use std::collections::HashSet;

pub const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone)]
pub enum Message {
    Import,
    Paste,
    Export,
    Imported(Result<Option<String>, String>),
    Exported(Result<bool, String>),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("McpConfigurationTransfer")
    }
}

pub fn backup(state: &State) -> Result<String, String> {
    let (servers, _) = editor_value(&state.editor).map_err(|error| error.to_string())?;
    let backup = pretty(&json!({ "mcpServers": servers })).map_err(|error| error.to_string())?;
    if backup.len() > MAX_FILE_BYTES {
        return Err("MCP JSON 超过 8 MiB".into());
    }
    Ok(backup)
}

/// Merge validated definitions into the editor. The normal Save action applies them.
pub fn stage_import(state: &mut State, source: &str) -> Result<(usize, usize), String> {
    let incoming = parse(source)?;
    validate_mcp_servers(&incoming).map_err(|error| error.to_string())?;
    let (mut current, _) = editor_value(&state.editor).map_err(|error| error.to_string())?;
    let servers = current.as_array_mut().expect("validated MCP array");
    let incoming = incoming.as_array().expect("validated MCP array");
    let mut names: HashSet<String> = servers.iter().map(|entry| name(entry).to_owned()).collect();
    let mut added = 0;
    let mut skipped = 0;
    for entry in incoming {
        if names.insert(name(entry).to_owned()) {
            servers.push(entry.clone());
            added += 1;
        } else {
            skipped += 1;
        }
    }
    if added > 0 {
        let count = servers.len();
        let text = pretty(&current).map_err(|error| error.to_string())?;
        state.editor = iced::widget::text_editor::Content::with_text(&text);
        state.server_count = count;
        state.active_tab = McpSettingsTab::Json;
        state.delete_pending = None;
        state.save_error = None;
        state.save_error_translation = None;
        state.status_message = None;
    }
    Ok((added, skipped))
}

fn name(entry: &Value) -> &str {
    entry["name"].as_str().expect("validated MCP name").trim()
}

fn parse(source: &str) -> Result<Value, String> {
    if source.len() > MAX_FILE_BYTES {
        return Err("MCP 配置超过 8 MiB".into());
    }
    let source = source.trim_start_matches('\u{feff}').trim();
    let root: Value = match serde_json::from_str(source) {
        Ok(root) => root,
        Err(_) => {
            let table: toml::Table =
                toml::from_str(source).map_err(|_| "无法解析 MCP JSON 或 Codex TOML")?;
            serde_json::to_value(table).map_err(|_| "无法转换 Codex TOML")?
        }
    };
    let entries = if root.is_array() {
        &root
    } else {
        root.get("mcpServers")
            .or_else(|| root.get("mcp_servers"))
            .or_else(|| root.pointer("/mcp/servers"))
            .ok_or("找不到 MCP 服务器列表")?
    };
    if entries.is_array() {
        return Ok(entries.clone());
    }
    let entries = entries.as_object().ok_or("MCP 服务器列表格式无效")?;
    entries
        .iter()
        .enumerate()
        .map(|(index, (name, entry))| {
            external_entry(name, entry).map_err(|error| format!("第 {} 项：{error}", index + 1))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)
}

fn external_entry(name: &str, entry: &Value) -> Result<Value, &'static str> {
    let entry = entry.as_object().ok_or("服务器定义必须为对象")?;
    let transport = match entry.get("type").and_then(Value::as_str) {
        Some("http" | "streamable_http" | "streamable-http") => "http",
        Some("sse") => "sse",
        Some("stdio") => "stdio",
        None if !entry.contains_key("type") && entry.contains_key("command") => "stdio",
        None if !entry.contains_key("type") && entry.contains_key("url") => "http",
        _ => return Err("不支持的 MCP 连接类型"),
    };
    let mut server = Map::new();
    for (key, value) in entry {
        let mapped = match key.as_str() {
            "type" | "enabled" => continue,
            "command" | "args" | "env" if transport == "stdio" => key.as_str(),
            "url" | "headers" if transport != "stdio" => key.as_str(),
            "http_headers" if transport != "stdio" => "headers",
            "required" | "timeout" | "allowedTools" | "deniedTools" => key.as_str(),
            "startup_timeout_sec" => "timeout",
            "enabled_tools" => "allowedTools",
            "disabled_tools" => "deniedTools",
            _ => return Err("包含当前全局 MCP 无法保留的字段，请调整后重新导入"),
        };
        let value = if matches!(mapped, "env" | "headers") {
            let pairs = value.as_object().ok_or("环境变量和请求头必须为对象")?;
            Value::Array(
                pairs.iter().map(|(name, value)| json!({ "name": name, "value": value })).collect(),
            )
        } else {
            value.clone()
        };
        if server.insert(mapped.into(), value).is_some() {
            return Err("存在重复配置字段");
        }
    }
    server.insert("name".into(), json!(name));
    server.insert("type".into(), json!(transport));
    server.insert("enabled".into(), json!(false));
    Ok(Value::Object(server))
}

#[cfg(test)]
#[path = "configuration_transfer_tests.rs"]
mod tests;

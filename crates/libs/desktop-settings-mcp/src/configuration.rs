use crate::state::{McpNewServerState, McpNewServerTransport};
use iced::widget::text_editor;
use serde_json::{Value, json};
#[derive(Debug, Clone)]
pub(crate) enum McpSettingsError {
    Raw(String),
    Localized(desktop_i18n::format::Message),
}
impl std::fmt::Display for McpSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw(raw) => f.write_str(raw),
            Self::Localized(message) => f.write_str(
                &message
                    .render(desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese))
                    .expect("valid MCP validation template"),
            ),
        }
    }
}
impl From<String> for McpSettingsError {
    fn from(raw: String) -> Self {
        Self::Raw(raw)
    }
}

fn invalid(path: &str, expected: &str) -> McpSettingsError {
    McpSettingsError::Localized(desktop_i18n::format::Message::new(
        "字段 %s 无效：需要 %s",
        vec![path.into(), expected.into()],
    ))
}

fn require_object<'a>(
    value: &'a Value,
    path: &str,
) -> Result<&'a serde_json::Map<String, Value>, McpSettingsError> {
    value.as_object().ok_or_else(|| invalid(path, "object"))
}

fn non_empty_string(value: Option<&Value>, path: &str) -> Result<String, McpSettingsError> {
    match value.and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        _ => Err(invalid(path, "non-empty string")),
    }
}

fn validate_optional_bool(value: Option<&Value>, path: &str) -> Result<(), McpSettingsError> {
    match value {
        None => Ok(()),
        Some(value) if value.is_boolean() => Ok(()),
        Some(_) => Err(invalid(path, "boolean")),
    }
}

fn validate_timeout(value: Option<&Value>, path: &str) -> Result<(), McpSettingsError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let Some(seconds) = value.as_f64() else {
        return Err(invalid(path, "positive seconds or null"));
    };
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err(invalid(path, "positive seconds or null"));
    }
    Ok(())
}

fn validate_string_array(
    value: Option<&Value>,
    path: &str,
    allow_empty: bool,
) -> Result<(), McpSettingsError> {
    let Some(value) = value else {
        return Ok(());
    };
    let array = value.as_array().ok_or_else(|| invalid(path, "array"))?;
    for (index, entry) in array.iter().enumerate() {
        match entry.as_str() {
            Some(text) if allow_empty || !text.trim().is_empty() => {}
            _ => {
                let expected = if allow_empty { "string" } else { "non-empty string" };
                return Err(invalid(&format!("{path}[{index}]"), expected));
            }
        }
    }
    Ok(())
}

fn validate_name_value_array(
    value: Option<&Value>,
    path: &str,
) -> Result<(), McpSettingsError> {
    let Some(value) = value else {
        return Ok(());
    };
    let array = value.as_array().ok_or_else(|| invalid(path, "array"))?;
    for (index, entry) in array.iter().enumerate() {
        let entry_path = format!("{path}[{index}]");
        let object = require_object(entry, &entry_path)?;
        non_empty_string(object.get("name"), &format!("{entry_path}.name"))?;
        non_empty_string(object.get("value"), &format!("{entry_path}.value"))?;
    }
    Ok(())
}

fn validate_meta(value: Option<&Value>, path: &str) -> Result<(), McpSettingsError> {
    match value {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Object(_)) => Ok(()),
        Some(_) => Err(invalid(path, "object or null")),
    }
}

pub(crate) fn validate_mcp_servers(value: &Value) -> Result<usize, McpSettingsError> {
    let servers =
        value.as_array().ok_or_else(|| invalid("mcpServers in global config", "array"))?;

    for (index, raw_server) in servers.iter().enumerate() {
        let path = format!("mcpServers[{index}] in global config");
        let server = require_object(raw_server, &path)?;
        non_empty_string(server.get("name"), &format!("{path}.name"))?;
        validate_meta(server.get("_meta"), &format!("{path}._meta"))?;
        validate_optional_bool(server.get("enabled"), &format!("{path}.enabled"))?;
        validate_optional_bool(server.get("required"), &format!("{path}.required"))?;
        validate_timeout(server.get("timeout"), &format!("{path}.timeout"))?;
        validate_string_array(server.get("allowedTools"), &format!("{path}.allowedTools"), false)?;
        validate_string_array(server.get("deniedTools"), &format!("{path}.deniedTools"), false)?;

        let server_type = match server.get("type") {
            None => "stdio".to_string(),
            Some(value) => non_empty_string(Some(value), &format!("{path}.type"))?,
        };
        match server_type.as_str() {
            "stdio" => {
                non_empty_string(server.get("command"), &format!("{path}.command"))?;
                validate_string_array(server.get("args"), &format!("{path}.args"), true)?;
                validate_name_value_array(server.get("env"), &format!("{path}.env"))?;
            }
            "http" | "sse" => {
                non_empty_string(server.get("url"), &format!("{path}.url"))?;
                validate_name_value_array(server.get("headers"), &format!("{path}.headers"))?;
            }
            _ => return Err(invalid(&format!("{path}.type"), "http, sse, or stdio")),
        }
    }

    Ok(servers.len())
}

fn validate_mcp_server_overrides(overrides: &[Value]) -> Result<(), McpSettingsError> {
    for (index, raw_override) in overrides.iter().enumerate() {
        let path = format!("mcpServerOverrides[{index}] in global config");
        let override_entry = require_object(raw_override, &path)?;
        non_empty_string(override_entry.get("name"), &format!("{path}.name"))?;
        validate_optional_bool(override_entry.get("enabled"), &format!("{path}.enabled"))?;
        validate_optional_bool(override_entry.get("required"), &format!("{path}.required"))?;
        validate_timeout(override_entry.get("timeout"), &format!("{path}.timeout"))?;
        validate_string_array(
            override_entry.get("allowedTools"),
            &format!("{path}.allowedTools"),
            false,
        )?;
        validate_string_array(
            override_entry.get("deniedTools"),
            &format!("{path}.deniedTools"),
            false,
        )?;
    }

    Ok(())
}

pub(crate) fn pretty(value: &Value) -> Result<String, McpSettingsError> {
    serde_json::to_string_pretty(value).map_err(|err| McpSettingsError::Raw(err.to_string()))
}

pub(crate) fn editor_value(editor: &text_editor::Content) -> Result<(Value, usize), McpSettingsError> {
    let text = editor.text();
    let value = if text.trim().is_empty() {
        Value::Array(Vec::new())
    } else {
        serde_json::from_str::<Value>(&text).map_err(|err| {
            McpSettingsError::Localized(desktop_i18n::format::Message::new(
                "JSON 解析失败: %s",
                vec![err.to_string().into()],
            ))
        })?
    };
    let count = validate_mcp_servers(&value)?;
    Ok((value, count))
}

fn optional_timeout(input: &str) -> Result<Option<Value>, McpSettingsError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let seconds = trimmed
        .parse::<f64>()
        .map_err(|_| invalid("new MCP server timeout", "positive seconds or empty"))?;
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err(invalid("new MCP server timeout", "positive seconds or empty"));
    }
    Ok(Some(json!(seconds)))
}

fn new_server_value(input: &McpNewServerState) -> Result<Value, McpSettingsError> {
    let name = input.name_input.trim();
    if name.is_empty() {
        return Err(invalid("new MCP server name", "non-empty string"));
    }

    let mut server = serde_json::Map::new();
    server.insert("name".to_string(), Value::String(name.to_string()));
    server.insert("type".to_string(), Value::String(input.transport.label().to_string()));
    server.insert("enabled".to_string(), Value::Bool(input.enabled));
    server.insert("required".to_string(), Value::Bool(input.required));
    if let Some(timeout) = optional_timeout(&input.timeout_input)? {
        server.insert("timeout".to_string(), timeout);
    }

    match input.transport {
        McpNewServerTransport::Stdio => {
            let command = input.command_input.trim();
            if command.is_empty() {
                return Err(invalid("new MCP stdio command", "non-empty string"));
            }
            server.insert("command".to_string(), Value::String(command.to_string()));
            let args = input
                .args_input
                .split([',', '\n'])
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if !args.is_empty() {
                server.insert("args".to_string(), json!(args));
            }
        }
        McpNewServerTransport::Http | McpNewServerTransport::Sse => {
            let url = input.url_input.trim();
            if url.is_empty() {
                return Err(invalid("new MCP server url", "non-empty string"));
            }
            server.insert("url".to_string(), Value::String(url.to_string()));
        }
    }

    Ok(Value::Object(server))
}

pub(crate) fn append_new_server(
    editor: &text_editor::Content,
    input: &McpNewServerState,
) -> Result<(Value, usize), McpSettingsError> {
    let (mut value, _) = editor_value(editor)?;
    let servers =
        value.as_array_mut().ok_or_else(|| invalid("mcpServers in global config", "array"))?;
    servers.push(new_server_value(input)?);
    let count = validate_mcp_servers(&value)?;
    Ok((value, count))
}

pub(crate) fn server_edit_state(
    editor: &text_editor::Content,
    index: usize,
) -> Result<McpNewServerState, McpSettingsError> {
    let (value, _) = editor_value(editor)?;
    let server = value
        .as_array()
        .and_then(|servers| servers.get(index))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid(&format!("mcpServers[{index}] in global config"), "object"))?;
    let transport = match server.get("type").and_then(Value::as_str).unwrap_or("stdio") {
        "stdio" => McpNewServerTransport::Stdio,
        "http" => McpNewServerTransport::Http,
        "sse" => McpNewServerTransport::Sse,
        value => return Err(invalid("MCP server type", &format!("known transport, got {value}"))),
    };
    let args_input = server
        .get("args")
        .and_then(Value::as_array)
        .map(|args| args.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "))
        .unwrap_or_default();
    let timeout_input = server
        .get("timeout")
        .filter(|value| !value.is_null())
        .map(Value::to_string)
        .unwrap_or_default();

    Ok(McpNewServerState {
        name_input: server.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
        transport,
        command_input: server
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        args_input,
        url_input: server.get("url").and_then(Value::as_str).unwrap_or_default().to_string(),
        timeout_input,
        enabled: server.get("enabled").and_then(Value::as_bool).unwrap_or(true),
        required: server.get("required").and_then(Value::as_bool).unwrap_or(false),
    })
}

pub(crate) fn update_server(
    editor: &text_editor::Content,
    index: usize,
    input: &McpNewServerState,
) -> Result<(Value, usize), McpSettingsError> {
    let (mut value, _) = editor_value(editor)?;
    let edited = new_server_value(input)?;
    let edited = edited.as_object().ok_or_else(|| invalid("edited MCP server", "object"))?;
    let server = value
        .as_array_mut()
        .and_then(|servers| servers.get_mut(index))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| invalid(&format!("mcpServers[{index}] in global config"), "object"))?;

    let previous_transport = server.get("type").and_then(Value::as_str).unwrap_or("stdio");
    let next_transport = edited.get("type").and_then(Value::as_str).unwrap_or("stdio");
    if previous_transport != next_transport {
        for key in ["command", "args", "env", "url", "headers"] {
            server.remove(key);
        }
    }

    for key in ["name", "type", "enabled", "required"] {
        if let Some(next) = edited.get(key) {
            server.insert(key.to_string(), next.clone());
        }
    }
    for key in ["timeout", "command", "args", "url"] {
        match edited.get(key) {
            Some(next) => {
                server.insert(key.to_string(), next.clone());
            }
            None => {
                server.remove(key);
            }
        }
    }

    let count = validate_mcp_servers(&value)?;
    Ok((value, count))
}

pub(crate) fn set_server_enabled(
    editor: &text_editor::Content,
    index: usize,
    enabled: bool,
) -> Result<(Value, usize), McpSettingsError> {
    let (mut value, _) = editor_value(editor)?;
    let servers =
        value.as_array_mut().ok_or_else(|| invalid("mcpServers in global config", "array"))?;
    let server = servers
        .get_mut(index)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| invalid(&format!("mcpServers[{index}] in global config"), "object"))?;
    server.insert("enabled".to_string(), Value::Bool(enabled));
    let count = validate_mcp_servers(&value)?;
    Ok((value, count))
}

pub(crate) fn delete_server(
    editor: &text_editor::Content,
    index: usize,
) -> Result<(Value, usize), McpSettingsError> {
    let (mut value, _) = editor_value(editor)?;
    let servers =
        value.as_array_mut().ok_or_else(|| invalid("mcpServers in global config", "array"))?;
    if index >= servers.len() {
        return Err(invalid(
            &format!("mcpServers[{index}] in global config"),
            "index within bounds",
        ));
    }
    servers.remove(index);
    let count = validate_mcp_servers(&value)?;
    Ok((value, count))
}

pub(crate) fn loaded_mcp_server_overrides(global_config: &Value) -> Vec<Value> {
    global_config.get("mcpServerOverrides").and_then(Value::as_array).cloned().unwrap_or_default()
}

pub(crate) fn loaded_mcp_servers(global_config: Value) -> Value {
    global_config.get("mcpServers").cloned().unwrap_or_else(|| Value::Array(Vec::new()))
}

pub(crate) fn set_runtime_server_enabled(
    overrides: &[Value],
    server_name: &str,
    enabled: bool,
) -> Result<Vec<Value>, McpSettingsError> {
    let name = server_name.trim();
    if name.is_empty() {
        return Err(invalid("runtime MCP server name", "non-empty string"));
    }

    let mut next = overrides.to_vec();
    let mut found = false;
    for raw_override in &mut next {
        let Some(object) = raw_override.as_object_mut() else {
            continue;
        };
        let Some(existing_name) = object.get("name").and_then(Value::as_str) else {
            continue;
        };
        if existing_name.trim() == name {
            object.insert("enabled".to_string(), Value::Bool(enabled));
            found = true;
            break;
        }
    }

    if !found {
        next.push(json!({
            "name": name,
            "enabled": enabled,
        }));
    }

    validate_mcp_server_overrides(&next)?;
    Ok(next)
}

#[cfg(test)]
#[path = "configuration_behavior_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "configuration_tests.rs"]
mod validation_tests;

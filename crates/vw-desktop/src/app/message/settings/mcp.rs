//! 处理 MCP 系统设置页的读取、校验和保存。

use crate::app::config::{
    load_global_config_value_async, load_mcp_inspect_async, patch_full_agent_config_async,
};
use crate::app::state::{McpNewServerState, McpNewServerTransport};
use crate::app::{App, Message};
use iced::Task;
use iced::widget::text_editor;
use serde_json::{Value, json};

use super::messages::{McpMessage, SettingsMessage};
use super::util::parse_comma_or_newline_list;

const MCP_TEMPLATE: &str = r#"[
  {
    "name": "local-demo",
    "type": "stdio",
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
    "enabled": true,
    "required": false,
    "timeout": 30,
    "allowedTools": []
  }
]"#;

fn refresh_task() -> Task<Message> {
    Task::perform(load_global_config_value_async(), |result| {
        Message::Settings(SettingsMessage::Mcp(McpMessage::Loaded(result)))
    })
}

fn inspect_task() -> Task<Message> {
    Task::perform(load_mcp_inspect_async(), |result| {
        Message::Settings(SettingsMessage::Mcp(McpMessage::Inspected(result)))
    })
}

fn refresh_all_task() -> Task<Message> {
    Task::batch(vec![refresh_task(), inspect_task()])
}

fn save_task(mcp_servers: Value) -> Task<Message> {
    Task::perform(
        async move { patch_full_agent_config_async(json!({ "mcpServers": mcp_servers })).await },
        |result| Message::Settings(SettingsMessage::Mcp(McpMessage::Saved(result))),
    )
}

fn save_runtime_overrides_task(runtime_overrides: Vec<Value>) -> Task<Message> {
    Task::perform(
        async move {
            patch_full_agent_config_async(json!({ "mcpServerOverrides": runtime_overrides })).await
        },
        |result| Message::Settings(SettingsMessage::Mcp(McpMessage::Saved(result))),
    )
}

fn invalid(path: &str, expected: &str) -> String {
    format!("Invalid {path}: expected {expected}")
}

fn require_object<'a>(
    value: &'a Value,
    path: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value.as_object().ok_or_else(|| invalid(path, "object"))
}

fn non_empty_string(value: Option<&Value>, path: &str) -> Result<String, String> {
    match value.and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        _ => Err(invalid(path, "non-empty string")),
    }
}

fn validate_optional_bool(value: Option<&Value>, path: &str) -> Result<(), String> {
    match value {
        None => Ok(()),
        Some(value) if value.is_boolean() => Ok(()),
        Some(_) => Err(invalid(path, "boolean")),
    }
}

fn validate_timeout(value: Option<&Value>, path: &str) -> Result<(), String> {
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
) -> Result<(), String> {
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

fn validate_name_value_array(value: Option<&Value>, path: &str) -> Result<(), String> {
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

fn validate_meta(value: Option<&Value>, path: &str) -> Result<(), String> {
    match value {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Object(_)) => Ok(()),
        Some(_) => Err(invalid(path, "object or null")),
    }
}

pub(super) fn validate_mcp_servers(value: &Value) -> Result<usize, String> {
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

fn validate_mcp_server_overrides(overrides: &[Value]) -> Result<(), String> {
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

fn pretty(value: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|err| err.to_string())
}

fn editor_value(editor: &text_editor::Content) -> Result<(Value, usize), String> {
    let text = editor.text();
    let value = if text.trim().is_empty() {
        Value::Array(Vec::new())
    } else {
        serde_json::from_str::<Value>(&text).map_err(|err| format!("JSON 解析失败: {err}"))?
    };
    let count = validate_mcp_servers(&value)?;
    Ok((value, count))
}

fn optional_timeout(input: &str) -> Result<Option<Value>, String> {
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

fn new_server_value(input: &McpNewServerState) -> Result<Value, String> {
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
            let args = parse_comma_or_newline_list(&input.args_input);
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

fn append_new_server(
    editor: &text_editor::Content,
    input: &McpNewServerState,
) -> Result<(Value, usize), String> {
    let (mut value, _) = editor_value(editor)?;
    let servers =
        value.as_array_mut().ok_or_else(|| invalid("mcpServers in global config", "array"))?;
    servers.push(new_server_value(input)?);
    let count = validate_mcp_servers(&value)?;
    Ok((value, count))
}

fn server_edit_state(
    editor: &text_editor::Content,
    index: usize,
) -> Result<McpNewServerState, String> {
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

fn update_server(
    editor: &text_editor::Content,
    index: usize,
    input: &McpNewServerState,
) -> Result<(Value, usize), String> {
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

fn set_server_enabled(
    editor: &text_editor::Content,
    index: usize,
    enabled: bool,
) -> Result<(Value, usize), String> {
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

fn delete_server(
    editor: &text_editor::Content,
    index: usize,
) -> Result<(Value, usize), String> {
    let (mut value, _) = editor_value(editor)?;
    let servers =
        value.as_array_mut().ok_or_else(|| invalid("mcpServers in global config", "array"))?;
    if index >= servers.len() {
        return Err(format!(
            "Invalid mcpServers[{index}] in global config: index out of bounds"
        ));
    }
    servers.remove(index);
    let count = validate_mcp_servers(&value)?;
    Ok((value, count))
}

fn loaded_mcp_server_overrides(global_config: &Value) -> Vec<Value> {
    global_config.get("mcpServerOverrides").and_then(Value::as_array).cloned().unwrap_or_default()
}

fn loaded_mcp_servers(global_config: Value) -> Value {
    global_config.get("mcpServers").cloned().unwrap_or_else(|| Value::Array(Vec::new()))
}

fn set_runtime_server_enabled(
    overrides: &[Value],
    server_name: &str,
    enabled: bool,
) -> Result<Vec<Value>, String> {
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

fn clear_edit_feedback(app: &mut App) {
    app.mcp_settings.save_error = None;
    app.mcp_settings.status_message = None;
}

/// 处理 `McpMessage` 对应的刷新、编辑和保存流程。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Mcp(message) = message else {
        return Task::none();
    };

    match message {
        McpMessage::TabSelected(tab) => {
            app.mcp_settings.active_tab = tab;
            app.mcp_settings.delete_pending = None;
            Task::none()
        }
        McpMessage::Refresh => {
            app.mcp_settings.loading = true;
            app.mcp_settings.inspect_loading = true;
            app.mcp_settings.delete_pending = None;
            app.mcp_settings.save_error = None;
            app.mcp_settings.inspect_error = None;
            app.mcp_settings.status_message = None;
            refresh_all_task()
        }
        McpMessage::Loaded(result) => {
            app.mcp_settings.loading = false;
            match result {
                Ok(global_config) => {
                    app.mcp_settings.delete_pending = None;
                    app.mcp_settings.runtime_overrides =
                        loaded_mcp_server_overrides(&global_config);
                    app.mcp_settings.new_server = McpNewServerState::default();
                    app.mcp_settings.editing_server_index = None;
                    let value = loaded_mcp_servers(global_config);
                    if let Ok(text) = pretty(&value) {
                        app.mcp_settings.editor = text_editor::Content::with_text(&text);
                    }
                    match validate_mcp_servers(&value) {
                        Ok(count) => {
                            app.mcp_settings.server_count = count;
                            app.mcp_settings.save_error = None;
                        }
                        Err(err) => {
                            app.mcp_settings.server_count = 0;
                            app.mcp_settings.save_error = Some(err);
                        }
                    }
                }
                Err(err) => app.mcp_settings.save_error = Some(err),
            }
            Task::none()
        }
        McpMessage::InspectRefresh => {
            app.mcp_settings.inspect_loading = true;
            app.mcp_settings.inspect_error = None;
            inspect_task()
        }
        McpMessage::Inspected(result) => {
            app.mcp_settings.inspect_loading = false;
            match result {
                Ok(inspect) => {
                    app.mcp_settings.inspect = Some(inspect);
                    app.mcp_settings.inspect_error = None;
                }
                Err(err) => app.mcp_settings.inspect_error = Some(err),
            }
            Task::none()
        }
        McpMessage::EditorAction(action) => {
            app.mcp_settings.editor.perform(action);
            app.mcp_settings.delete_pending = None;
            clear_edit_feedback(app);
            Task::none()
        }
        McpMessage::Format => match editor_value(&app.mcp_settings.editor) {
            Ok((value, count)) => {
                if let Ok(text) = pretty(&value) {
                    app.mcp_settings.editor = text_editor::Content::with_text(&text);
                }
                app.mcp_settings.server_count = count;
                clear_edit_feedback(app);
                Task::none()
            }
            Err(err) => {
                app.mcp_settings.save_error = Some(err);
                Task::none()
            }
        },
        McpMessage::InsertTemplate => {
            app.mcp_settings.editor = text_editor::Content::with_text(MCP_TEMPLATE);
            app.mcp_settings.server_count = 1;
            clear_edit_feedback(app);
            Task::none()
        }
        McpMessage::NewServerPanelToggled => {
            app.mcp_settings.new_server_expanded = !app.mcp_settings.new_server_expanded;
            Task::none()
        }
        McpMessage::NewServerNameChanged(value) => {
            app.mcp_settings.new_server.name_input = value;
            clear_edit_feedback(app);
            Task::none()
        }
        McpMessage::NewServerTransportChanged(value) => {
            app.mcp_settings.new_server.transport = value;
            clear_edit_feedback(app);
            Task::none()
        }
        McpMessage::NewServerCommandChanged(value) => {
            app.mcp_settings.new_server.command_input = value;
            clear_edit_feedback(app);
            Task::none()
        }
        McpMessage::NewServerArgsChanged(value) => {
            app.mcp_settings.new_server.args_input = value;
            clear_edit_feedback(app);
            Task::none()
        }
        McpMessage::NewServerUrlChanged(value) => {
            app.mcp_settings.new_server.url_input = value;
            clear_edit_feedback(app);
            Task::none()
        }
        McpMessage::NewServerTimeoutChanged(value) => {
            app.mcp_settings.new_server.timeout_input = value;
            clear_edit_feedback(app);
            Task::none()
        }
        McpMessage::NewServerEnabledToggled(value) => {
            app.mcp_settings.new_server.enabled = value;
            clear_edit_feedback(app);
            Task::none()
        }
        McpMessage::NewServerRequiredToggled(value) => {
            app.mcp_settings.new_server.required = value;
            clear_edit_feedback(app);
            Task::none()
        }
        McpMessage::AddServer => {
            match append_new_server(&app.mcp_settings.editor, &app.mcp_settings.new_server) {
                Ok((value, count)) => {
                    if let Ok(text) = pretty(&value) {
                        app.mcp_settings.editor = text_editor::Content::with_text(&text);
                    }
                    app.mcp_settings.new_server = McpNewServerState::default();
                    app.mcp_settings.editing_server_index = None;
                    app.mcp_settings.server_count = count;
                    app.mcp_settings.saving = true;
                    clear_edit_feedback(app);
                    save_task(value)
                }
                Err(err) => {
                    app.mcp_settings.save_error = Some(err);
                    Task::none()
                }
            }
        }
        McpMessage::ServerEditRequested(index) => {
            app.mcp_settings.delete_pending = None;
            match server_edit_state(&app.mcp_settings.editor, index) {
                Ok(state) => {
                    app.mcp_settings.new_server = state;
                    app.mcp_settings.editing_server_index = Some(index);
                    app.mcp_settings.new_server_expanded = true;
                    clear_edit_feedback(app);
                }
                Err(err) => app.mcp_settings.save_error = Some(err),
            }
            Task::none()
        }
        McpMessage::ServerEditCanceled => {
            app.mcp_settings.new_server = McpNewServerState::default();
            app.mcp_settings.editing_server_index = None;
            app.mcp_settings.delete_pending = None;
            clear_edit_feedback(app);
            Task::none()
        }
        McpMessage::SaveServerEdit => {
            let Some(index) = app.mcp_settings.editing_server_index else {
                app.mcp_settings.save_error = Some("没有正在编辑的 MCP 服务器".to_string());
                return Task::none();
            };
            match update_server(&app.mcp_settings.editor, index, &app.mcp_settings.new_server) {
                Ok((value, count)) => {
                    if let Ok(text) = pretty(&value) {
                        app.mcp_settings.editor = text_editor::Content::with_text(&text);
                    }
                    app.mcp_settings.new_server = McpNewServerState::default();
                    app.mcp_settings.editing_server_index = None;
                    app.mcp_settings.server_count = count;
                    app.mcp_settings.saving = true;
                    clear_edit_feedback(app);
                    save_task(value)
                }
                Err(err) => {
                    app.mcp_settings.save_error = Some(err);
                    Task::none()
                }
            }
        }
        McpMessage::ServerEnabledToggled(index, enabled) => {
            match set_server_enabled(&app.mcp_settings.editor, index, enabled) {
                Ok((value, count)) => {
                    if let Ok(text) = pretty(&value) {
                        app.mcp_settings.editor = text_editor::Content::with_text(&text);
                    }
                    app.mcp_settings.server_count = count;
                    app.mcp_settings.saving = true;
                    clear_edit_feedback(app);
                    save_task(value)
                }
                Err(err) => {
                    app.mcp_settings.save_error = Some(err);
                    Task::none()
                }
            }
        }
        McpMessage::RuntimeServerEnabledToggled(name, enabled) => {
            match set_runtime_server_enabled(&app.mcp_settings.runtime_overrides, &name, enabled) {
                Ok(overrides) => {
                    app.mcp_settings.runtime_overrides = overrides.clone();
                    app.mcp_settings.saving = true;
                    clear_edit_feedback(app);
                    save_runtime_overrides_task(overrides)
                }
                Err(err) => {
                    app.mcp_settings.save_error = Some(err);
                    Task::none()
                }
            }
        }
        McpMessage::PreviewExpandedToggled(key) => {
            if app.mcp_settings.expanded_previews.contains(&key) {
                app.mcp_settings.expanded_previews.remove(&key);
            } else {
                app.mcp_settings.expanded_previews.insert(key);
            }
            Task::none()
        }
        McpMessage::RequestDelete(index) => {
            if app.mcp_settings.loading
                || app.mcp_settings.saving
                || app.mcp_settings.active_tab != crate::app::state::McpSettingsTab::Servers
            {
                return Task::none();
            }
            match editor_value(&app.mcp_settings.editor) {
                Ok((value, _)) if value.as_array().is_some_and(|servers| index < servers.len()) => {
                    app.mcp_settings.delete_pending = Some(index);
                    clear_edit_feedback(app);
                }
                Ok(_) => {
                    app.mcp_settings.delete_pending = None;
                    app.mcp_settings.save_error = Some(format!(
                        "Invalid mcpServers[{index}] in global config: index out of bounds"
                    ));
                }
                Err(err) => {
                    app.mcp_settings.delete_pending = None;
                    app.mcp_settings.save_error = Some(err);
                }
            }
            Task::none()
        }
        McpMessage::ConfirmDelete(index) => {
            let pending_matches = app.mcp_settings.delete_pending == Some(index);
            app.mcp_settings.delete_pending = None;
            if !pending_matches || app.mcp_settings.loading || app.mcp_settings.saving {
                return Task::none();
            }
            match delete_server(&app.mcp_settings.editor, index) {
                Ok((value, count)) => {
                    if let Ok(text) = pretty(&value) {
                        app.mcp_settings.editor = text_editor::Content::with_text(&text);
                    }
                    app.mcp_settings.server_count = count;
                    match app.mcp_settings.editing_server_index {
                        Some(editing_index) if editing_index == index => {
                            app.mcp_settings.editing_server_index = None;
                            app.mcp_settings.new_server = McpNewServerState::default();
                            app.mcp_settings.new_server_expanded = false;
                        }
                        Some(editing_index) if editing_index > index => {
                            app.mcp_settings.editing_server_index = Some(editing_index - 1);
                        }
                        _ => {}
                    }
                    app.mcp_settings.saving = true;
                    clear_edit_feedback(app);
                    save_task(value)
                }
                Err(err) => {
                    app.mcp_settings.save_error = Some(err);
                    Task::none()
                }
            }
        }
        McpMessage::CancelDelete => {
            app.mcp_settings.delete_pending = None;
            Task::none()
        }
        McpMessage::Save => match editor_value(&app.mcp_settings.editor) {
            Ok((value, count)) => {
                app.mcp_settings.server_count = count;
                app.mcp_settings.saving = true;
                clear_edit_feedback(app);
                save_task(value)
            }
            Err(err) => {
                app.mcp_settings.save_error = Some(err);
                Task::none()
            }
        },
        McpMessage::Saved(result) => {
            app.mcp_settings.saving = false;
            match result {
                Ok(()) => {
                    app.mcp_settings.loading = true;
                    app.mcp_settings.inspect_loading = true;
                    app.mcp_settings.save_error = None;
                    app.mcp_settings.inspect_error = None;
                    app.mcp_settings.status_message = Some("MCP 配置已保存".to_string());
                    refresh_all_task()
                }
                Err(err) => {
                    app.mcp_settings.save_error = Some(err);
                    Task::none()
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod mcp_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validate_mcp_servers_accepts_supported_transports() {
        let value = json!([
            {"name": "local", "type": "stdio", "command": "npx", "args": ["tool"], "env": [{"name": "TOKEN", "value": "x"}]},
            {"name": "http", "type": "http", "url": "https://example.test/mcp", "headers": [{"name": "Authorization", "value": "Bearer x"}]},
            {"name": "sse", "type": "sse", "url": "https://example.test/sse", "allowedTools": ["search"], "timeout": 15}
        ]);

        assert_eq!(validate_mcp_servers(&value).expect("valid"), 3);
    }

    #[test]
    fn validate_mcp_servers_rejects_invalid_shape() {
        let value = json!([{"name": "broken", "type": "http"}]);

        let error = validate_mcp_servers(&value).expect_err("invalid");
        assert!(error.contains("mcpServers[0] in global config.url"));
    }

    #[test]
    fn set_server_enabled_updates_selected_server_only() {
        let editor = text_editor::Content::with_text(
            r#"[
              {"name": "first", "command": "npx", "enabled": true},
              {"name": "second", "command": "node", "enabled": false}
            ]"#,
        );

        let (value, count) = set_server_enabled(&editor, 1, true).expect("updated");

        assert_eq!(count, 2);
        assert_eq!(value[0]["enabled"], true);
        assert_eq!(value[1]["enabled"], true);
    }

    #[test]
    fn set_runtime_server_enabled_appends_override() {
        let overrides = set_runtime_server_enabled(&[], "plugin-mcp", false).expect("updated");

        assert_eq!(overrides, vec![json!({"name": "plugin-mcp", "enabled": false})]);
    }

    #[test]
    fn set_runtime_server_enabled_updates_existing_override_only() {
        let raw = vec![
            json!({"name": "plugin-mcp", "enabled": false, "required": true}),
            json!({"name": "other", "enabled": false}),
        ];

        let overrides = set_runtime_server_enabled(&raw, "plugin-mcp", true).expect("updated");

        assert_eq!(overrides[0]["enabled"], true);
        assert_eq!(overrides[0]["required"], true);
        assert_eq!(overrides[1]["enabled"], false);
    }

    #[test]
    fn append_new_server_adds_stdio_server() {
        let editor = text_editor::Content::with_text("[]");
        let form = McpNewServerState {
            name_input: "filesystem".to_string(),
            command_input: "npx".to_string(),
            args_input: "-y\n@modelcontextprotocol/server-filesystem\n/tmp".to_string(),
            ..McpNewServerState::default()
        };

        let (value, count) = append_new_server(&editor, &form).expect("appended");

        assert_eq!(count, 1);
        assert_eq!(value[0]["name"], json!("filesystem"));
        assert_eq!(value[0]["type"], json!("stdio"));
        assert_eq!(value[0]["command"], json!("npx"));
        assert_eq!(
            value[0]["args"],
            json!(["-y", "@modelcontextprotocol/server-filesystem", "/tmp"])
        );
        assert_eq!(value[0]["enabled"], json!(true));
        assert_eq!(value[0]["timeout"], json!(30.0));
    }

    #[test]
    fn append_new_server_adds_http_server_without_empty_timeout() {
        let editor = text_editor::Content::with_text(r#"[{"name": "local", "command": "npx"}]"#);
        let form = McpNewServerState {
            name_input: "remote".to_string(),
            transport: McpNewServerTransport::Http,
            url_input: "https://example.test/mcp".to_string(),
            timeout_input: String::new(),
            enabled: false,
            required: true,
            ..McpNewServerState::default()
        };

        let (value, count) = append_new_server(&editor, &form).expect("appended");

        assert_eq!(count, 2);
        assert_eq!(value[1]["name"], json!("remote"));
        assert_eq!(value[1]["type"], json!("http"));
        assert_eq!(value[1]["url"], json!("https://example.test/mcp"));
        assert_eq!(value[1]["enabled"], json!(false));
        assert_eq!(value[1]["required"], json!(true));
        assert!(value[1].get("timeout").is_none());
    }

    #[test]
    fn append_new_server_rejects_missing_stdio_command() {
        let editor = text_editor::Content::with_text("[]");
        let form = McpNewServerState {
            name_input: "broken".to_string(),
            command_input: " ".to_string(),
            ..McpNewServerState::default()
        };

        let error = append_new_server(&editor, &form).expect_err("invalid");

        assert!(error.contains("new MCP stdio command"));
    }
}

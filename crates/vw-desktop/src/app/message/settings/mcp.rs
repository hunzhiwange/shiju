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

#[derive(Debug, Clone)]
pub(super) enum McpSettingsError {
    Raw(String),
    Localized(crate::app::i18n::format::Message),
}
impl std::fmt::Display for McpSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw(raw) => f.write_str(raw),
            Self::Localized(message) => f.write_str(
                &message
                    .render(crate::app::i18n::I18n::new(
                        vw_config_types::ui::DesktopLocale::SimplifiedChinese,
                    ))
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
fn set_validation_error(app: &mut App, error: impl Into<McpSettingsError>) {
    let error = error.into();
    app.mcp_settings.save_error = Some(error.to_string());
    app.mcp_settings.save_error_translation = match error {
        McpSettingsError::Localized(message) => Some(message),
        McpSettingsError::Raw(_) => None,
    };
}

fn invalid(path: &str, expected: &str) -> McpSettingsError {
    McpSettingsError::Localized(crate::app::i18n::format::Message::new(
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

fn validate_name_value_array(value: Option<&Value>, path: &str) -> Result<(), McpSettingsError> {
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

pub(super) fn validate_mcp_servers(value: &Value) -> Result<usize, McpSettingsError> {
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

fn pretty(value: &Value) -> Result<String, McpSettingsError> {
    serde_json::to_string_pretty(value).map_err(|err| McpSettingsError::Raw(err.to_string()))
}

fn editor_value(editor: &text_editor::Content) -> Result<(Value, usize), McpSettingsError> {
    let text = editor.text();
    let value = if text.trim().is_empty() {
        Value::Array(Vec::new())
    } else {
        serde_json::from_str::<Value>(&text).map_err(|err| {
            McpSettingsError::Localized(crate::app::i18n::format::Message::new(
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
) -> Result<(Value, usize), McpSettingsError> {
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

fn update_server(
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

fn set_server_enabled(
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

fn delete_server(
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
        McpMessage::ConnectionForm(message) => super::mcp_management_form::update(app, message),
        McpMessage::Diagnostic(message) => {
            crate::app::message::agent_apps_diagnostics::update(app, message)
        }
        McpMessage::Bindings(message) => super::mcp_management_bindings::update(app, message),
        McpMessage::Import(message) => super::mcp_management_import::update(app, message),
        McpMessage::Catalog(message) => super::mcp_management_list::update(app, message),
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
                            set_validation_error(app, err);
                        }
                    }
                }
                Err(err) => set_validation_error(app, err),
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
                set_validation_error(app, err);
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
                    set_validation_error(app, err);
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
                Err(err) => set_validation_error(app, err),
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
                    set_validation_error(app, err);
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
                    set_validation_error(app, err);
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
                    set_validation_error(app, err);
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
                    set_validation_error(app, err);
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
                    set_validation_error(app, err);
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
                set_validation_error(app, err);
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
                    set_validation_error(app, err);
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
#[path = "mcp_behavior_tests.rs"]
mod tests;

use crate::configuration::*;
use crate::state::McpNewServerState;
use iced::widget::text_editor;
use serde_json::Value;
const MCP_TEMPLATE: &str = r#"  {
    "name": "local-demo",
    "type": "stdio",
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
    "enabled": true,
    "required": false,
    "timeout": 30,
    "allowedTools": []
  }"#;

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(crate::state::McpSettingsTab),
    ServerSearchChanged(String),
    Refresh,
    Loaded(Result<serde_json::Value, String>),
    InspectRefresh,
    Connect(String),
    Connected(String, Result<vw_api_mcp_inspect::McpInspectResponse, String>),
    Inspected(Result<vw_api_mcp_inspect::McpInspectResponse, String>),
    EditorAction(text_editor::Action),
    Format,
    InsertTemplate,
    NewServerPanelToggled,
    NewServerNameChanged(String),
    NewServerTransportChanged(crate::state::McpNewServerTransport),
    NewServerCommandChanged(String),
    NewServerArgsChanged(String),
    NewServerUrlChanged(String),
    NewServerTimeoutChanged(String),
    NewServerEnabledToggled(bool),
    NewServerRequiredToggled(bool),
    AddServer,
    ServerEditRequested(usize),
    ServerEditCanceled,
    SaveServerEdit,
    ServerEnabledToggled(usize, bool),
    RuntimeServerEnabledToggled(String, bool),
    PreviewExpandedToggled(String),
    /// 请求删除指定索引的 MCP 服务器（弹出确认）。
    RequestDelete(usize),
    /// 确认删除指定的 MCP 服务器；处理时仍需匹配当前待确认索引。
    ConfirmDelete(usize),
    /// 取消删除操作。
    CancelDelete,
    Save,
    Saved(Result<(), String>),
}
#[derive(Debug)]
pub enum Effect {
    None,
    Inspect,
    InspectServer { key: String, name: String },
    RefreshAll,
    RefreshAllAndCatalog,
    Save(Value),
    SaveOverrides(Vec<Value>),
}
fn set_validation_error(app: &mut crate::state::State, error: impl Into<McpSettingsError>) {
    let error = error.into();
    app.save_error = Some(error.to_string());
    app.save_error_translation = match error {
        McpSettingsError::Localized(message) => Some(message),
        McpSettingsError::Raw(_) => None,
    };
}
fn clear_edit_feedback(app: &mut crate::state::State) {
    app.save_error = None;
    app.status_message = None;
}

fn append_template(editor: &text_editor::Content) -> Result<(String, usize), McpSettingsError> {
    let (servers, count) = editor_value(editor)?;
    let servers = servers.as_array().expect("validated MCP server array");
    let mut name = "local-demo".to_owned();
    let mut suffix = 2;
    while servers
        .iter()
        .any(|server| server["name"].as_str().map(str::trim) == Some(name.as_str()))
    {
        name = format!("local-demo-{suffix}");
        suffix += 1;
    }
    let template = MCP_TEMPLATE.replacen("local-demo", &name, 1);

    let mut text = editor.text();
    if text.trim().is_empty() {
        text.push_str("[]");
    }
    // Insert at the end of the array while preserving the existing draft's text.
    let before_close = text.trim_end().strip_suffix(']').expect("validated JSON array");
    let insertion_point = before_close.trim_end().len();
    let separator = if count == 0 { "\n" } else { ",\n" };
    let newline = if before_close[insertion_point..].contains('\n') { "" } else { "\n" };
    text.insert_str(insertion_point, &format!("{separator}{template}{newline}"));
    Ok((text, count + 1))
}

/// 处理 `McpMessage` 对应的刷新、编辑和保存流程。
pub fn update(app: &mut crate::state::State, message: Message) -> Effect {
    match message {
        Message::TabSelected(tab) => {
            app.active_tab = tab;
            app.delete_pending = None;
            Effect::None
        }
        Message::ServerSearchChanged(query) => {
            app.server_query = query;
            Effect::None
        }
        Message::Refresh => {
            app.loading = true;
            app.inspect_loading = true;
            app.delete_pending = None;
            app.save_error = None;
            app.inspect_error = None;
            app.status_message = None;
            Effect::RefreshAllAndCatalog
        }
        Message::Loaded(result) => {
            app.loading = false;
            match result {
                Ok(global_config) => {
                    app.delete_pending = None;
                    app.runtime_overrides = loaded_mcp_server_overrides(&global_config);
                    app.new_server = McpNewServerState::default();
                    app.editing_server_index = None;
                    let value = loaded_mcp_servers(global_config);
                    if let Ok(text) = pretty(&value) {
                        app.editor = text_editor::Content::with_text(&text);
                    }
                    match validate_mcp_servers(&value) {
                        Ok(count) => {
                            app.server_count = count;
                            app.save_error = None;
                        }
                        Err(err) => {
                            app.server_count = 0;
                            set_validation_error(app, err);
                        }
                    }
                }
                Err(err) => set_validation_error(app, err),
            }
            Effect::None
        }
        Message::InspectRefresh => {
            app.inspect_loading = true;
            app.inspect_error = None;
            Effect::Inspect
        }
        Message::Connect(key) => {
            if app.loading || app.saving || app.connecting.is_some() {
                return Effect::None;
            }
            let Some((_, name)) = key.split_once(':') else {
                return Effect::None;
            };
            let name = name.to_owned();
            if app.inspect_loading {
                app.rechecked_during_refresh.insert(name.clone());
            }
            app.connecting = Some(key.clone());
            app.expanded_previews.insert(key.clone());
            app.inspect_error = None;
            Effect::InspectServer { key, name }
        }
        Message::Connected(key, result) => {
            if app.connecting.as_ref() != Some(&key) {
                return Effect::None;
            }
            app.connecting = None;
            let Some((_, name)) = key.split_once(':') else {
                return Effect::None;
            };
            let found = match result {
                Ok(response) => response
                    .servers
                    .into_iter()
                    .find(|server| server.name == name)
                    .ok_or_else(|| format!("当前运行时配置中找不到 MCP 服务器 {name}")),
                Err(error) => Err(error),
            };
            match found {
                Ok(server) => {
                    let inspect = app.inspect.get_or_insert_with(|| {
                        vw_api_mcp_inspect::McpInspectResponse { servers: Vec::new() }
                    });
                    if let Some(existing) =
                        inspect.servers.iter_mut().find(|server| server.name == name)
                    {
                        *existing = server;
                    } else {
                        inspect.servers.push(server);
                    }
                }
                Err(error) => {
                    if let Some(server) = app.inspect.as_mut().and_then(|inspect| {
                        inspect.servers.iter_mut().find(|server| server.name == name)
                    }) {
                        server.connection_status = vw_api_mcp_inspect::McpConnectionStatusDto::Error;
                        server.error = Some(error.clone());
                    }
                    app.inspect_error = Some(error);
                }
            }
            Effect::None
        }
        Message::Inspected(result) => {
            app.inspect_loading = false;
            let rechecked = std::mem::take(&mut app.rechecked_during_refresh);
            match result {
                Ok(mut inspect) => {
                    // A later, explicit check owns this server's preview.
                    inspect.servers.retain(|server| !rechecked.contains(&server.name));
                    if let Some(previous) = app.inspect.take() {
                        inspect.servers.extend(
                            previous.servers.into_iter().filter(|server| rechecked.contains(&server.name)),
                        );
                    }
                    app.inspect = Some(inspect);
                    if rechecked.is_empty() {
                        app.inspect_error = None;
                    }
                }
                Err(err) => app.inspect_error = Some(err),
            }
            Effect::None
        }
        Message::EditorAction(action) => {
            app.editor.perform(action);
            app.delete_pending = None;
            clear_edit_feedback(app);
            Effect::None
        }
        Message::Format => match editor_value(&app.editor) {
            Ok((value, count)) => {
                if let Ok(text) = pretty(&value) {
                    app.editor = text_editor::Content::with_text(&text);
                }
                app.server_count = count;
                clear_edit_feedback(app);
                Effect::None
            }
            Err(err) => {
                set_validation_error(app, err);
                Effect::None
            }
        },
        Message::InsertTemplate => {
            match append_template(&app.editor) {
                Ok((text, count)) => {
                    app.editor = text_editor::Content::with_text(&text);
                    app.server_count = count;
                    app.delete_pending = None;
                    clear_edit_feedback(app);
                }
                Err(error) => set_validation_error(app, error),
            }
            Effect::None
        }
        Message::NewServerPanelToggled => {
            app.new_server_expanded = !app.new_server_expanded;
            Effect::None
        }
        Message::NewServerNameChanged(value) => {
            app.new_server.name_input = value;
            clear_edit_feedback(app);
            Effect::None
        }
        Message::NewServerTransportChanged(value) => {
            app.new_server.transport = value;
            clear_edit_feedback(app);
            Effect::None
        }
        Message::NewServerCommandChanged(value) => {
            app.new_server.command_input = value;
            clear_edit_feedback(app);
            Effect::None
        }
        Message::NewServerArgsChanged(value) => {
            app.new_server.args_input = value;
            clear_edit_feedback(app);
            Effect::None
        }
        Message::NewServerUrlChanged(value) => {
            app.new_server.url_input = value;
            clear_edit_feedback(app);
            Effect::None
        }
        Message::NewServerTimeoutChanged(value) => {
            app.new_server.timeout_input = value;
            clear_edit_feedback(app);
            Effect::None
        }
        Message::NewServerEnabledToggled(value) => {
            app.new_server.enabled = value;
            clear_edit_feedback(app);
            Effect::None
        }
        Message::NewServerRequiredToggled(value) => {
            app.new_server.required = value;
            clear_edit_feedback(app);
            Effect::None
        }
        Message::AddServer => match append_new_server(&app.editor, &app.new_server) {
            Ok((value, count)) => {
                if let Ok(text) = pretty(&value) {
                    app.editor = text_editor::Content::with_text(&text);
                }
                app.new_server = McpNewServerState::default();
                app.editing_server_index = None;
                app.server_count = count;
                app.saving = true;
                clear_edit_feedback(app);
                Effect::Save(value)
            }
            Err(err) => {
                set_validation_error(app, err);
                Effect::None
            }
        },
        Message::ServerEditRequested(index) => {
            app.delete_pending = None;
            match server_edit_state(&app.editor, index) {
                Ok(state) => {
                    app.new_server = state;
                    app.editing_server_index = Some(index);
                    app.new_server_expanded = true;
                    clear_edit_feedback(app);
                }
                Err(err) => set_validation_error(app, err),
            }
            Effect::None
        }
        Message::ServerEditCanceled => {
            app.new_server = McpNewServerState::default();
            app.editing_server_index = None;
            app.delete_pending = None;
            clear_edit_feedback(app);
            Effect::None
        }
        Message::SaveServerEdit => {
            let Some(index) = app.editing_server_index else {
                app.save_error = Some("没有正在编辑的 MCP 服务器".to_string());
                return Effect::None;
            };
            match update_server(&app.editor, index, &app.new_server) {
                Ok((value, count)) => {
                    if let Ok(text) = pretty(&value) {
                        app.editor = text_editor::Content::with_text(&text);
                    }
                    app.new_server = McpNewServerState::default();
                    app.editing_server_index = None;
                    app.server_count = count;
                    app.saving = true;
                    clear_edit_feedback(app);
                    Effect::Save(value)
                }
                Err(err) => {
                    set_validation_error(app, err);
                    Effect::None
                }
            }
        }
        Message::ServerEnabledToggled(index, enabled) => {
            match set_server_enabled(&app.editor, index, enabled) {
                Ok((value, count)) => {
                    if let Ok(text) = pretty(&value) {
                        app.editor = text_editor::Content::with_text(&text);
                    }
                    app.server_count = count;
                    app.saving = true;
                    clear_edit_feedback(app);
                    Effect::Save(value)
                }
                Err(err) => {
                    set_validation_error(app, err);
                    Effect::None
                }
            }
        }
        Message::RuntimeServerEnabledToggled(name, enabled) => {
            match set_runtime_server_enabled(&app.runtime_overrides, &name, enabled) {
                Ok(overrides) => {
                    app.runtime_overrides = overrides.clone();
                    app.saving = true;
                    clear_edit_feedback(app);
                    Effect::SaveOverrides(overrides)
                }
                Err(err) => {
                    set_validation_error(app, err);
                    Effect::None
                }
            }
        }
        Message::PreviewExpandedToggled(key) => {
            if app.expanded_previews.contains(&key) {
                app.expanded_previews.remove(&key);
            } else {
                app.expanded_previews.insert(key);
            }
            Effect::None
        }
        Message::RequestDelete(index) => {
            if app.loading || app.saving || app.active_tab != crate::state::McpSettingsTab::Servers
            {
                return Effect::None;
            }
            match editor_value(&app.editor)
                .and_then(|(value, _)| validate_server_deletion(&value, index))
            {
                Ok(()) => {
                    app.delete_pending = Some(index);
                    clear_edit_feedback(app);
                }
                Err(err) => {
                    app.delete_pending = None;
                    set_validation_error(app, err);
                }
            }
            Effect::None
        }
        Message::ConfirmDelete(index) => {
            let pending_matches = app.delete_pending == Some(index);
            app.delete_pending = None;
            if !pending_matches || app.loading || app.saving {
                return Effect::None;
            }
            match delete_server(&app.editor, index) {
                Ok((value, count)) => {
                    if let Ok(text) = pretty(&value) {
                        app.editor = text_editor::Content::with_text(&text);
                    }
                    app.server_count = count;
                    match app.editing_server_index {
                        Some(editing_index) if editing_index == index => {
                            app.editing_server_index = None;
                            app.new_server = McpNewServerState::default();
                            app.new_server_expanded = false;
                        }
                        Some(editing_index) if editing_index > index => {
                            app.editing_server_index = Some(editing_index - 1);
                        }
                        _ => {}
                    }
                    app.saving = true;
                    clear_edit_feedback(app);
                    Effect::Save(value)
                }
                Err(err) => {
                    set_validation_error(app, err);
                    Effect::None
                }
            }
        }
        Message::CancelDelete => {
            app.delete_pending = None;
            Effect::None
        }
        Message::Save => match editor_value(&app.editor) {
            Ok((value, count)) => {
                app.server_count = count;
                app.saving = true;
                clear_edit_feedback(app);
                Effect::Save(value)
            }
            Err(err) => {
                set_validation_error(app, err);
                Effect::None
            }
        },
        Message::Saved(result) => {
            app.saving = false;
            match result {
                Ok(()) => {
                    app.loading = true;
                    app.inspect_loading = true;
                    app.save_error = None;
                    app.inspect_error = None;
                    app.status_message = Some("MCP 配置已保存".to_string());
                    Effect::RefreshAll
                }
                Err(err) => {
                    set_validation_error(app, err);
                    Effect::None
                }
            }
        }
    }
}

// Managed definitions share the configuration domain; gateway clients retain authorization.
pub use crate::management_bindings_update as bindings;
pub use crate::management_form_update as form;
pub use crate::management_import_update as import;
pub use crate::management_list as catalog;
pub use crate::management_probe as probe;
pub use crate::management_resync as resync;
pub use crate::management_tools as tools;

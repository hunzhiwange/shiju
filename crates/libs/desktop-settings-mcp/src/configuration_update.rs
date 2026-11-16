use crate::configuration::*;
use crate::state::McpNewServerState;
use iced::widget::text_editor;
use serde_json::Value;
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

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(crate::state::McpSettingsTab),
    Refresh,
    Loaded(Result<serde_json::Value, String>),
    InspectRefresh,
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

/// 处理 `McpMessage` 对应的刷新、编辑和保存流程。
pub fn update(app: &mut crate::state::State, message: Message) -> Effect {
    match message {
        Message::TabSelected(tab) => {
            app.active_tab = tab;
            app.delete_pending = None;
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
        Message::Inspected(result) => {
            app.inspect_loading = false;
            match result {
                Ok(inspect) => {
                    app.inspect = Some(inspect);
                    app.inspect_error = None;
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
            app.editor = text_editor::Content::with_text(MCP_TEMPLATE);
            app.server_count = 1;
            clear_edit_feedback(app);
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
            match editor_value(&app.editor) {
                Ok((value, _)) if value.as_array().is_some_and(|servers| index < servers.len()) => {
                    app.delete_pending = Some(index);
                    clear_edit_feedback(app);
                }
                Ok(_) => {
                    app.delete_pending = None;
                    app.save_error = Some(format!(
                        "Invalid mcpServers[{index}] in global config: index out of bounds"
                    ));
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

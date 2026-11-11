//! Connection drafts use the public DTO; only the gateway validates and persists definitions.
use super::{McpMessage, SettingsMessage, mcp_management_list as catalog};
use crate::app::{App, Message};
use iced::{Task, widget::text_editor};
use std::collections::BTreeMap;
use vw_gateway_client::{GatewayClient, vw_api_types::mcp_management::*};

#[derive(Clone)]
pub(crate) struct Draft {
    pub(crate) context: catalog::Context,
    generation: u64,
    pub(crate) record: Option<McpCatalogRecord>,
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) transport: String,
    pub(crate) command: String,
    pub(crate) args: String,
    pub(crate) values: String,
    pub(crate) cwd: String,
    pub(crate) url: String,
    pub(crate) editor: text_editor::Content,
    pub(crate) json: bool,
    pub(crate) saving: bool,
    pub(crate) error: Option<String>,
    pub(crate) presets: Vec<McpPresetDraftView>,
}
impl std::fmt::Debug for Draft {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpConnectionDraft").finish_non_exhaustive()
    }
}
#[derive(Clone)]
pub enum FormMessage {
    Open(Option<McpCatalogRecord>),
    Close,
    Field(&'static str, String),
    Json(bool),
    Editor(text_editor::Action),
    Presets(catalog::Context, u64, Result<McpPresetsResponse, String>),
    Preset(usize),
    ValueAdd,
    ValueRemove(String),
    ValueName(String, String),
    ValueKind(String, bool),
    ValueText(String, String),
    ValueService(String, String),
    Save,
    Saved(catalog::Context, Result<McpCatalogRecord, String>),
}
impl std::fmt::Debug for FormMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("McpConnectionFormMessage { .. }")
    }
}
pub(crate) fn message(message: FormMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::ConnectionForm(message)))
}
fn parse<T: serde::de::DeserializeOwned>(input: &str) -> Result<T, String> {
    serde_json::from_str(input).map_err(|error| {
        // serde messages may echo secrets; expose only position and category.
        format!(
            "JSON 无效：第 {} 行，第 {} 列（{:?}）",
            error.line(),
            error.column(),
            error.classify()
        )
    })
}
impl Draft {
    pub(crate) fn new(context: catalog::Context, record: Option<McpCatalogRecord>) -> Self {
        let transport = match record.as_ref().map(|record| record.transport) {
            Some(McpTransport::StreamableHttp) => "streamable_http",
            Some(McpTransport::Sse) => "sse",
            _ => "stdio",
        };
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            generation: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            context,
            id: record.as_ref().map(|r| r.record.id.0.clone()).unwrap_or_default(),
            name: record.as_ref().map(|r| r.name.clone()).unwrap_or_default(),
            record,
            transport: transport.into(),
            command: String::new(),
            args: "[]".into(),
            values: "{}".into(),
            cwd: String::new(),
            url: String::new(),
            editor: text_editor::Content::new(),
            json: false,
            saving: false,
            error: None,
            presets: vec![],
        }
    }
    pub(crate) fn connection(&self) -> Result<McpConnectionDefinition, String> {
        let connection = if self.json {
            parse(&self.editor.text())?
        } else {
            let values: BTreeMap<String, McpConfigValue> = parse(&self.values)?;
            match self.transport.as_str() {
                "stdio" => McpConnectionDefinition::Stdio {
                    command: self.command.clone(),
                    args: parse(&self.args)?,
                    env: values,
                    cwd: (!self.cwd.is_empty()).then(|| self.cwd.clone()),
                },
                "streamable_http" => McpConnectionDefinition::StreamableHttp {
                    url: self.url.clone(),
                    headers: values,
                },
                "sse" => McpConnectionDefinition::Sse { url: self.url.clone(), headers: values },
                _ => return Err("不支持的传输".into()),
            }
        };
        let values = match &connection {
            McpConnectionDefinition::Stdio { env, .. } => env,
            McpConnectionDefinition::StreamableHttp { headers, .. }
            | McpConnectionDefinition::Sse { headers, .. } => headers,
        };
        if values.values().any(|value| match value {
            McpConfigValue::Literal { value } => value.trim() == "••••••••",
            McpConfigValue::Credential { reference } => reference.account_id.trim() == "••••••••",
        }) {
            return Err("未知凭据遮罩，请填写凭据引用或省略已有凭据字段".into());
        }
        Ok(connection)
    }
    pub(crate) fn edit_values(&mut self, event: FormMessage) -> Result<(), String> {
        use vw_gateway_client::vw_api_types::agent_apps_auth::AuthService;
        let mut values: BTreeMap<String, McpConfigValue> = parse(&self.values)?;
        match event {
            FormMessage::ValueAdd => {
                let mut index = 1;
                while values.contains_key(&format!("KEY_{index}")) {
                    index += 1;
                }
                values.insert(
                    format!("KEY_{index}"),
                    McpConfigValue::Literal { value: String::new() },
                );
            }
            FormMessage::ValueRemove(name) => {
                values.remove(&name);
            }
            FormMessage::ValueName(old, new) => {
                if old != new && values.contains_key(&new) {
                    return Err("字段名已存在".into());
                }
                if let Some(value) = values.remove(&old) {
                    values.insert(new, value);
                }
            }
            FormMessage::ValueKind(name, credential) => {
                values.insert(
                    name,
                    if credential {
                        McpConfigValue::Credential {
                            reference: McpCredentialRef {
                                service: AuthService::CodexOauth,
                                account_id: String::new(),
                            },
                        }
                    } else {
                        McpConfigValue::Literal { value: String::new() }
                    },
                );
            }
            FormMessage::ValueText(name, text) => {
                if let Some(value) = values.get_mut(&name) {
                    match value {
                        McpConfigValue::Literal { value } => *value = text,
                        McpConfigValue::Credential { reference } => reference.account_id = text,
                    }
                }
            }
            FormMessage::ValueService(name, service) => {
                if let Some(McpConfigValue::Credential { reference }) = values.get_mut(&name) {
                    reference.service = serde_json::from_value(serde_json::Value::String(service))
                        .map_err(|_| "不支持的凭据服务")?;
                }
            }
            _ => return Ok(()),
        }
        self.values = serde_json::to_string(&values).expect("DTO");
        Ok(())
    }
    fn set_connection(&mut self, connection: McpConnectionDefinition) {
        match connection {
            McpConnectionDefinition::Stdio { command, args, env, cwd } => {
                self.transport = "stdio".into();
                self.command = command;
                self.args = serde_json::to_string(&args).expect("strings");
                self.values = serde_json::to_string(&env).expect("DTO");
                self.cwd = cwd.unwrap_or_default();
            }
            McpConnectionDefinition::StreamableHttp { url, headers } => {
                self.transport = "streamable_http".into();
                self.url = url;
                self.values = serde_json::to_string(&headers).expect("DTO");
            }
            McpConnectionDefinition::Sse { url, headers } => {
                self.transport = "sse".into();
                self.url = url;
                self.values = serde_json::to_string(&headers).expect("DTO");
            }
        }
    }
    fn toggle_json(&mut self, json: bool) -> Result<(), String> {
        if json != self.json {
            let connection = self.connection()?;
            if json {
                self.editor = text_editor::Content::with_text(
                    &serde_json::to_string_pretty(&connection).expect("DTO"),
                );
            } else {
                self.set_connection(connection);
            }
            self.json = json;
        }
        Ok(())
    }
    fn preset(&mut self, index: usize) {
        if let Some(preset) = self.presets.get(index).cloned() {
            self.id = preset.id;
            self.name = preset.name;
            self.set_connection(preset.connection);
            self.json = false;
            self.error = None;
            // Saving creates an unbound catalog definition; presets never enable consumers.
        }
    }
    fn finish(&mut self, result: &Result<McpCatalogRecord, String>) {
        self.saving = false;
        self.error = result.as_ref().err().cloned();
    }
}

pub(crate) async fn save(
    client: &GatewayClient,
    token: &str,
    draft: Draft,
) -> Result<McpCatalogRecord, String> {
    let connection = draft.connection()?;
    let project_id = match draft.record.as_ref().map(|r| &r.scope) {
        Some(McpCatalogScope::Project { scope }) => Some(scope.project_id().clone()),
        _ => None,
    };
    let result = if let Some(record) = &draft.record {
        client
            .mcp_management_update(
                &record.record.id,
                &McpUpdateRequest {
                    target_host_id: draft.context.host.clone(),
                    project_id,
                    expected_revision: record.record.revision,
                    connection,
                },
                token,
            )
            .await
    } else {
        client
            .mcp_management_create(
                &McpCreateRequest {
                    target_host_id: draft.context.host.clone(),
                    project_id: None,
                    id: draft.id.clone(),
                    name: Some(draft.name.clone()),
                    description: None,
                    homepage: None,
                    documentation: None,
                    tags: vec![],
                    connection,
                },
                token,
            )
            .await
    };
    result.map_err(|error| match error {
        vw_gateway_client::AgentAppsSettingsError::Http { status: 409, .. } => {
            "目录或修订冲突，请核对刷新后的目录再编辑".into()
        }
        vw_gateway_client::AgentAppsSettingsError::Http { status: 400 | 422, .. } => {
            "连接定义无效，请检查字段及凭据引用".into()
        }
        _ => "保存失败，请检查网关连接和权限".into(),
    })
}
fn authorized(app: &App, context: &catalog::Context) -> bool {
    catalog::context(app).as_ref() == Some(context)
        && !context.host.is_empty()
        && !app.apps_catalog.token.is_empty()
        && app.apps_catalog.context.as_ref() == Some(&context.endpoint)
}
pub(crate) fn update(app: &mut App, event: FormMessage) -> Task<Message> {
    match event {
        FormMessage::Open(record) => {
            if app.mcp_settings.connection_form.is_some() {
                return Task::none();
            }
            let Some(context) = catalog::context(app).filter(|context| authorized(app, context))
            else {
                return app.show_error_toast("请先在应用管理中连接目标主机");
            };
            if let Some(record) = &record {
                let current = catalog::visible(app).and_then(|r| r.as_ref().ok());
                if !current.is_some_and(|c| c.records.contains(record)) {
                    return app.show_error_toast("目录已变化，请刷新");
                }
            }
            let new = record.is_none();
            app.mcp_settings.connection_form = Some(Draft::new(context.clone(), record));
            if new && let Ok(client) = crate::app::config::gateway_client() {
                let token = app.apps_catalog.token.clone();
                let generation = app.mcp_settings.connection_form.as_ref().unwrap().generation;
                return Task::perform(
                    async move {
                        let result = client
                            .mcp_management_presets(&context.host, &token)
                            .await
                            .map_err(|_| "预设读取失败，可手动编辑".into());
                        FormMessage::Presets(context, generation, result)
                    },
                    message,
                );
            }
        }
        FormMessage::Saved(context, result) => {
            let current = authorized(app, &context);
            let Some(draft) = app
                .mcp_settings
                .connection_form
                .as_mut()
                .filter(|d| d.context == context && d.saving)
            else {
                return Task::none();
            };
            if !current {
                draft.finish(&Err("连接上下文已变化，请关闭后重新编辑".into()));
                return catalog::update(app, catalog::CatalogMessage::Refresh);
            }
            draft.finish(&result);
            let toast = match result {
                Ok(_) => {
                    app.mcp_settings.connection_form = None;
                    app.show_success_toast("定义已保存；尚未分发或连接")
                }
                Err(error) => app.show_error_toast(error),
            };
            return Task::batch([toast, catalog::update(app, catalog::CatalogMessage::Refresh)]);
        }
        FormMessage::Save => {
            let Some(draft) = app.mcp_settings.connection_form.as_ref() else {
                return Task::none();
            };
            if draft.saving {
                return Task::none();
            }
            if !authorized(app, &draft.context) {
                return app.show_error_toast("连接上下文已变化，请关闭后重新编辑");
            }
            if let Err(error) = draft.connection() {
                app.mcp_settings.connection_form.as_mut().unwrap().error = Some(error);
                return Task::none();
            }
            let Ok(client) = crate::app::config::gateway_client() else {
                return app.show_error_toast("网关不可用");
            };
            let draft = app.mcp_settings.connection_form.as_mut().unwrap();
            draft.saving = true;
            draft.error = None;
            let snapshot = draft.clone();
            let token = app.apps_catalog.token.clone();
            return Task::perform(
                async move {
                    let context = snapshot.context.clone();
                    let result = save(&client, &token, snapshot).await;
                    FormMessage::Saved(context, result)
                },
                message,
            );
        }
        event => {
            let Some(draft) = app.mcp_settings.connection_form.as_mut() else {
                return Task::none();
            };
            if draft.saving {
                return Task::none();
            }
            match event {
                FormMessage::Close => app.mcp_settings.connection_form = None,
                FormMessage::Field(field, value) => {
                    if field == "transport" && (draft.transport == "stdio") != (value == "stdio") {
                        draft.values = "{}".into();
                    }
                    let target = match field {
                        "id" => &mut draft.id,
                        "name" => &mut draft.name,
                        "command" => &mut draft.command,
                        "args" => &mut draft.args,
                        "cwd" => &mut draft.cwd,
                        "url" => &mut draft.url,
                        "transport" => &mut draft.transport,
                        _ => return Task::none(),
                    };
                    *target = value;
                }
                event @ (FormMessage::ValueAdd
                | FormMessage::ValueRemove(_)
                | FormMessage::ValueName(_, _)
                | FormMessage::ValueKind(_, _)
                | FormMessage::ValueText(_, _)
                | FormMessage::ValueService(_, _)) => {
                    draft.error = draft.edit_values(event).err();
                }
                FormMessage::Editor(action) => draft.editor.perform(action),
                FormMessage::Json(json) => {
                    draft.error = draft.toggle_json(json).err();
                }
                FormMessage::Preset(index) => {
                    if draft.record.is_none() {
                        draft.preset(index);
                    }
                }
                FormMessage::Presets(context, generation, result)
                    if draft.context == context && draft.generation == generation =>
                {
                    match result {
                        Ok(response) => draft.presets = response.presets,
                        Err(error) => draft.error = Some(error),
                    }
                }
                _ => {}
            }
        }
    }
    Task::none()
}
#[cfg(test)]
#[path = "mcp_management_form_tests.rs"]
mod tests;

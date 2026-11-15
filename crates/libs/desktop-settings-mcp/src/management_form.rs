use iced::widget::text_editor;
use std::collections::BTreeMap;
use vw_api_mcp_management::*;
use vw_gateway_client::GatewayClient;
#[derive(Clone)]
pub struct Draft {
    pub context: crate::management_context::Context,
    pub generation: u64,
    pub record: Option<McpCatalogRecord>,
    pub id: String,
    pub name: String,
    pub transport: String,
    pub command: String,
    pub args: String,
    pub values: String,
    pub cwd: String,
    pub url: String,
    pub editor: text_editor::Content,
    pub json: bool,
    pub saving: bool,
    pub error: Option<String>,
    pub presets: Vec<McpPresetDraftView>,
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
    Presets(crate::management_context::Context, u64, Result<McpPresetsResponse, String>),
    Preset(usize),
    ValueAdd,
    ValueRemove(String),
    ValueName(String, String),
    ValueKind(String, bool),
    ValueText(String, String),
    ValueService(String, String),
    Save,
    Saved(crate::management_context::Context, Result<McpCatalogRecord, String>),
}
impl std::fmt::Debug for FormMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("McpConnectionFormMessage { .. }")
    }
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
    pub fn new(
        context: crate::management_context::Context,
        record: Option<McpCatalogRecord>,
    ) -> Self {
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
    pub fn connection(&self) -> Result<McpConnectionDefinition, String> {
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
    pub fn edit_values(&mut self, event: FormMessage) -> Result<(), String> {
        use vw_api_app_auth::AuthService;
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
    pub fn set_connection(&mut self, connection: McpConnectionDefinition) {
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
    pub fn toggle_json(&mut self, json: bool) -> Result<(), String> {
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
    pub fn preset(&mut self, index: usize) {
        if let Some(preset) = self.presets.get(index).cloned() {
            self.id = preset.id;
            self.name = preset.name;
            self.set_connection(preset.connection);
            self.json = false;
            self.error = None;
            // Saving creates an unbound catalog definition; presets never enable consumers.
        }
    }
    pub fn finish(&mut self, result: &Result<McpCatalogRecord, String>) {
        self.saving = false;
        self.error = result.as_ref().err().cloned();
    }
}

pub async fn save(
    client: &GatewayClient,
    token: &str,
    draft: Draft,
) -> Result<McpCatalogRecord, String> {
    let client = gateway_mcp_write::GatewayMcpWriteClient::new(client.transport().clone());
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
        gateway_mcp_write::AgentAppsSettingsError::Http { status: 409, .. } => {
            "目录或修订冲突，请核对刷新后的目录再编辑".into()
        }
        gateway_mcp_write::AgentAppsSettingsError::Http { status: 400 | 422, .. } => {
            "连接定义无效，请检查字段及凭据引用".into()
        }
        _ => "保存失败，请检查网关连接和权限".into(),
    })
}

#[cfg(test)]
#[path = "management_form_tests.rs"]
mod tests;

impl Draft {
    pub fn edit(&mut self, event: FormMessage) {
        match event {
            FormMessage::Field(field, value) => {
                if field == "transport" && (self.transport == "stdio") != (value == "stdio") {
                    self.values = "{}".into();
                }
                let target = match field {
                    "id" => &mut self.id,
                    "name" => &mut self.name,
                    "command" => &mut self.command,
                    "args" => &mut self.args,
                    "cwd" => &mut self.cwd,
                    "url" => &mut self.url,
                    "transport" => &mut self.transport,
                    _ => return,
                };
                *target = value;
            }
            event @ (FormMessage::ValueAdd
            | FormMessage::ValueRemove(_)
            | FormMessage::ValueName(_, _)
            | FormMessage::ValueKind(_, _)
            | FormMessage::ValueText(_, _)
            | FormMessage::ValueService(_, _)) => {
                self.error = self.edit_values(event).err();
            }
            FormMessage::Editor(action) => self.editor.perform(action),
            FormMessage::Json(json) => {
                self.error = self.toggle_json(json).err();
            }
            FormMessage::Preset(index) => {
                if self.record.is_none() {
                    self.preset(index);
                }
            }
            _ => {}
        }
    }
}

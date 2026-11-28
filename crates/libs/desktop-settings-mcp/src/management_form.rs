#[path = "management_form_global.rs"]
mod global;
#[path = "management_form_paste.rs"]
pub mod paste;

use crate::management_context::Context;
use gateway_mcp::global_sync::{Options, SyncItem};
use iced::widget::text_editor;
use serde_json::Value;
use std::collections::BTreeMap;
use vw_api_app_identity::AppId;
use vw_api_mcp_management::{
    McpAppCapability, McpConfigValue, McpConnectionDefinition, McpPresetDraftView, McpSupport,
};
use vw_gateway_client::GatewayClient;

#[derive(Clone)]
pub struct Draft {
    pub context: Context,
    pub(crate) global: global::GlobalDraft,
    pub sync_only: bool,
    pub generation: u64,
    pub id: String,
    pub transport: String,
    pub command: String,
    pub args: String,
    pub values: String,
    pub cwd: String,
    pub url: String,
    pub editor: text_editor::Content,
    pub json: bool,
    pub saving: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub presets: Vec<McpPresetDraftView>,
    pub paste: text_editor::Content,
    pub candidates: Vec<paste::Candidate>,
    pub targets: Vec<(McpAppCapability, bool)>,
    pub deliveries: Vec<SyncItem>,
    pub retry: Vec<AppId>,
}
impl std::fmt::Debug for Draft {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpConnectionDraft").finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub enum FormMessage {
    OpenGlobal(Option<usize>, bool),
    OpenLink(Vec<paste::Candidate>),
    GlobalEnabled(bool),
    GlobalRequired(bool),
    OptionsLoaded(Context, u64, Result<std::sync::Arc<Options>, String>),
    Completed {
        context: Context,
        generation: u64,
        written: Option<Value>,
        requested: Vec<AppId>,
        result: Result<Vec<SyncItem>, String>,
    },
    Close,
    Field(&'static str, String),
    Json(bool),
    Editor(text_editor::Action),
    Preset(usize),
    ValueAdd,
    ValueRemove(String),
    ValueName(String, String),
    ValueText(String, String),
    ArgAdd,
    ArgRemove(usize),
    ArgSet(usize, String),
    PasteEditor(text_editor::Action),
    ParsePaste,
    ChoosePaste(usize),
    SkipPaste(usize),
    Target(AppId, bool),
    Retry,
    Save,
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
    pub fn new(context: Context) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            context,
            global: global::GlobalDraft::default(),
            sync_only: false,
            generation: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            id: String::new(),
            transport: "stdio".into(),
            command: String::new(),
            args: "[]".into(),
            values: "{}".into(),
            cwd: String::new(),
            url: String::new(),
            editor: text_editor::Content::new(),
            json: false,
            saving: false,
            loading: false,
            error: None,
            presets: vec![],
            paste: text_editor::Content::new(),
            candidates: vec![],
            targets: vec![],
            deliveries: vec![],
            retry: vec![],
        }
    }

    pub fn open(
        context: Context,
        page: &crate::state::State,
        index: Option<usize>,
        sync_only: bool,
    ) -> Result<Self, String> {
        let mut draft = Self::new(context);
        global::open(&mut draft, page, index)?;
        draft.sync_only = sync_only;
        Ok(draft)
    }

    pub fn editing(&self) -> bool {
        self.global.original.is_some()
    }

    pub fn set_options(&mut self, options: &Options) {
        self.presets = options.presets.clone();
        self.targets = options.applications.iter().cloned().map(|app| (app, false)).collect();
    }

    pub fn save_value(&self) -> Result<Value, String> {
        global::server(self, &self.global)
    }

    pub fn original(&self) -> Option<&Value> {
        self.global.original.as_ref()
    }

    pub fn mark_saved(&mut self, server: Value) {
        self.global.original = Some(server);
    }

    pub fn next_candidate(self) -> Option<Self> {
        if self.candidates.is_empty() {
            return None;
        }
        let mut next = Self::new(self.context);
        next.candidates = self.candidates;
        next.presets = self.presets;
        next.targets = self.targets.into_iter().map(|(app, _)| (app, false)).collect();
        Some(next)
    }

    pub fn target_supported(&self, info: &McpAppCapability) -> bool {
        let transport = if self.json {
            match self.connection() {
                Ok(McpConnectionDefinition::Stdio { .. }) => "stdio",
                Ok(McpConnectionDefinition::StreamableHttp { .. }) => "streamable_http",
                Ok(McpConnectionDefinition::Sse { .. }) => "sse",
                Err(_) => return false,
            }
        } else {
            self.transport.as_str()
        };
        let support = match transport {
            "stdio" => info.stdio,
            "streamable_http" => info.streamable_http,
            "sse" => info.sse,
            _ => return false,
        };
        support == McpSupport::Supported
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
            FormMessage::ValueText(name, text) => {
                if let Some(value) = values.get_mut(&name) {
                    match value {
                        McpConfigValue::Literal { value } => *value = text,
                        McpConfigValue::Credential { reference } => reference.account_id = text,
                    }
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
                if parse::<McpConnectionDefinition>(&self.editor.text()).ok().as_ref()
                    != Some(&connection)
                {
                    self.editor = text_editor::Content::with_text(
                        &serde_json::to_string_pretty(&connection).expect("DTO"),
                    );
                }
            } else {
                self.set_connection(connection);
            }
            self.json = json;
        }
        Ok(())
    }
    pub fn edit(&mut self, event: FormMessage) {
        match event {
            FormMessage::GlobalEnabled(value) => self.global.enabled = value,
            FormMessage::GlobalRequired(value) => self.global.required = value,
            FormMessage::Field(field, value) => {
                if field == "transport" && (self.transport == "stdio") != (value == "stdio") {
                    self.values = "{}".into();
                }
                let target = match field {
                    "id" if !self.editing() => &mut self.id,
                    "timeout" => &mut self.global.timeout,
                    "command" => &mut self.command,
                    "args" => &mut self.args,
                    "url" => &mut self.url,
                    "transport" => &mut self.transport,
                    _ => return,
                };
                *target = value;
            }
            event @ (FormMessage::ValueAdd
            | FormMessage::ValueRemove(_)
            | FormMessage::ValueName(..)
            | FormMessage::ValueText(..)) => {
                self.error = self.edit_values(event).err();
            }
            FormMessage::ArgAdd | FormMessage::ArgRemove(_) | FormMessage::ArgSet(..) => {
                let mut args: Vec<String> = parse(&self.args).unwrap_or_default();
                match event {
                    FormMessage::ArgAdd => args.push(String::new()),
                    FormMessage::ArgRemove(index) if index < args.len() => {
                        args.remove(index);
                    }
                    FormMessage::ArgSet(index, value) if index < args.len() => args[index] = value,
                    _ => {}
                }
                self.args = serde_json::to_string(&args).expect("strings");
            }
            FormMessage::PasteEditor(action) => {
                self.paste.perform(action);
                self.candidates.clear();
            }
            FormMessage::ParsePaste => match paste::parse(&self.paste.text()) {
                Ok(items) => {
                    self.candidates = items;
                    self.error = None;
                    if self.candidates.len() == 1 {
                        self.edit(FormMessage::ChoosePaste(0));
                    }
                }
                Err(error) => self.error = Some(error),
            },
            FormMessage::SkipPaste(index) => {
                if index < self.candidates.len() {
                    self.candidates.remove(index);
                }
            }
            FormMessage::ChoosePaste(index) => {
                if let Some(candidate) = self.candidates.get(index).cloned() {
                    match candidate.connection {
                        Ok(connection) => {
                            if self.editing() {
                                self.candidates.clear();
                            } else {
                                self.id = candidate.id;
                                self.candidates.remove(index);
                            }
                            self.set_connection(connection);
                            self.json = false;
                            self.error = None;
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
            }
            FormMessage::Target(id, selected) => {
                if let Some(index) = self.targets.iter().position(|(app, _)| app.app_id == id) {
                    if !selected || self.target_supported(&self.targets[index].0) {
                        self.targets[index].1 = selected;
                    }
                }
            }
            FormMessage::Editor(action) => self.editor.perform(action),
            FormMessage::Json(value) => self.error = self.toggle_json(value).err(),
            FormMessage::Preset(index) => {
                if let Some(preset) = self.presets.get(index).cloned() {
                    if !self.editing() {
                        self.id = preset.id;
                        self.global.enabled = preset.enabled;
                    }
                    self.set_connection(preset.connection);
                    self.json = false;
                    self.error = None;
                }
            }
            _ => {}
        }
    }
}

pub async fn persist(
    client: &GatewayClient,
    original: Option<&Value>,
    server: Option<&Value>,
) -> Result<(), String> {
    global::persist(client, original, server).await
}

#[cfg(test)]
#[path = "management_form_tests.rs"]
mod tests;

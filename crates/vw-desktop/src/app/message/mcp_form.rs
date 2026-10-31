//! Ephemeral forms. Every edit and reply remains bound to its original run and gateway.
use crate::app::{App, Message};
use iced::Task;
use serde_json::{Map, Value};
use std::{collections::HashMap, sync::Arc};
use vw_gateway_client::{
    GatewayClient,
    vw_api_types::mcp_form::{McpFormAction, McpFormHostReply, McpFormRequest, McpFormResponse},
};

#[derive(Clone)]
pub(crate) struct FormScope {
    pub run_id: u64,
    pub session: String,
    pub host_id: String,
    pub client: Arc<GatewayClient>,
}
impl std::fmt::Debug for FormScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormScope").field("run_id", &self.run_id).finish_non_exhaustive()
    }
}
impl FormScope {
    fn matches(&self, other: &Self) -> bool {
        self.run_id == other.run_id
            && self.session == other.session
            && self.host_id == other.host_id
    }
}
#[derive(Clone)]
pub(crate) enum McpFormMessage {
    Pending { scope: FormScope, requests: Vec<McpFormRequest> },
    Closed(FormScope),
    Unavailable { run_id: u64, session: String },
    Changed { id: String, field: String, value: String },
    Submit { id: String, action: McpFormAction },
    Replied { scope: FormScope, id: String, result: Result<bool, String> },
}
impl std::fmt::Debug for McpFormMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // User values, host capabilities and remote metadata must not enter debug logs.
        f.write_str("McpFormMessage { .. }")
    }
}
#[derive(Debug, Clone)]
pub(crate) struct Field {
    pub name: String,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub required: bool,
    pub options: Vec<(String, String)>,
    pub hint: String,
}
pub(crate) struct Draft {
    pub scope: FormScope,
    pub fields: Vec<Field>,
    pub values: HashMap<String, String>,
    pub error: Option<String>,
    pub busy: bool,
}
#[derive(Default)]
pub(crate) struct McpFormState {
    pub requests: Vec<McpFormRequest>,
    pub drafts: HashMap<String, Draft>,
    pub error: Option<String>,
}
fn fields(schema: &Value) -> Result<Vec<Field>, String> {
    let invalid = || "此表单包含暂不支持的字段，请取消后联系服务提供方。".to_string();
    if schema.get("type").and_then(Value::as_str) != Some("object") {
        return Err(invalid());
    }
    let properties = schema.get("properties").and_then(Value::as_object).ok_or_else(invalid)?;
    if properties.len() > 64 {
        return Err(invalid());
    }
    let required = schema.get("required").and_then(Value::as_array);
    properties
        .iter()
        .map(|(name, value)| {
            let kind = value.get("type").and_then(Value::as_str).ok_or_else(invalid)?;
            if !matches!(kind, "string" | "number" | "integer" | "boolean") {
                return Err(invalid());
            }
            let mut options = Vec::new();
            if kind == "boolean" {
                options = vec![("true".into(), "是".into()), ("false".into(), "否".into())];
            }
            if let Some(choices) = value.get("enum") {
                let choices = choices
                    .as_array()
                    .filter(|c| !c.is_empty() && c.len() <= 64)
                    .ok_or_else(invalid)?;
                let labels = value.get("enumNames").and_then(Value::as_array);
                for (i, choice) in choices.iter().enumerate() {
                    let choice = choice.as_str().ok_or_else(invalid)?.to_owned();
                    let label = labels
                        .and_then(|labels| labels.get(i))
                        .and_then(Value::as_str)
                        .unwrap_or(&choice)
                        .to_owned();
                    options.push((choice, label));
                }
            }
            Ok(Field {
                name: name.clone(),
                title: value.get("title").and_then(Value::as_str).unwrap_or(name).to_owned(),
                description: value
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                kind: kind.to_owned(),
                required: required.is_some_and(|r| r.iter().any(|v| v == name)),
                options,
                hint: value
                    .get("default")
                    .map(|v| {
                        format!(
                            "建议值：{}（需自行确认）",
                            v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string())
                        )
                    })
                    .unwrap_or_default(),
            })
        })
        .collect()
}
pub(crate) fn current(app: &App) -> Option<(&McpFormRequest, &Draft)> {
    app.mcp_forms.requests.iter().find_map(|request| {
        let draft = app.mcp_forms.drafts.get(&request.id)?;
        (app.active_session_id.as_deref() == Some(request.session_id.as_str())
            && live(app, &draft.scope))
        .then_some((request, draft))
    })
}
fn live(app: &App, scope: &FormScope) -> bool {
    app.session_runtime_states.get(&scope.session).is_some_and(|runtime| {
        runtime.is_requesting
            && runtime
                .active_agent_request
                .as_ref()
                .is_some_and(|request| request.id == scope.run_id)
    })
}
pub(crate) fn prune(app: &mut App) {
    let ids: Vec<String> = app
        .mcp_forms
        .drafts
        .iter()
        .filter(|(_, draft)| !live(app, &draft.scope))
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids {
        app.mcp_forms.drafts.remove(&id);
        app.mcp_forms.requests.retain(|r| r.id != id);
    }
}
fn remove_scope(app: &mut App, scope: &FormScope) {
    let ids: Vec<_> = app
        .mcp_forms
        .drafts
        .iter()
        .filter(|(_, d)| d.scope.matches(scope))
        .map(|(id, _)| id.clone())
        .collect();
    app.mcp_forms.requests.retain(|r| !ids.contains(&r.id));
    for id in ids {
        app.mcp_forms.drafts.remove(&id);
    }
}
pub(crate) fn response(draft: &Draft, action: McpFormAction) -> Result<McpFormResponse, String> {
    if action != McpFormAction::Accept {
        return Ok(McpFormResponse { action, content: None });
    }
    let mut content = Map::new();
    for field in &draft.fields {
        let Some(value) = draft.values.get(&field.name) else {
            if field.required {
                return Err("请填写全部必填项；默认值不会自动提交。".into());
            }
            continue;
        };
        let value = match field.kind.as_str() {
            "string" => Value::String(value.clone()),
            "boolean" => match value.as_str() {
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                _ => return Err("请选择是或否。".into()),
            },
            "integer" | "number" => {
                let number: Value =
                    serde_json::from_str(value).map_err(|_| "请输入有效数字。".to_string())?;
                if !number.is_number()
                    || (field.kind == "integer" && number.as_f64().is_none_or(|v| v.fract() != 0.0))
                {
                    return Err("请输入有效的数字或整数。".into());
                }
                number
            }
            _ => return Err("暂不支持此字段。".into()),
        };
        if !field.options.is_empty()
            && !field.options.iter().any(|(raw, _)| {
                value.as_str().map(str::to_owned).unwrap_or_else(|| value.to_string()) == *raw
            })
        {
            return Err("请选择列表中的选项。".into());
        }
        content.insert(field.name.clone(), value);
    }
    Ok(McpFormResponse { action, content: Some(Value::Object(content)) })
}
pub(crate) fn update(app: &mut App, message: McpFormMessage) -> Task<Message> {
    match message {
        McpFormMessage::Pending { scope, requests } => {
            if !live(app, &scope)
                || requests.len() > 16
                || requests.iter().any(|r| {
                    r.session_id != scope.session
                        || app.mcp_forms.requests.iter().any(|old| old.id == r.id && old != r)
                })
            {
                return Task::none();
            }
            let incoming: Vec<_> = requests.iter().map(|r| r.id.clone()).collect();
            let finished: Vec<_> = app
                .mcp_forms
                .drafts
                .iter()
                .filter(|(id, d)| d.scope.matches(&scope) && !incoming.contains(id))
                .map(|(id, _)| id.clone())
                .collect();
            for id in finished {
                app.mcp_forms.drafts.remove(&id);
                app.mcp_forms.requests.retain(|r| r.id != id);
            }
            for request in requests {
                if app.mcp_forms.drafts.contains_key(&request.id) {
                    continue;
                }
                match fields(&request.requested_schema) {
                    Ok(fields) => {
                        app.mcp_forms.drafts.insert(
                            request.id.clone(),
                            Draft {
                                scope: scope.clone(),
                                fields,
                                values: HashMap::new(),
                                error: None,
                                busy: false,
                            },
                        );
                        app.mcp_forms.requests.push(request);
                    }
                    Err(error) => {
                        app.mcp_forms.error = Some(error.clone());
                        return app.show_error_toast(error);
                    }
                }
            }
        }
        McpFormMessage::Closed(scope) => remove_scope(app, &scope),
        McpFormMessage::Unavailable { run_id, session } => {
            if app
                .session_runtime_states
                .get(&session)
                .and_then(|r| r.active_agent_request.as_ref())
                .is_some_and(|r| r.id == run_id)
            {
                return app.show_warning_toast("MCP 表单交互暂不可用，未授予任何确认。");
            }
        }
        McpFormMessage::Changed { id, field, value } => {
            if current(app).is_none_or(|(r, d)| r.id != id || d.busy) {
                return Task::none();
            }
            if let Some(draft) = app.mcp_forms.drafts.get_mut(&id)
                && value.len() <= 65_536
                && draft.fields.iter().any(|f| f.name == field)
            {
                draft.values.insert(field, value);
                draft.error = None;
            }
        }
        McpFormMessage::Submit { id, action } => {
            if current(app).is_none_or(|(r, d)| r.id != id || d.busy) {
                return Task::none();
            }
            let request = app
                .mcp_forms
                .requests
                .iter()
                .find(|r| r.id == id)
                .cloned()
                .expect("current form exists");
            let draft = app.mcp_forms.drafts.get_mut(&id).expect("current draft exists");
            let response = match response(draft, action) {
                Ok(r) => r,
                Err(error) => {
                    draft.error = Some(error.clone());
                    return app.show_error_toast(error);
                }
            };
            draft.busy = true;
            let scope = draft.scope.clone();
            let body = McpFormHostReply { host_id: scope.host_id.clone(), request, response };
            return Task::perform(
                async move {
                    let result = scope.client.mcp_form_reply(&body).await;
                    (scope, id, result)
                },
                |(scope, id, result)| {
                    Message::McpForm(McpFormMessage::Replied { scope, id, result })
                },
            );
        }
        McpFormMessage::Replied { scope, id, result } => {
            let Some(draft) = app.mcp_forms.drafts.get_mut(&id).filter(|d| d.scope.matches(&scope))
            else {
                return Task::none();
            };
            if matches!(result, Ok(true)) {
                app.mcp_forms.requests.retain(|r| r.id != id);
                app.mcp_forms.drafts.remove(&id);
            } else {
                draft.busy = false;
                draft.error = Some("表单值无效或请求已过期，请检查后重试。".into());
                return app.show_error_toast("表单值无效或请求已过期，请检查后重试。");
            }
        }
    }
    Task::none()
}

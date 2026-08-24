//! TUI Workflow 流请求、归属数据与阻塞传输边界。

use std::collections::BTreeMap;
use std::fmt;

use serde_json::{Value, json};
use vw_gateway_client::vw_api_types::workflow::{
    WorkflowPauseDto, WorkflowResumeRequest, WorkflowRunResponse, WorkflowRunStatus,
};
use vw_gateway_client::{
    GatewayStreamAttemptResult, GatewayWorkflowResponseMode, GatewayWorkflowStreamEvent,
    GatewayWorkflowStreamRequest,
};

use crate::cli::tui::model::UiQuestionOverlay;
use crate::cli::tui::model::overlay::workflow_action_label;

use super::gateway::{GatewayUiRuntime, block_on_gateway};
use super::orchestration::OrchestrationSnapshot;
use super::stream_adapter::{
    UiRuntimeEvent, UiRuntimeTerminalEvent, adapt_gateway_workflow_stream_event,
};

/// 已通过当前 Workflow snapshot 校验的运行请求。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OwnedWorkflowRunRequest {
    pub(crate) uuid: String,
    pub(crate) body: GatewayWorkflowStreamRequest,
}

/// 构造仅指向当前目录已加载 Workflow 的流式请求。
pub(crate) fn workflow_run_request(
    snapshot: &OrchestrationSnapshot,
    session_id: Option<&str>,
    uuid: &str,
    query: &str,
) -> Result<OwnedWorkflowRunRequest, String> {
    let uuid = uuid.trim();
    if !snapshot.workflows.items.iter().any(|item| item.uuid == uuid) {
        return Err(
            "workflow run requires a UUID from the current Workflow snapshot".to_string()
        );
    }
    let query = query.trim();
    if query.is_empty() {
        return Err("workflow run requires a non-empty query".to_string());
    }
    let user = session_id
        .map(str::trim)
        .filter(|session_id| !session_id.is_empty())
        .ok_or_else(|| "workflow run requires a stable current session id".to_string())?;

    Ok(OwnedWorkflowRunRequest {
        uuid: uuid.to_string(),
        body: GatewayWorkflowStreamRequest {
            query: query.to_string(),
            inputs: BTreeMap::new(),
            response_mode: GatewayWorkflowResponseMode::Streaming,
            user: user.to_string(),
            conversation_id: None,
        },
    })
}

/// 在后台线程中运行真实 Workflow SSE，并保留调用方提前停止语义。
pub(crate) fn stream_workflow_run_blocking(
    runtime: &GatewayUiRuntime,
    request: &OwnedWorkflowRunRequest,
    mut on_event: impl FnMut(GatewayWorkflowStreamEvent) -> bool,
) -> Result<GatewayStreamAttemptResult, String> {
    let mut caller_stopped = false;
    block_on_gateway(async {
        runtime
            .client()
            .stream_workflow_application(request.uuid.as_str(), &request.body, |event| {
                let keep_streaming = on_event(event);
                caller_stopped |= !keep_streaming;
                keep_streaming
            })
            .await
    })?;
    Ok(if caller_stopped {
        GatewayStreamAttemptResult::CallerStopped
    } else {
        GatewayStreamAttemptResult::Terminated
    })
}

/// 使用原始 pause token 恢复同一个 Workflow run；请求体不会进入 UI 状态。
pub(crate) fn resume_workflow_run_blocking(
    runtime: &GatewayUiRuntime,
    request: &WorkflowResumeRequest,
) -> Result<WorkflowRunResponse, String> {
    block_on_gateway(async { runtime.client().workflow_resume(request).await })
}

/// 独立于 chat/plan-run 的 Workflow 请求、运行、会话与暂停归属。
#[derive(Clone, PartialEq)]
pub(crate) struct WorkflowRunOwnership {
    request_id: String,
    workflow_uuid: String,
    session_id: String,
    task_id: Option<String>,
    workflow_run_id: Option<String>,
    conversation_id: Option<String>,
    pause: Option<WorkflowPauseDto>,
    pause_generation: u64,
    terminal_events: usize,
    saw_text_chunk: bool,
}

impl fmt::Debug for WorkflowRunOwnership {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorkflowRunOwnership")
            .field("request_id", &self.request_id)
            .field("workflow_uuid", &self.workflow_uuid)
            .field("session_id", &self.session_id)
            .field("task_id", &self.task_id)
            .field("workflow_run_id", &self.workflow_run_id)
            .field("conversation_id", &self.conversation_id)
            .field("has_pause", &self.pause.is_some())
            .field("pause_generation", &self.pause_generation)
            .field("terminal_events", &self.terminal_events)
            .field("saw_text_chunk", &self.saw_text_chunk)
            .finish()
    }
}

impl WorkflowRunOwnership {
    pub(crate) fn new_for_session(
        request_id: String,
        workflow_uuid: String,
        session_id: String,
    ) -> Self {
        Self {
            request_id,
            workflow_uuid,
            session_id,
            task_id: None,
            workflow_run_id: None,
            conversation_id: None,
            pause: None,
            pause_generation: 0,
            terminal_events: 0,
            saw_text_chunk: false,
        }
    }

    /// 更新稳定归属字段，并保证一个 Workflow stream 只投影一个终态。
    pub(crate) fn project_event(
        &mut self,
        event: GatewayWorkflowStreamEvent,
    ) -> Vec<UiRuntimeEvent> {
        self.observe_event(&event);
        let suppress_answer = self.saw_text_chunk
            && matches!(&event, GatewayWorkflowStreamEvent::Message { .. });
        if matches!(&event, GatewayWorkflowStreamEvent::TextChunk { text, .. } if !text.is_empty()) {
            self.saw_text_chunk = true;
        }
        let mut events = if suppress_answer {
            Vec::new()
        } else {
            adapt_gateway_workflow_stream_event(event)
        };
        events.retain(|event| {
            if !matches!(event, UiRuntimeEvent::Terminal(_)) {
                return true;
            }
            if self.terminal_events > 0 {
                return false;
            }
            self.terminal_events += 1;
            true
        });
        events
    }

    fn observe_event(&mut self, event: &GatewayWorkflowStreamEvent) {
        match event {
            GatewayWorkflowStreamEvent::WorkflowStarted {
                task_id,
                workflow_run_id,
                ..
            }
            | GatewayWorkflowStreamEvent::NodeStarted {
                task_id,
                workflow_run_id,
                ..
            }
            | GatewayWorkflowStreamEvent::NodeFinished {
                task_id,
                workflow_run_id,
                ..
            }
            | GatewayWorkflowStreamEvent::TextChunk {
                task_id,
                workflow_run_id,
                ..
            } => {
                record_once(&mut self.task_id, task_id);
                record_once(&mut self.workflow_run_id, workflow_run_id);
            }
            GatewayWorkflowStreamEvent::Message { task_id, conversation_id, .. }
            | GatewayWorkflowStreamEvent::MessageEnd { task_id, conversation_id, .. } => {
                record_once(&mut self.task_id, task_id);
                record_once(&mut self.conversation_id, conversation_id);
            }
            GatewayWorkflowStreamEvent::WorkflowFinished {
                task_id,
                workflow_run_id,
                pause,
                ..
            } => {
                record_once(&mut self.task_id, task_id);
                record_once(&mut self.workflow_run_id, workflow_run_id);
                self.pause.clone_from(pause);
                if self.pause.is_some() {
                    self.pause_generation = self.pause_generation.wrapping_add(1);
                }
            }
            GatewayWorkflowStreamEvent::Error(error) => {
                record_once(&mut self.task_id, &error.task_id);
            }
            GatewayWorkflowStreamEvent::Unknown { .. } => {}
        }
    }

    pub(crate) fn request_id(&self) -> &str {
        self.request_id.as_str()
    }

    pub(crate) fn workflow_uuid(&self) -> &str {
        self.workflow_uuid.as_str()
    }

    pub(crate) fn task_id(&self) -> Option<&str> {
        self.task_id.as_deref()
    }

    pub(crate) fn workflow_run_id(&self) -> Option<&str> {
        self.workflow_run_id.as_deref()
    }

    pub(crate) fn conversation_id(&self) -> Option<&str> {
        self.conversation_id.as_deref()
    }

    pub(crate) fn pause(&self) -> Option<&WorkflowPauseDto> {
        self.pause.as_ref()
    }

    pub(crate) fn pause_request_id(&self) -> Option<String> {
        self.pause.as_ref()?;
        self.workflow_run_id.as_ref()?;
        Some(format!("workflow-pause-{}-{}", self.request_id, self.pause_generation))
    }

    /// 返回仅含表单展示信息的 pause question，不复制恢复 token。
    pub(crate) fn pause_question(&self) -> Result<Option<UiQuestionOverlay>, String> {
        let Some(pause) = self.pause.as_ref() else {
            return Ok(None);
        };
        if self.session_id.trim().is_empty() {
            return Err("workflow pause requires its owned session id".to_string());
        }
        let request_id = self
            .pause_request_id()
            .ok_or_else(|| "workflow pause requires its owned run id".to_string())?;
        Ok(Some(UiQuestionOverlay::from_workflow_pause(
            request_id,
            self.session_id.clone(),
            pause,
        )))
    }

    /// 只允许当前会话对当前 pause 生成一次精确恢复请求。
    pub(crate) fn resume_request(
        &self,
        current_session_id: Option<&str>,
        question: &UiQuestionOverlay,
    ) -> Result<WorkflowResumeRequest, String> {
        let current_session_id = current_session_id
            .map(str::trim)
            .filter(|session_id| !session_id.is_empty())
            .ok_or_else(|| "workflow resume requires a current session id".to_string())?;
        if current_session_id != self.session_id || question.session_id != self.session_id {
            return Err("workflow pause does not belong to the current session".to_string());
        }
        let expected_request_id = self
            .pause_request_id()
            .ok_or_else(|| "workflow resume requires the current owned pause".to_string())?;
        if question.request_id != expected_request_id {
            return Err("workflow resume question is stale or externally owned".to_string());
        }
        let pause = self
            .pause
            .as_ref()
            .ok_or_else(|| "workflow resume requires the current owned pause".to_string())?;
        let expected_question = UiQuestionOverlay::from_workflow_pause(
            expected_request_id,
            self.session_id.clone(),
            pause,
        );
        if question.prompts != expected_question.prompts {
            return Err("workflow resume question shape does not match the current pause".to_string());
        }
        let run_id = self
            .workflow_run_id
            .clone()
            .ok_or_else(|| "workflow resume requires the current owned run id".to_string())?;
        let (form_values, action) = workflow_resume_values(pause, question.answers.as_slice())?;
        Ok(WorkflowResumeRequest {
            run_id,
            form_token: pause.form_token.clone(),
            form_values,
            action,
        })
    }

    /// 原子投影 resume 响应：完成/失败清理 pause，再次暂停则替换其所有权。
    pub(crate) fn project_resume_response(
        &mut self,
        response: WorkflowRunResponse,
    ) -> Result<Vec<UiRuntimeEvent>, String> {
        if self.workflow_run_id.as_deref() != Some(response.run_id.as_str()) {
            return Err("workflow resume response run id does not match the owned run".to_string());
        }

        self.terminal_events = 0;
        let mut events = response
            .answer
            .filter(|answer| !answer.trim().is_empty())
            .map(UiRuntimeEvent::Delta)
            .into_iter()
            .collect::<Vec<_>>();
        match response.status {
            WorkflowRunStatus::Paused => {
                let Some(pause) = response.pause else {
                    self.pause = None;
                    self.terminal_events = 1;
                    events.push(UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Error(
                        "workflow resume returned paused without pause details".to_string(),
                    )));
                    return Ok(events);
                };
                self.pause = Some(pause.clone());
                self.pause_generation = self.pause_generation.wrapping_add(1);
                events.extend(adapt_gateway_workflow_stream_event(
                    GatewayWorkflowStreamEvent::WorkflowFinished {
                        task_id: self.task_id.clone().unwrap_or_default(),
                        workflow_run_id: response.run_id,
                        data: json!({"status": "paused"}),
                        pause: Some(pause),
                    },
                ));
            }
            WorkflowRunStatus::Succeeded => {
                self.pause = None;
                events.push(UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Done {
                    finish_reason: Some("succeeded".to_string()),
                    usage: None,
                    message_id: None,
                    parent_message_id: None,
                }));
            }
            WorkflowRunStatus::Failed => {
                self.pause = None;
                events.push(UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Error(
                    response.error.unwrap_or_else(|| "workflow resume failed".to_string()),
                )));
            }
            WorkflowRunStatus::Running => {
                self.pause = None;
                events.push(UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Error(
                    "workflow resume returned unsupported running status".to_string(),
                )));
            }
        }
        self.terminal_events = usize::from(
            events.iter().any(|event| matches!(event, UiRuntimeEvent::Terminal(_))),
        );
        Ok(events)
    }

    pub(crate) fn terminal_events(&self) -> usize {
        self.terminal_events
    }

    pub(crate) fn project_local_terminal(
        &mut self,
        terminal: UiRuntimeTerminalEvent,
    ) -> Option<UiRuntimeEvent> {
        if self.terminal_events > 0 {
            return None;
        }
        self.terminal_events = 1;
        Some(UiRuntimeEvent::Terminal(terminal))
    }
}

#[derive(Debug, Clone)]
struct WorkflowFormField {
    name: String,
    kind: String,
    required: bool,
    multiple: bool,
    options: Vec<WorkflowFormOption>,
}

#[derive(Debug, Clone)]
struct WorkflowFormOption {
    display: String,
    value: Value,
}

fn workflow_resume_values(
    pause: &WorkflowPauseDto,
    answers: &[Vec<String>],
) -> Result<(BTreeMap<String, Value>, Option<String>), String> {
    let fields = workflow_form_fields(pause);
    let has_actions = !pause.actions.is_empty();
    let has_implicit_continue = fields.is_empty() && !has_actions;
    let expected_answers = fields.len() + usize::from(has_actions || has_implicit_continue);
    if answers.len() != expected_answers {
        return Err("workflow resume answer shape does not match the current pause".to_string());
    }

    let mut form_values = BTreeMap::new();
    for (field, answer) in fields.iter().zip(answers) {
        let values = workflow_field_answer(field, answer)?;
        if values.is_empty() {
            if field.required {
                return Err(format!("workflow resume field is required: {}", field.name));
            }
            continue;
        }
        let value = if field.multiple {
            Value::Array(values)
        } else {
            values.into_iter().next().expect("non-empty field answers")
        };
        form_values.insert(field.name.clone(), value);
    }

    let action = if has_actions {
        let action_answer = answers
            .get(fields.len())
            .and_then(|values| values.iter().find(|value| !is_user_note(value)))
            .ok_or_else(|| "workflow resume action is required".to_string())?;
        pause
            .actions
            .iter()
            .find(|action| {
                workflow_action_label(action.label.as_str(), action.id.as_str()) == *action_answer
            })
            .map(|action| action.id.clone())
            .ok_or_else(|| "workflow resume action does not match the current pause".to_string())?
            .into()
    } else {
        None
    };
    Ok((form_values, action))
}

fn workflow_form_fields(pause: &WorkflowPauseDto) -> Vec<WorkflowFormField> {
    pause
        .form
        .get("fields")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|field| {
            let name = workflow_value_string(field, "name")
                .or_else(|| workflow_value_string(field, "variable"))?;
            let options = field
                .get("options")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(workflow_form_option)
                .collect();
            Some(WorkflowFormField {
                name: name.to_string(),
                kind: workflow_value_string(field, "type").unwrap_or("text").to_ascii_lowercase(),
                required: field.get("required").and_then(Value::as_bool).unwrap_or(false),
                multiple: field.get("multiple").and_then(Value::as_bool).unwrap_or(false),
                options,
            })
        })
        .collect()
}

fn workflow_form_option(option: &Value) -> Option<WorkflowFormOption> {
    match option {
        Value::String(value) => Some(WorkflowFormOption {
            display: value.clone(),
            value: Value::String(value.clone()),
        }),
        Value::Object(_) => {
            let display = workflow_value_string(option, "label")
                .or_else(|| workflow_value_string(option, "name"))
                .or_else(|| workflow_value_string(option, "value"))?;
            Some(WorkflowFormOption {
                display: display.to_string(),
                value: option
                    .get("value")
                    .cloned()
                    .unwrap_or_else(|| Value::String(display.to_string())),
            })
        }
        _ => None,
    }
}

fn workflow_field_answer(
    field: &WorkflowFormField,
    answers: &[String],
) -> Result<Vec<Value>, String> {
    let values = if field.options.is_empty() {
        answers
            .iter()
            .filter_map(|answer| user_note(answer).or_else(|| (!answer.trim().is_empty()).then_some(answer.trim())))
            .map(|answer| coerce_workflow_value(field.kind.as_str(), Value::String(answer.to_string())))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        let note = answers.iter().find_map(|answer| user_note(answer));
        let mut values = Vec::new();
        for answer in answers.iter().filter(|answer| !is_user_note(answer)) {
            let value = if answer == "Other" {
                note.map(|value| Value::String(value.to_string())).ok_or_else(|| {
                    format!("workflow resume custom value is empty: {}", field.name)
                })?
            } else {
                field
                    .options
                    .iter()
                    .find(|option| option.display == *answer)
                    .map(|option| option.value.clone())
                    .ok_or_else(|| {
                        format!("workflow resume option does not match field: {}", field.name)
                    })?
            };
            values.push(coerce_workflow_value(field.kind.as_str(), value)?);
        }
        values
    };
    if !field.multiple && values.len() > 1 {
        return Err(format!("workflow resume field accepts one value: {}", field.name));
    }
    Ok(values)
}

fn coerce_workflow_value(kind: &str, value: Value) -> Result<Value, String> {
    let Value::String(text) = value else {
        return Ok(value);
    };
    match kind {
        "number" => text
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .ok_or_else(|| format!("workflow resume value is not a number: {text}")),
        "integer" => text
            .parse::<i64>()
            .map(serde_json::Number::from)
            .map(Value::Number)
            .map_err(|_| format!("workflow resume value is not an integer: {text}")),
        "boolean" | "bool" => match text.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" => Ok(Value::Bool(true)),
            "false" | "no" | "0" => Ok(Value::Bool(false)),
            _ => Err(format!("workflow resume value is not a boolean: {text}")),
        },
        _ => Ok(Value::String(text)),
    }
}

fn workflow_value_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty())
}

fn user_note(value: &str) -> Option<&str> {
    value.strip_prefix("user_note:").map(str::trim).filter(|value| !value.is_empty())
}

fn is_user_note(value: &str) -> bool {
    value.starts_with("user_note:")
}

fn record_once(slot: &mut Option<String>, value: &str) {
    if slot.is_none() {
        let value = value.trim();
        if !value.is_empty() {
            *slot = Some(value.to_string());
        }
    }
}

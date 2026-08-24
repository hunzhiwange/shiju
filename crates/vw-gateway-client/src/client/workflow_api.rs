//! Workflow 网关客户端。

use std::{collections::BTreeMap, fmt};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vw_api_types::workflow::{
    WorkflowRecord, WorkflowRecordDeleteResponse, WorkflowRecordSummary, WorkflowRecordUpsertBody,
    WorkflowPauseDto, WorkflowResumeRequest, WorkflowRunResponse,
};

use crate::http::{
    apply_request_context, log_request, log_request_succeeded, response_error, transport_error,
};

use super::GatewayClient;

const WORKFLOW_APPLICATION_STREAM_PATH_SUFFIX: &str = "/chat-messages";
const WORKFLOW_DEFAULT_STREAM_PATH: &str = "/v1/workflow/applications/chat-messages";

/// Workflow streaming 响应模式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayWorkflowResponseMode {
    Streaming,
}

/// Workflow streaming 请求体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayWorkflowStreamRequest {
    pub query: String,
    #[serde(default)]
    pub inputs: BTreeMap<String, Value>,
    pub response_mode: GatewayWorkflowResponseMode,
    pub user: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

/// Workflow 默认入口的 streaming 请求体，可指定保存应用或内联 YAML。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayWorkflowDefaultStreamRequest {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_workflow: Option<String>,
    #[serde(default)]
    pub inputs: BTreeMap<String, Value>,
    pub response_mode: GatewayWorkflowResponseMode,
    pub user: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

/// Workflow 应用一次运行后的精简结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayWorkflowRunOutcome {
    pub summary: String,
    pub workflow_run_id: Option<String>,
    pub conversation_id: Option<String>,
}

/// Workflow streaming 错误事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayWorkflowStreamError {
    pub task_id: String,
    pub status: u32,
    pub code: String,
    pub message: String,
}

/// Workflow streaming SSE 事件。
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum GatewayWorkflowStreamEvent {
    WorkflowStarted {
        task_id: String,
        workflow_run_id: String,
        data: Value,
    },
    NodeStarted {
        task_id: String,
        workflow_run_id: String,
        data: Value,
    },
    NodeFinished {
        task_id: String,
        workflow_run_id: String,
        data: Value,
    },
    TextChunk {
        task_id: String,
        workflow_run_id: String,
        text: String,
        data: Value,
    },
    Message {
        id: String,
        task_id: String,
        message_id: String,
        conversation_id: String,
        answer: String,
        created_at: i64,
    },
    WorkflowFinished {
        task_id: String,
        workflow_run_id: String,
        data: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pause: Option<WorkflowPauseDto>,
    },
    MessageEnd {
        id: String,
        task_id: String,
        message_id: String,
        conversation_id: String,
        metadata: Value,
    },
    Error(GatewayWorkflowStreamError),
    Unknown {
        event_type: Option<String>,
        payload: Value,
    },
}

impl fmt::Debug for GatewayWorkflowStreamEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkflowStarted { task_id, workflow_run_id, data } => formatter
                .debug_struct("WorkflowStarted")
                .field("task_id", task_id)
                .field("workflow_run_id", workflow_run_id)
                .field("data", data)
                .finish(),
            Self::NodeStarted { task_id, workflow_run_id, data } => formatter
                .debug_struct("NodeStarted")
                .field("task_id", task_id)
                .field("workflow_run_id", workflow_run_id)
                .field("data", data)
                .finish(),
            Self::NodeFinished { task_id, workflow_run_id, data } => formatter
                .debug_struct("NodeFinished")
                .field("task_id", task_id)
                .field("workflow_run_id", workflow_run_id)
                .field("data", data)
                .finish(),
            Self::TextChunk { task_id, workflow_run_id, text, data } => formatter
                .debug_struct("TextChunk")
                .field("task_id", task_id)
                .field("workflow_run_id", workflow_run_id)
                .field("text", text)
                .field("data", data)
                .finish(),
            Self::Message {
                id,
                task_id,
                message_id,
                conversation_id,
                answer,
                created_at,
            } => formatter
                .debug_struct("Message")
                .field("id", id)
                .field("task_id", task_id)
                .field("message_id", message_id)
                .field("conversation_id", conversation_id)
                .field("answer", answer)
                .field("created_at", created_at)
                .finish(),
            Self::WorkflowFinished { task_id, workflow_run_id, data, pause } => {
                let mut debug = formatter.debug_struct("WorkflowFinished");
                debug.field("task_id", task_id).field("workflow_run_id", workflow_run_id);
                if data.get("pause").is_some() {
                    debug.field("data", &"[REDACTED]");
                } else {
                    debug.field("data", data);
                }
                debug.field("pause", &pause.as_ref().map(RedactedWorkflowPause)).finish()
            }
            Self::MessageEnd { id, task_id, message_id, conversation_id, metadata } => formatter
                .debug_struct("MessageEnd")
                .field("id", id)
                .field("task_id", task_id)
                .field("message_id", message_id)
                .field("conversation_id", conversation_id)
                .field("metadata", metadata)
                .finish(),
            Self::Error(error) => formatter.debug_tuple("Error").field(error).finish(),
            Self::Unknown { event_type, payload } => formatter
                .debug_struct("Unknown")
                .field("event_type", event_type)
                .field("payload", payload)
                .finish(),
        }
    }
}

struct RedactedWorkflowPause<'a>(&'a WorkflowPauseDto);

impl fmt::Debug for RedactedWorkflowPause<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorkflowPauseDto")
            .field("node_id", &self.0.node_id)
            .field("title", &self.0.title)
            .field("form_token", &"[REDACTED]")
            .field("form", &self.0.form)
            .field("actions", &self.0.actions)
            .finish()
    }
}

impl GatewayClient {
    /// 恢复暂停的 Workflow 运行。
    pub async fn workflow_resume(
        &self,
        body: &WorkflowResumeRequest,
    ) -> Result<WorkflowRunResponse, String> {
        self.post_json("/v1/workflow/resume", &[], body).await
    }

    /// 列出本地保存的 Dify Workflow。
    pub async fn workflow_applications_list(&self) -> Result<Vec<WorkflowRecordSummary>, String> {
        self.get_json("/v1/workflow/applications", &[]).await
    }

    /// 读取本地保存的 Dify Workflow。
    pub async fn workflow_application_get(&self, uuid: &str) -> Result<WorkflowRecord, String> {
        self.get_json(&workflow_application_path(uuid)?, &[]).await
    }

    /// 新增本地 Dify Workflow。
    pub async fn workflow_application_create(
        &self,
        body: &WorkflowRecordUpsertBody,
    ) -> Result<WorkflowRecord, String> {
        self.post_json("/v1/workflow/applications", &[], body).await
    }

    /// 更新本地 Dify Workflow。
    pub async fn workflow_application_update(
        &self,
        uuid: &str,
        body: &WorkflowRecordUpsertBody,
    ) -> Result<WorkflowRecord, String> {
        self.put_json(&workflow_application_path(uuid)?, &[], body).await
    }

    /// 删除本地 Dify Workflow。
    pub async fn workflow_application_delete(
        &self,
        uuid: &str,
    ) -> Result<WorkflowRecordDeleteResponse, String> {
        self.delete_json(&workflow_application_path(uuid)?, &[], &serde_json::json!({}))
            .await
    }

    /// 运行默认 Workflow 入口并按 SSE 顺序回调事件。
    pub async fn stream_default_workflow_application(
        &self,
        body: &GatewayWorkflowDefaultStreamRequest,
        on_event: impl FnMut(GatewayWorkflowStreamEvent) -> bool,
    ) -> Result<(), String> {
        let has_uuid = body
            .application_uuid
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());
        let has_workflow = body
            .application_workflow
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());
        if !has_uuid && !has_workflow {
            return Err("application_uuid or application_workflow is required".to_string());
        }
        self.stream_workflow_request(WORKFLOW_DEFAULT_STREAM_PATH, body, on_event).await
    }

    /// 运行本地 Workflow 应用并按 SSE 顺序回调事件。
    pub async fn stream_workflow_application(
        &self,
        uuid: &str,
        body: &GatewayWorkflowStreamRequest,
        on_event: impl FnMut(GatewayWorkflowStreamEvent) -> bool,
    ) -> Result<(), String> {
        let path = workflow_application_stream_path(uuid)?;
        self.stream_workflow_request(&path, body, on_event).await
    }

    async fn stream_workflow_request<T: Serialize>(
        &self,
        path: &str,
        body: &T,
        mut on_event: impl FnMut(GatewayWorkflowStreamEvent) -> bool,
    ) -> Result<(), String> {
        log_request("POST", &self.endpoint, path, &[], Some(body));
        let request = self.client.post(format!("{}{}", self.endpoint.base_url(), path)).json(body);
        let response = apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|err| transport_error("POST", &self.endpoint, path, err))?;
        if !response.status().is_success() {
            return Err(response_error("POST", &self.endpoint, path, response).await);
        }

        log_request_succeeded("POST", &self.endpoint, path);
        let mut bytes_stream = response.bytes_stream();
        let mut sse_buffer = String::new();
        while let Some(chunk) = bytes_stream.next().await {
            let chunk = chunk.map_err(|err| err.to_string())?;
            sse_buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(frame) = crate::stream::take_next_sse_event(&mut sse_buffer) {
                let payload = frame
                    .lines()
                    .filter_map(|line| line.trim_end_matches('\r').strip_prefix("data:"))
                    .map(str::trim_start)
                    .collect::<Vec<_>>()
                    .join("\n");
                if payload.is_empty() {
                    continue;
                }

                let payload: Value =
                    serde_json::from_str(&payload).map_err(|err| err.to_string())?;
                if !on_event(parse_workflow_stream_event(payload)) {
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    /// 运行本地 Workflow 应用并返回适合上层记录的安全摘要。
    pub async fn run_workflow_application(
        &self,
        uuid: &str,
        body: &GatewayWorkflowStreamRequest,
    ) -> Result<GatewayWorkflowRunOutcome, String> {
        let mut text = String::new();
        let mut answer = None;
        let mut workflow_run_id = None;
        let mut conversation_id = None;
        let mut workflow_error = None;

        self.stream_workflow_application(uuid, body, |event| {
            match event {
                GatewayWorkflowStreamEvent::WorkflowStarted { workflow_run_id: run_id, .. }
                | GatewayWorkflowStreamEvent::NodeStarted { workflow_run_id: run_id, .. }
                | GatewayWorkflowStreamEvent::NodeFinished { workflow_run_id: run_id, .. } => {
                    if workflow_run_id.is_none() && !run_id.trim().is_empty() {
                        workflow_run_id = Some(run_id);
                    }
                }
                GatewayWorkflowStreamEvent::TextChunk {
                    workflow_run_id: run_id,
                    text: chunk,
                    ..
                } => {
                    if workflow_run_id.is_none() && !run_id.trim().is_empty() {
                        workflow_run_id = Some(run_id);
                    }
                    text.push_str(&chunk);
                }
                GatewayWorkflowStreamEvent::Message {
                    answer: message_answer,
                    conversation_id: message_conversation_id,
                    ..
                } => {
                    if !message_answer.trim().is_empty() {
                        answer = Some(message_answer);
                    }
                    if conversation_id.is_none() && !message_conversation_id.trim().is_empty() {
                        conversation_id = Some(message_conversation_id);
                    }
                }
                GatewayWorkflowStreamEvent::WorkflowFinished {
                    workflow_run_id: run_id,
                    data,
                    ..
                } => {
                    if workflow_run_id.is_none() && !run_id.trim().is_empty() {
                        workflow_run_id = Some(run_id);
                    }
                    if let Some(status) = data.get("status").and_then(Value::as_str)
                        && matches!(status, "failed" | "error")
                    {
                        workflow_error = Some(format!("workflow finished with status {status}"));
                        return false;
                    }
                }
                GatewayWorkflowStreamEvent::MessageEnd {
                    conversation_id: message_conversation_id,
                    ..
                } => {
                    if conversation_id.is_none() && !message_conversation_id.trim().is_empty() {
                        conversation_id = Some(message_conversation_id);
                    }
                }
                GatewayWorkflowStreamEvent::Error(error) => {
                    workflow_error = Some(format!("{}: {}", error.code, error.message));
                    return false;
                }
                GatewayWorkflowStreamEvent::Unknown { .. } => {}
            }
            true
        })
        .await?;

        if let Some(error) = workflow_error {
            return Err(error);
        }

        let summary = answer
            .or_else(|| {
                let text = text.trim();
                (!text.is_empty()).then(|| text.to_string())
            })
            .unwrap_or_else(|| "workflow completed".to_string());

        Ok(GatewayWorkflowRunOutcome { summary, workflow_run_id, conversation_id })
    }
}

fn workflow_application_path(uuid: &str) -> Result<String, String> {
    if uuid.trim().is_empty() {
        return Err("workflow application uuid must not be empty".to_string());
    }
    Ok(format!("/v1/workflow/applications/{}", encode_path_segment(uuid)))
}

fn workflow_application_stream_path(uuid: &str) -> Result<String, String> {
    Ok(format!(
        "{}{WORKFLOW_APPLICATION_STREAM_PATH_SUFFIX}",
        workflow_application_path(uuid)?
    ))
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn parse_workflow_stream_event(payload: Value) -> GatewayWorkflowStreamEvent {
    match payload.get("event").and_then(Value::as_str) {
        Some("workflow_started") => GatewayWorkflowStreamEvent::WorkflowStarted {
            task_id: string_field(&payload, "task_id"),
            workflow_run_id: string_field(&payload, "workflow_run_id"),
            data: data_field(&payload),
        },
        Some("node_started") => GatewayWorkflowStreamEvent::NodeStarted {
            task_id: string_field(&payload, "task_id"),
            workflow_run_id: string_field(&payload, "workflow_run_id"),
            data: data_field(&payload),
        },
        Some("node_finished") => GatewayWorkflowStreamEvent::NodeFinished {
            task_id: string_field(&payload, "task_id"),
            workflow_run_id: string_field(&payload, "workflow_run_id"),
            data: data_field(&payload),
        },
        Some("text_chunk") => {
            let data = data_field(&payload);
            GatewayWorkflowStreamEvent::TextChunk {
                task_id: string_field(&payload, "task_id"),
                workflow_run_id: string_field(&payload, "workflow_run_id"),
                text: string_field(&data, "text"),
                data,
            }
        }
        Some("message") => GatewayWorkflowStreamEvent::Message {
            id: string_field(&payload, "id"),
            task_id: string_field(&payload, "task_id"),
            message_id: string_field(&payload, "message_id"),
            conversation_id: string_field(&payload, "conversation_id"),
            answer: string_field(&payload, "answer"),
            created_at: payload.get("created_at").and_then(Value::as_i64).unwrap_or_default(),
        },
        Some("workflow_finished") => {
            let data = data_field(&payload);
            let pause = data
                .get("pause")
                .cloned()
                .and_then(|value| serde_json::from_value(value).ok());
            GatewayWorkflowStreamEvent::WorkflowFinished {
                task_id: string_field(&payload, "task_id"),
                workflow_run_id: string_field(&payload, "workflow_run_id"),
                data,
                pause,
            }
        }
        Some("message_end") => GatewayWorkflowStreamEvent::MessageEnd {
            id: string_field(&payload, "id"),
            task_id: string_field(&payload, "task_id"),
            message_id: string_field(&payload, "message_id"),
            conversation_id: string_field(&payload, "conversation_id"),
            metadata: payload.get("metadata").cloned().unwrap_or(Value::Object(Default::default())),
        },
        Some("error") => GatewayWorkflowStreamEvent::Error(GatewayWorkflowStreamError {
            task_id: string_field(&payload, "task_id"),
            status: payload
                .get("status")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or_default(),
            code: string_field(&payload, "code"),
            message: string_field(&payload, "message"),
        }),
        other => GatewayWorkflowStreamEvent::Unknown {
            event_type: other.map(ToOwned::to_owned),
            payload,
        },
    }
}

fn data_field(payload: &Value) -> Value {
    payload.get("data").cloned().unwrap_or(Value::Object(Default::default()))
}

fn string_field(payload: &Value, field: &str) -> String {
    payload.get(field).and_then(Value::as_str).unwrap_or_default().to_string()
}

#[cfg(test)]
#[path = "workflow_api_tests.rs"]
mod workflow_api_tests;

//! 入站 ACP 提示序列化及原生会话事件映射。

use acp_server::server::{BackendEvent, BackendTermination, BackendToolStatus, PromptContent};
use session_port::{PromptEvent, PromptTermination, ToolCallStatus, ToolEvent, ToolEventKind};
use serde_json::{Value, json};
use acp_server::server::{BackendHistoryEntry, BackendRole};
use session_port::ui_types::{ChatMessage, ChatRole};

pub fn prompt_query(prompt: Vec<PromptContent>) -> String {
    prompt
        .into_iter()
        .map(|content| match content {
            PromptContent::Text(text) => text,
            PromptContent::ResourceLink { name, uri, mime_type } => json!({
                "type": "resource_link",
                "name": name,
                "uri": uri,
                "mime_type": mime_type,
            })
            .to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn backend_tool_status(status: &ToolCallStatus) -> BackendToolStatus {
    match status {
        ToolCallStatus::Running | ToolCallStatus::Waiting => BackendToolStatus::Running,
        ToolCallStatus::Completed => BackendToolStatus::Completed,
        ToolCallStatus::Failed | ToolCallStatus::Denied => BackendToolStatus::Failed,
        ToolCallStatus::Cancelled => BackendToolStatus::Cancelled,
        ToolCallStatus::Other(_) => BackendToolStatus::Failed,
    }
}

fn payload_text(payload: &Value) -> Option<String> {
    payload
        .get("output")
        .or_else(|| payload.get("error"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn map_tool_event(event: ToolEvent) -> BackendEvent {
    let status = backend_tool_status(&event.status);
    match event.kind {
        ToolEventKind::Start => BackendEvent::ToolStart {
            call_id: event.call_id,
            title: event
                .payload
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or(&event.name)
                .to_string(),
            raw_input: event.payload.get("input").cloned().unwrap_or(Value::Null),
        },
        ToolEventKind::Update => BackendEvent::ToolUpdate {
            call_id: event.call_id,
            status,
            content: payload_text(&event.payload),
            raw_output: event.payload.get("output").cloned(),
        },
        ToolEventKind::Complete => BackendEvent::ToolComplete {
            call_id: event.call_id,
            status,
            content: payload_text(&event.payload),
            raw_output: event.payload.get("output").cloned(),
        },
    }
}

pub fn map_runtime_event(event: PromptEvent) -> Option<BackendEvent> {
    match event {
        PromptEvent::LlmRawArtifact(_) | PromptEvent::ModelRecovery(_) => None,
        PromptEvent::ContextLoaded { paths } => {
            Some(BackendEvent::Status(format!("loaded context: {}", paths.join(", "))))
        }
        PromptEvent::Text(text) => Some(BackendEvent::Text(text)),
        PromptEvent::Tool(event) => Some(map_tool_event(event)),
        PromptEvent::TodoContinuation => {
            Some(BackendEvent::Status("TODO items remain; continuing work".into()))
        }
        PromptEvent::Compaction { .. } => None,
        PromptEvent::StepStart { step_index, .. } => {
            Some(BackendEvent::Status(format!("model step {step_index} started")))
        }
        PromptEvent::StepFinish { step_index, .. } => {
            Some(BackendEvent::Status(format!("model step {step_index} finished")))
        }
        PromptEvent::PostToolRound { step_index } => {
            Some(BackendEvent::Status(format!("tool round {step_index} finished")))
        }
        PromptEvent::Terminated(PromptTermination::Completed(_)) => {
            Some(BackendEvent::Terminal(BackendTermination::EndTurn))
        }
        PromptEvent::Terminated(PromptTermination::Cancelled) => {
            Some(BackendEvent::Terminal(BackendTermination::Cancelled))
        }
        PromptEvent::Terminated(PromptTermination::Failed(error)) => {
            Some(BackendEvent::Error(error))
        }
    }
}


pub fn processor_history(history: &[BackendHistoryEntry]) -> Vec<ChatMessage> {
    history
        .iter()
        .map(|entry| ChatMessage {
            role: match entry.role {
                BackendRole::User => ChatRole::User,
                BackendRole::Assistant => ChatRole::Assistant,
            },
            content: entry.text.clone(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        })
        .collect()
}

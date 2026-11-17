//! 网关派发请求转换及流事件日志投影。
use std::path::Path;
use anyhow::Result;
use serde_json::{Map, Value, json};
use vw_api_chat::chat::GatewayChatStreamRequest;
use vw_api_transport::common::TimestampMs;
use crate::domain::{RunId, TaskId};
use crate::runner::local::LocalAgentExecutionRequest;
use crate::store::AgentSchedulerStore;
use crate::store::logs::AppendAgentTaskLogRequest;
pub(crate) const GATEWAY_STREAM_DELTA_LOG_LIMIT: usize = 280;
pub(crate) const GATEWAY_STREAM_FIELD_LOG_LIMIT: usize = 160;
/// 将网关流式载荷裁剪为安全长度后追加到任务日志。
pub fn append_gateway_stream_payload_task_log(
    store: &AgentSchedulerStore,
    project_path: &Path,
    task_id: &TaskId,
    run_id: &RunId,
    timestamp_ms: TimestampMs,
    payload: &Value,
) -> Result<()> {
    for message in gateway_stream_payload_log_messages(payload) {
        store.append_task_log(
            project_path,
            AppendAgentTaskLogRequest {
                task_id: task_id.clone(),
                run_id: Some(run_id.clone()),
                timestamp_ms,
                from_status: None,
                to_status: None,
                from_status_key: None,
                to_status_key: None,
                message,
            },
        )?;
    }
    Ok(())
}

pub fn gateway_stream_payload_log_messages(payload: &Value) -> Vec<String> {
    match payload.get("type").and_then(Value::as_str) {
        Some("chat.delta") => payload
            .get("delta")
            .and_then(Value::as_str)
            .map(gateway_delta_log_messages)
            .unwrap_or_default(),
        Some("chat.step_start") => vec![gateway_stream_step_message(
            payload,
            "started",
            payload.get("model").and_then(Value::as_str).and_then(non_empty_str).map(|model| {
                format!(" model={}", truncate_log_text(model, GATEWAY_STREAM_FIELD_LOG_LIMIT))
            }),
        )],
        Some("chat.step_finish") => vec![gateway_stream_step_message(
            payload,
            "finished",
            payload.get("finish_reason").and_then(Value::as_str).and_then(non_empty_str).map(
                |reason| {
                    format!(
                        " finish_reason={}",
                        truncate_log_text(reason, GATEWAY_STREAM_FIELD_LOG_LIMIT)
                    )
                },
            ),
        )],
        Some("chat.post_tool_round") => {
            vec![gateway_stream_step_message(payload, "completed tool round", None)]
        }
        Some("chat.error") => payload
            .get("error")
            .and_then(Value::as_str)
            .and_then(non_empty_str)
            .map(|error| {
                format!(
                    "gateway stream error: {}",
                    truncate_log_text(error, GATEWAY_STREAM_DELTA_LOG_LIMIT)
                )
            })
            .into_iter()
            .collect(),
        Some("chat.done") => vec!["gateway stream completed".to_string()],
        _ => Vec::new(),
    }
}

pub fn gateway_delta_log_messages(delta: &str) -> Vec<String> {
    delta
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(3)
        .map(|line| {
            format!(
                "gateway stream output: {}",
                truncate_log_text(line, GATEWAY_STREAM_DELTA_LOG_LIMIT)
            )
        })
        .collect()
}

pub fn gateway_stream_step_message(payload: &Value, action: &str, suffix: Option<String>) -> String {
    let step_index =
        i64_field(payload, "step_index").map_or_else(|| "?".to_string(), |value| value.to_string());
    format!("gateway stream step {step_index} {action}{}", suffix.unwrap_or_default())
}

pub fn truncate_log_text(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let mut truncated: String = trimmed.chars().take(keep).collect();
    truncated.push_str("...");
    truncated
}

/// 从执行请求构造网关流式聊天请求，并保留模型与代理路由选择。
pub fn gateway_local_agent_stream_request(
    request: &LocalAgentExecutionRequest,
) -> GatewayChatStreamRequest {
    let mut options = gateway_local_agent_options(request.workspace.path.clone());
    if let Some(agent) = request.agent.as_deref() {
        options.insert("agent".to_string(), json!(agent));
    }
    if let Some(acp_agent) = request.acp_agent.as_deref() {
        options.insert("acp_test".to_string(), json!(true));
        options.insert("acp_agent".to_string(), json!(acp_agent));
        options.insert("acp_force_new_session".to_string(), json!(true));
        options.insert("acp_close_client_after_request".to_string(), json!(true));
        options.insert("acp_history_strategy".to_string(), json!("discard"));
        options.insert("acp_history_recent_count".to_string(), json!(1));
        options.insert("full_access".to_string(), json!(false));
        options.insert("acp_permission_mode".to_string(), json!("approve-reads"));
        options.insert("acp_non_interactive_permissions".to_string(), json!("deny"));
    }

    GatewayChatStreamRequest {
        acp_model_source: vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel,
        session_id: Some(
            format!("agent-scheduler-{}-{}", request.task_id.as_str(), request.run_id.as_str())
                .into(),
        ),
        messages: vec![json!({ "role": "user", "content": request.prompt.clone() })],
        system: None,
        model: request.model.clone(),
        reasoning_effort: request.reasoning_effort,
        agent: request.agent.clone(),
        allowed_tools: None,
        acp_agent: request.acp_agent.clone(),
        acp_allowed_tools: None,
        options: Some(Value::Object(options)),
    }
}

pub fn gateway_local_agent_options(cwd: String) -> Map<String, Value> {
    let mut options = Map::new();
    options.insert("cwd".to_string(), json!(cwd));
    // Leave native access to the bound agent defaults and session permission policy.
    options
}

pub fn i64_field(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

pub fn non_empty_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

//! 队列 IPC 请求与响应消息结构定义。

use agent_client_protocol::SetSessionConfigOptionResponse;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use acp_types::acp_jsonrpc::is_acp_json_rpc_message;
use acp_types::prompt_content::{PromptInput, is_prompt_input, text_prompt};
use acp_types::types::{
    AcpJsonRpcMessage, NonInteractivePermissionPolicy, OutputErrorAcpPayload, OutputErrorCode,
    OutputErrorOrigin, PermissionMode, SessionResumePolicy, SessionSendResult,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum QueueRequest {
    SubmitPrompt {
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_generation: Option<u64>,
        message: String,
        prompt: PromptInput,
        permission_mode: PermissionMode,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resume_policy: Option<SessionResumePolicy>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        suppress_sdk_console_errors: Option<bool>,
        wait_for_completion: bool,
    },
    CancelPrompt {
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_generation: Option<u64>,
    },
    SetMode {
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_generation: Option<u64>,
        mode_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    SetModel {
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_generation: Option<u64>,
        model_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    SetConfigOption {
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_generation: Option<u64>,
        config_id: String,
        value: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum QueueOwnerMessage {
    Accepted {
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_generation: Option<u64>,
    },
    Event {
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_generation: Option<u64>,
        message: AcpJsonRpcMessage,
    },
    Result {
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_generation: Option<u64>,
        result: Box<SessionSendResult>,
    },
    CancelResult {
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_generation: Option<u64>,
        cancelled: bool,
    },
    SetModeResult {
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_generation: Option<u64>,
        mode_id: String,
    },
    SetModelResult {
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_generation: Option<u64>,
        model_id: String,
    },
    SetConfigOptionResult {
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_generation: Option<u64>,
        response: SetSessionConfigOptionResponse,
    },
    Error {
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_generation: Option<u64>,
        code: OutputErrorCode,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail_code: Option<String>,
        origin: OutputErrorOrigin,
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        retryable: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        acp: Option<OutputErrorAcpPayload>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output_already_emitted: Option<bool>,
    },
}

fn as_record(value: &Value) -> Option<&Map<String, Value>> {
    match value {
        Value::Object(record) => Some(record),
        _ => None,
    }
}

fn deserialize<T: DeserializeOwned>(value: &Value) -> Option<T> {
    serde_json::from_value(value.clone()).ok()
}

fn parse_owner_generation(value: Option<&Value>) -> Result<Option<u64>, ()> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => match value.as_u64() {
            Some(owner_generation) if owner_generation > 0 => Ok(Some(owner_generation)),
            _ => Err(()),
        },
    }
}

fn parse_timeout_ms(value: Option<&Value>) -> Option<u64> {
    let timeout_ms = value?.as_f64()?;
    if !timeout_ms.is_finite() || timeout_ms <= 0.0 || timeout_ms.round() > u64::MAX as f64 {
        return None;
    }

    Some(timeout_ms.round() as u64)
}

fn parse_acp_error(value: Option<&Value>) -> Option<OutputErrorAcpPayload> {
    let record = as_record(value?)?;
    let code = record.get("code")?.as_i64()?;
    let message = record.get("message")?.as_str()?.trim();
    if message.is_empty() {
        return None;
    }

    Some(OutputErrorAcpPayload {
        code,
        message: message.to_string(),
        data: record.get("data").cloned(),
    })
}

pub fn parse_queue_request(raw: &Value) -> Option<QueueRequest> {
    let request = as_record(raw)?;
    let request_type = request.get("type")?.as_str()?;
    let request_id = request.get("requestId")?.as_str()?.to_string();
    let owner_generation = parse_owner_generation(request.get("ownerGeneration")).ok()?;
    let timeout_ms = parse_timeout_ms(request.get("timeoutMs"));

    match request_type {
        "submit_prompt" => {
            let message = request.get("message")?.as_str()?.to_string();
            let permission_mode = deserialize::<PermissionMode>(request.get("permissionMode")?)?;
            let resume_policy = match request.get("resumePolicy") {
                Some(Value::Null) | None => None,
                Some(value) => Some(deserialize::<SessionResumePolicy>(value)?),
            };
            let non_interactive_permissions = match request.get("nonInteractivePermissions") {
                Some(Value::Null) | None => None,
                Some(value) => Some(deserialize::<NonInteractivePermissionPolicy>(value)?),
            };
            let suppress_sdk_console_errors = match request.get("suppressSdkConsoleErrors") {
                Some(Value::Bool(value)) => Some(*value),
                Some(Value::Null) | None => None,
                Some(_) => return None,
            };
            let wait_for_completion = request.get("waitForCompletion")?.as_bool()?;
            let prompt = match request.get("prompt") {
                Some(value) if is_prompt_input(value) => deserialize::<PromptInput>(value)?,
                Some(Value::Null) | None => text_prompt(message.clone()),
                Some(_) => return None,
            };

            Some(QueueRequest::SubmitPrompt {
                request_id,
                owner_generation,
                message,
                prompt,
                permission_mode,
                resume_policy,
                non_interactive_permissions,
                timeout_ms,
                suppress_sdk_console_errors,
                wait_for_completion,
            })
        }
        "cancel_prompt" => Some(QueueRequest::CancelPrompt { request_id, owner_generation }),
        "set_mode" => {
            let mode_id = request.get("modeId")?.as_str()?.to_string();
            if mode_id.trim().is_empty() {
                return None;
            }

            Some(QueueRequest::SetMode { request_id, owner_generation, mode_id, timeout_ms })
        }
        "set_model" => {
            let model_id = request.get("modelId")?.as_str()?.to_string();
            if model_id.trim().is_empty() {
                return None;
            }

            Some(QueueRequest::SetModel { request_id, owner_generation, model_id, timeout_ms })
        }
        "set_config_option" => {
            let config_id = request.get("configId")?.as_str()?.to_string();
            let value = request.get("value")?.as_str()?.to_string();
            if config_id.trim().is_empty() || value.trim().is_empty() {
                return None;
            }

            Some(QueueRequest::SetConfigOption {
                request_id,
                owner_generation,
                config_id,
                value,
                timeout_ms,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "queue_messages_tests.rs"]
mod queue_messages_tests;

pub fn parse_queue_owner_message(raw: &Value) -> Option<QueueOwnerMessage> {
    let message = as_record(raw)?;
    let message_type = message.get("type")?.as_str()?;
    let request_id = message.get("requestId")?.as_str()?.to_string();
    let owner_generation = parse_owner_generation(message.get("ownerGeneration")).ok()?;

    match message_type {
        "accepted" => Some(QueueOwnerMessage::Accepted { request_id, owner_generation }),
        "event" => {
            let payload = message.get("message")?;
            if !is_acp_json_rpc_message(payload) {
                return None;
            }

            Some(QueueOwnerMessage::Event {
                request_id,
                owner_generation,
                message: deserialize::<AcpJsonRpcMessage>(payload)?,
            })
        }
        "result" => Some(QueueOwnerMessage::Result {
            request_id,
            owner_generation,
            result: Box::new(deserialize::<SessionSendResult>(message.get("result")?)?),
        }),
        "cancel_result" => Some(QueueOwnerMessage::CancelResult {
            request_id,
            owner_generation,
            cancelled: message.get("cancelled")?.as_bool()?,
        }),
        "set_mode_result" => Some(QueueOwnerMessage::SetModeResult {
            request_id,
            owner_generation,
            mode_id: message.get("modeId")?.as_str()?.to_string(),
        }),
        "set_model_result" => Some(QueueOwnerMessage::SetModelResult {
            request_id,
            owner_generation,
            model_id: message.get("modelId")?.as_str()?.to_string(),
        }),
        "set_config_option_result" => Some(QueueOwnerMessage::SetConfigOptionResult {
            request_id,
            owner_generation,
            response: deserialize::<SetSessionConfigOptionResponse>(message.get("response")?)?,
        }),
        "error" => {
            let error_message = message.get("message")?.as_str()?.to_string();
            let detail_code = message
                .get("detailCode")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|detail_code| !detail_code.is_empty())
                .map(ToOwned::to_owned);
            let retryable = message.get("retryable").and_then(Value::as_bool);
            let output_already_emitted =
                message.get("outputAlreadyEmitted").and_then(Value::as_bool);

            Some(QueueOwnerMessage::Error {
                request_id,
                owner_generation,
                code: deserialize::<OutputErrorCode>(message.get("code")?)?,
                detail_code,
                origin: deserialize::<OutputErrorOrigin>(message.get("origin")?)?,
                message: error_message,
                retryable,
                acp: parse_acp_error(message.get("acp")),
                output_already_emitted,
            })
        }
        _ => None,
    }
}

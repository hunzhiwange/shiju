//! ACP JSON-RPC 消息识别与字段解析工具。

use agent_client_protocol::SessionNotification;
use serde_json::{Map, Value};

fn as_record(value: &Value) -> Option<&Map<String, Value>> {
    match value {
        Value::Object(record) => Some(record),
        _ => None,
    }
}

fn has_valid_id(value: &Value) -> bool {
    matches!(value, Value::Null | Value::String(_))
        || value.as_f64().is_some_and(|number| number.is_finite())
}

fn is_error_object(value: &Value) -> bool {
    let Some(record) = as_record(value) else {
        return false;
    };

    record.get("code").and_then(Value::as_f64).is_some_and(f64::is_finite)
        && record.get("message").and_then(Value::as_str).is_some()
}

fn has_result_or_error(value: &Map<String, Value>) -> bool {
    let has_result = value.contains_key("result");
    let has_error = value.contains_key("error");
    if has_result == has_error {
        return false;
    }
    if has_error && !value.get("error").is_some_and(is_error_object) {
        return false;
    }
    true
}

pub fn is_acp_json_rpc_message(value: &Value) -> bool {
    let Some(record) = as_record(value) else {
        return false;
    };
    if record.get("jsonrpc") != Some(&Value::String("2.0".to_string())) {
        return false;
    }

    let has_method =
        record.get("method").and_then(Value::as_str).is_some_and(|method| !method.is_empty());
    let has_id = record.contains_key("id");

    if has_method && !has_id {
        return true;
    }
    if has_method && has_id {
        return record.get("id").is_some_and(has_valid_id);
    }
    if !has_method && has_id {
        return record.get("id").is_some_and(has_valid_id) && has_result_or_error(record);
    }

    false
}

pub fn is_json_rpc_notification(message: &Value) -> bool {
    let Some(record) = as_record(message) else {
        return false;
    };

    record.get("method").and_then(Value::as_str).is_some() && !record.contains_key("id")
}

pub fn is_session_update_notification(message: &Value) -> bool {
    let Some(record) = as_record(message) else {
        return false;
    };

    is_json_rpc_notification(message)
        && record.get("method") == Some(&Value::String("session/update".to_string()))
}

pub fn extract_session_update_notification(message: &Value) -> Option<SessionNotification> {
    if !is_session_update_notification(message) {
        return None;
    }

    let params = as_record(as_record(message)?.get("params")?)?;
    params.get("sessionId").and_then(Value::as_str)?;
    let update = as_record(params.get("update")?)?;
    update.get("sessionUpdate").and_then(Value::as_str)?;

    serde_json::from_value(Value::Object(params.clone())).ok()
}

pub fn parse_prompt_stop_reason(message: &Value) -> Option<String> {
    let result = as_record(as_record(message)?.get("result")?)?;
    result.get("stopReason").and_then(Value::as_str).map(ToOwned::to_owned)
}

pub fn parse_json_rpc_error_message(message: &Value) -> Option<String> {
    let error = as_record(as_record(message)?.get("error")?)?;
    error.get("message").and_then(Value::as_str).map(ToOwned::to_owned)
}

#[cfg(test)]
#[path = "acp_jsonrpc_tests.rs"]
mod acp_jsonrpc_tests;

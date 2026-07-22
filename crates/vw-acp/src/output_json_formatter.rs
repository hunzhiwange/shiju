//! JSON 输出格式化细节与读操作抑制处理。

use std::collections::HashMap;
use std::io::Write;

use serde_json::{Map, Value, json};

use crate::jsonrpc_error::BuildJsonRpcErrorParams;
use crate::read_output_suppression::{ReadLikeToolDescriptor, SUPPRESSED_READ_OUTPUT};
use crate::types::{AcpJsonRpcMessage, OutputErrorParams, OutputFormatter, OutputFormatterContext};
use crate::{build_json_rpc_error_response, is_read_like_tool};

const DEFAULT_JSON_SESSION_ID: &str = "unknown";

fn json_rpc_id_key(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(format!("s:{value}")),
        Value::Number(value) => Some(format!("n:{value}")),
        _ => None,
    }
}

fn sanitize_read_result(result: &Value) -> Value {
    let Some(record) = result.as_object() else {
        return result.clone();
    };
    let Some(content) = record.get("content") else {
        return result.clone();
    };
    if !content.is_string() {
        return result.clone();
    }

    let mut sanitized = record.clone();
    sanitized.insert("content".to_string(), Value::String(SUPPRESSED_READ_OUTPUT.to_string()));
    Value::Object(sanitized)
}

fn sanitize_tool_content(content: &Value) -> Value {
    if !content.is_array() {
        return content.clone();
    }

    json!([
        {
            "type": "content",
            "content": {
                "type": "text",
                "text": SUPPRESSED_READ_OUTPUT
            }
        }
    ])
}

fn sanitize_tool_message(message: &Value) -> Value {
    let Some(root) = message.as_object() else {
        return message.clone();
    };
    let Some(params) = root.get("params").and_then(Value::as_object) else {
        return message.clone();
    };
    let Some(update) = params.get("update").and_then(Value::as_object) else {
        return message.clone();
    };

    let mut sanitized_update = update.clone();
    if update.contains_key("rawOutput") && !update.get("rawOutput").is_some_and(Value::is_null) {
        sanitized_update.insert(
            "rawOutput".to_string(),
            json!({
                "content": SUPPRESSED_READ_OUTPUT,
            }),
        );
    }
    if update.contains_key("content") && !update.get("content").is_some_and(Value::is_null) {
        sanitized_update.insert(
            "content".to_string(),
            sanitize_tool_content(update.get("content").unwrap_or(&Value::Null)),
        );
    }

    let mut sanitized_params = params.clone();
    sanitized_params.insert("update".to_string(), Value::Object(sanitized_update));

    let mut sanitized_root = root.clone();
    sanitized_root.insert("params".to_string(), Value::Object(sanitized_params));
    Value::Object(sanitized_root)
}

pub struct JsonOutputFormatter<W: Write> {
    stdout: W,
    suppress_reads: bool,
    session_id: String,
    request_method_by_id: HashMap<String, String>,
    tool_state_by_id: HashMap<String, ReadLikeToolDescriptor>,
}

impl<W: Write> JsonOutputFormatter<W> {
    pub fn new(stdout: W, suppress_reads: bool, context: Option<OutputFormatterContext>) -> Self {
        Self {
            stdout,
            suppress_reads,
            session_id: context
                .map(|context| context.session_id.trim().to_string())
                .filter(|session_id| !session_id.is_empty())
                .unwrap_or_else(|| DEFAULT_JSON_SESSION_ID.to_string()),
            request_method_by_id: HashMap::new(),
            tool_state_by_id: HashMap::new(),
        }
    }

    pub fn into_inner(self) -> W {
        self.stdout
    }

    fn write_json_line(&mut self, value: &Value) {
        if let Ok(line) = serde_json::to_string(value) {
            let _ = self.stdout.write_all(line.as_bytes());
            let _ = self.stdout.write_all(b"\n");
        }
    }

    fn sanitize_message_value(&mut self, message: Value) -> Value {
        if !self.suppress_reads {
            self.track_request_method(&message);
            return message;
        }

        if let Some(sanitized) = self.sanitize_read_response(&message) {
            return sanitized;
        }

        if let Some(sanitized) = self.sanitize_read_tool_message(&message) {
            return sanitized;
        }

        self.track_request_method(&message);
        message
    }

    fn track_request_method(&mut self, message: &Value) {
        let Some(root) = message.as_object() else {
            return;
        };
        let Some(method) = root.get("method").and_then(Value::as_str) else {
            return;
        };
        let Some(id_key) = root.get("id").and_then(json_rpc_id_key) else {
            return;
        };
        self.request_method_by_id.insert(id_key, method.to_string());
    }

    fn sanitize_read_response(&mut self, message: &Value) -> Option<Value> {
        let root = message.as_object()?;
        let id_key = json_rpc_id_key(root.get("id")?)?;
        let result = root.get("result")?;
        let method = self.request_method_by_id.remove(&id_key);
        if method.as_deref() != Some("fs/read_text_file") {
            return None;
        }

        let mut sanitized = root.clone();
        sanitized.insert("result".to_string(), sanitize_read_result(result));
        Some(Value::Object(sanitized))
    }

    fn sanitize_read_tool_message(&mut self, message: &Value) -> Option<Value> {
        let root = message.as_object()?;
        if root.get("method").and_then(Value::as_str) != Some("session/update") {
            return None;
        }

        let params = root.get("params")?.as_object()?;
        let update = params.get("update")?.as_object()?;
        let session_update = update.get("sessionUpdate").and_then(Value::as_str)?;
        if session_update != "tool_call" && session_update != "tool_call_update" {
            return None;
        }

        let tool_call_id = update.get("toolCallId").and_then(Value::as_str)?;
        let previous = self.tool_state_by_id.get(tool_call_id).cloned().unwrap_or_default();
        let current = ReadLikeToolDescriptor {
            title: update
                .get("title")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .or(previous.title),
            kind: match update.get("kind") {
                Some(Value::String(kind)) => Some(kind.clone()),
                Some(Value::Null) => None,
                _ => previous.kind,
            },
        };
        self.tool_state_by_id.insert(tool_call_id.to_string(), current.clone());

        if !is_read_like_tool(&current) {
            return None;
        }

        Some(sanitize_tool_message(message))
    }
}

impl<W: Write> OutputFormatter for JsonOutputFormatter<W> {
    fn set_context(&mut self, context: OutputFormatterContext) {
        let session_id = context.session_id.trim();
        if !session_id.is_empty() {
            self.session_id = session_id.to_string();
        } else if self.session_id.trim().is_empty() {
            self.session_id = DEFAULT_JSON_SESSION_ID.to_string();
        }
    }

    fn on_acp_message(&mut self, message: AcpJsonRpcMessage) {
        let value = serde_json::to_value(message).unwrap_or(Value::Object(Map::new()));
        let sanitized = self.sanitize_message_value(value);
        self.write_json_line(&sanitized);
    }

    fn on_error(&mut self, params: OutputErrorParams) {
        let response = build_json_rpc_error_response(BuildJsonRpcErrorParams {
            id: None,
            output_code: params.code,
            detail_code: params.detail_code,
            origin: params.origin,
            message: params.message,
            retryable: params.retryable,
            timestamp: params.timestamp,
            session_id: Some(self.session_id.clone()),
            acp: params.acp,
        });
        let value = serde_json::to_value(response).unwrap_or(Value::Object(Map::new()));
        self.write_json_line(&value);
    }

    fn flush(&mut self) {
        let _ = self.stdout.flush();
    }
}

pub fn create_json_output_formatter<W: Write>(
    stdout: W,
    suppress_reads: bool,
    context: Option<OutputFormatterContext>,
) -> JsonOutputFormatter<W> {
    JsonOutputFormatter::new(stdout, suppress_reads, context)
}

#[cfg(test)]
#[path = "output_json_formatter_tests.rs"]
mod output_json_formatter_tests;

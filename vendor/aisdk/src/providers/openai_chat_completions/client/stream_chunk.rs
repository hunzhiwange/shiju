use crate::error::Error;
use serde_json::Value;

use super::types::ChatCompletionsStreamChunk;

fn error_details_from_value(value: &Value) -> Option<String> {
    let error = value.get("error")?;

    if let Some(message) = error.get("message").and_then(Value::as_str) {
        return Some(message.to_string());
    }

    if let Some(message) = error.as_str() {
        return Some(message.to_string());
    }

    Some(error.to_string())
}

pub(super) fn parse_stream_chunk_json(
    data: &str,
) -> crate::error::Result<ChatCompletionsStreamChunk> {
    let mut value: Value = serde_json::from_str(data).map_err(|e| Error::ApiError {
        status_code: None,
        details: format!("Invalid JSON in SSE: {e}"),
    })?;

    if let Some(details) = error_details_from_value(&value) {
        return Err(Error::ApiError {
            status_code: None,
            details,
        });
    }

    if let Some(object) = value.as_object_mut() {
        object
            .entry("choices".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
    }

    serde_json::from_value(value).map_err(|e| Error::ApiError {
        status_code: None,
        details: format!("Invalid JSON in SSE: {e}"),
    })
}

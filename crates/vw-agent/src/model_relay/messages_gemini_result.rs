//! Resident Gemini JSON differences; strict decoding remains owned by the bridge.
use axum::http::StatusCode;
use serde_json::{Value, json};
use vw_shared::provider::ModelProtocol;

const INVALID: StatusCode = StatusCode::BAD_GATEWAY;

pub(crate) fn convert(body: &Value, model: &str) -> Result<Value, StatusCode> {
    if !body.is_object() || body.get("error").is_some_and(|v| !v.is_null()) {
        return Err(INVALID);
    }
    let mut normalized = body.clone();
    let blocked = match body.pointer("/promptFeedback/blockReason") {
        None => false,
        Some(value) => {
            value.as_str().filter(|s| !s.is_empty()).ok_or(INVALID)?;
            true
        }
    };
    if blocked {
        // Never relay a candidate or tool from a blocked prompt.
        normalized["candidates"] = json!([{"content":{"parts":[]},"finishReason":"STOP"}]);
    }
    normalized.as_object_mut().ok_or(INVALID)?.remove("promptFeedback");
    let candidates =
        normalized["candidates"].as_array_mut().filter(|a| a.len() == 1).ok_or(INVALID)?;
    let candidate = &mut candidates[0];
    if !candidate.is_object() {
        return Err(INVALID);
    }
    let refusal = blocked
        || matches!(
            candidate["finishReason"].as_str(),
            Some("SAFETY" | "RECITATION" | "SPII" | "BLOCKLIST" | "PROHIBITED_CONTENT")
        );
    if refusal {
        candidate["content"] = json!({"parts":[]});
        candidate["finishReason"] = json!("STOP");
    }
    if candidate["content"].get("role").is_some_and(|role| role != "model") {
        return Err(INVALID);
    }
    let parts = candidate["content"]["parts"].as_array_mut().ok_or(INVALID)?;
    for part in parts.iter_mut() {
        if let Some(call) = part.get_mut("functionCall") {
            let object = call.as_object_mut().ok_or(INVALID)?;
            match object.get("id") {
                None | Some(Value::Null) => {
                    object.insert(
                        "id".into(),
                        json!(format!("toolu_{}", uuid::Uuid::new_v4().simple())),
                    );
                }
                Some(Value::String(id)) if id.is_empty() => {
                    object.insert(
                        "id".into(),
                        json!(format!("toolu_{}", uuid::Uuid::new_v4().simple())),
                    );
                }
                Some(Value::String(_)) => {}
                _ => return Err(INVALID),
            }
        }
    }
    let ordered_parts = parts.clone();
    normalized["modelVersion"] = model.into();
    if normalized.get("responseId").is_none() {
        normalized["responseId"] = json!(format!("msg_{}", uuid::Uuid::new_v4().simple()));
    } else if normalized["responseId"].as_str().is_none_or(str::is_empty) {
        return Err(INVALID);
    }
    // Match the source's total-minus-prompt output accounting, including thoughts.
    let usage = body.get("usageMetadata");
    if usage.is_some_and(|u| !u.is_object()) {
        return Err(INVALID);
    }
    let count = |key: &str| -> Result<u64, StatusCode> {
        usage.and_then(|u| u.get(key)).map_or(Ok(0), |v| v.as_u64().ok_or(INVALID))
    };
    let prompt = count("promptTokenCount")?;
    let cached = count("cachedContentTokenCount")?;
    prompt.checked_sub(cached).ok_or(INVALID)?;
    let output = if usage.is_some_and(|u| u.get("totalTokenCount").is_some()) {
        count("totalTokenCount")?.checked_sub(prompt).ok_or(INVALID)?
    } else {
        count("candidatesTokenCount")?.checked_add(count("thoughtsTokenCount")?).ok_or(INVALID)?
    };
    normalized["usageMetadata"] = json!({"promptTokenCount":prompt,
        "candidatesTokenCount":output,"cachedContentTokenCount":cached});
    let chat = super::protocol_bridge::response(ModelProtocol::Gemini, &normalized)
        .map_err(|_| INVALID)?;
    let mut result = super::messages_chat_result::from_chat(&chat, model).map_err(|_| INVALID)?;
    // The shared Chat representation groups text before tools. Restore Gemini order
    // using already validated tool blocks rather than decoding tools a second time.
    let tools: Vec<_> = result["content"]
        .as_array()
        .ok_or(INVALID)?
        .iter()
        .filter(|block| block["type"] == "tool_use")
        .cloned()
        .collect();
    let mut tools = tools.into_iter();
    let mut content = Vec::new();
    for part in ordered_parts {
        if let Some(text) = part["text"].as_str() {
            if !text.is_empty() {
                content.push(json!({"type":"text","text":text}));
            }
        } else {
            content.push(tools.next().ok_or(INVALID)?);
        }
    }
    result["content"] = content.into();
    if refusal {
        result["stop_reason"] = json!("refusal");
    }
    Ok(result)
}

/// Applies only after the common bounded upstream decoder has accepted JSON.
pub(crate) async fn response(
    response: axum::response::Response,
    model: &str,
    cancel: tokio_util::sync::CancellationToken,
    max_bytes: usize,
) -> Result<axum::response::Response, StatusCode> {
    let (parts, body) = response.into_parts();
    let bytes = tokio::select! {
        biased;
        () = cancel.cancelled() => return Err(StatusCode::REQUEST_TIMEOUT),
        bytes = axum::body::to_bytes(body, max_bytes) => bytes.map_err(|_| INVALID)?,
    };
    let body: Value = serde_json::from_slice(&bytes).map_err(|_| INVALID)?;
    let body = convert(&body, model)?;
    Ok(axum::response::Response::from_parts(parts, axum::body::Body::from(body.to_string())))
}

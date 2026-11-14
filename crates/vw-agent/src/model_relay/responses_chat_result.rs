//! Resident Chat JSON differences, layered on the session result converter.
use super::responses::{self, ChatTools};
use axum::http::StatusCode;
use serde_json::{Value, json};

const INVALID: StatusCode = StatusCode::BAD_GATEWAY;

pub(super) fn from_chat(
    body: &Value,
    model: &str,
    id: &str,
    tools: &ChatTools,
) -> Result<Value, StatusCode> {
    let mut body = body.clone();
    if !body["choices"].is_array() && body["data"]["choices"].is_array() {
        let outer = body.as_object_mut().ok_or(INVALID)?;
        let mut inner = outer.remove("data").ok_or(INVALID)?;
        for (key, value) in outer.iter() {
            if !value.is_null() {
                inner.as_object_mut().ok_or(INVALID)?.insert(key.clone(), value.clone());
            }
        }
        body = inner;
    }
    if body.get("error").is_some() {
        return Err(INVALID);
    }
    let choice = body["choices"]
        .as_array_mut()
        .filter(|v| v.len() == 1)
        .and_then(|v| v.first_mut())
        .ok_or(INVALID)?;
    let finish = choice["finish_reason"].as_str().ok_or(INVALID)?.to_owned();
    choice["finish_reason"] = match finish.as_str() {
        "function_call" => json!("tool_calls"),
        "content_filter" => json!("stop"),
        "stop" | "length" | "tool_calls" => json!(finish),
        _ => return Err(INVALID),
    };
    let message = choice["message"].as_object_mut().ok_or(INVALID)?;
    let mut content = Vec::new();
    match message.remove("content").unwrap_or(Value::Null) {
        Value::Null => {}
        Value::String(text) => push_content(&mut content, "output_text", text),
        Value::Array(parts) => {
            for part in parts {
                let (kind, key) = match part["type"].as_str() {
                    Some("text" | "output_text") => ("output_text", "text"),
                    Some("refusal") => ("refusal", "refusal"),
                    _ => return Err(INVALID),
                };
                super::messages::object_keys(&part, &["type", key]).map_err(|_| INVALID)?;
                push_content(&mut content, kind, part[key].as_str().ok_or(INVALID)?.into());
            }
        }
        _ => return Err(INVALID),
    }
    if let Some(refusal) = message.remove("refusal").filter(|v| !v.is_null()) {
        push_content(&mut content, "refusal", refusal.as_str().ok_or(INVALID)?.into());
    }
    message.insert("content".into(), Value::Null);
    if let Some(legacy) = message.remove("function_call").filter(|v| !v.is_null()) {
        if message.get("tool_calls").is_some_and(|v| !v.is_null()) {
            return Err(INVALID);
        }
        message.insert(
            "tool_calls".into(),
            json!([{"type":"function","id":legacy["id"],"function":{
            "name":legacy["name"],"arguments":legacy["arguments"]}}]),
        );
    }
    if let Some(calls) = message.get_mut("tool_calls").filter(|v| !v.is_null()) {
        for (index, call) in calls.as_array_mut().ok_or(INVALID)?.iter_mut().enumerate() {
            if call["id"].as_str().is_none_or(str::is_empty) {
                call["id"] = format!("call_{index}").into();
            }
            if call["function"]["name"].as_str().is_none_or(|v| v.trim().is_empty()) {
                return Err(INVALID);
            }
            if call["function"]["arguments"].is_object() {
                call["function"]["arguments"] = call["function"]["arguments"].to_string().into();
            }
        }
    }
    let mut result = responses::from_chat(&body, model, id, tools, false).map_err(|_| INVALID)?;
    let status = result["status"].clone();
    if !content.is_empty() {
        result["output"].as_array_mut().ok_or(INVALID)?.insert(
            0,
            json!({"type":"message","id":format!("{id}_msg"),"role":"assistant",
                "status":status,"content":content}),
        );
    }
    if let Some(created) = body["created"].as_u64() {
        result["created_at"] = created.into();
    }
    Ok(result)
}

fn push_content(content: &mut Vec<Value>, kind: &str, text: String) {
    if text.is_empty() {
        return;
    }
    content.push(if kind == "refusal" {
        json!({"type":kind,"refusal":text})
    } else {
        json!({"type":kind,"text":text,"annotations":[],"logprobs":[]})
    });
}

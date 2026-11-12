//! Gemini text/function boundary. All history is request-local; no tool is executed here.
use super::messages::object_keys;
use axum::{body::Body, http::StatusCode, response::Response};
use serde_json::{Value, json};
use std::collections::BTreeMap;
const UNSUPPORTED: StatusCode = StatusCode::UNPROCESSABLE_ENTITY;
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str, StatusCode> {
    v[key].as_str().filter(|s| !s.is_empty()).ok_or(StatusCode::BAD_REQUEST)
}
fn schema(v: &Value) -> Result<Value, StatusCode> {
    let mut v = v.clone();
    let obj = v.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
    if let Some(t) = obj.get_mut("type") {
        let name = t.as_str().ok_or(UNSUPPORTED)?.to_ascii_lowercase();
        if !["object", "array", "string", "number", "integer", "boolean", "null"]
            .contains(&name.as_str())
        {
            return Err(UNSUPPORTED);
        }
        *t = name.into();
    }
    // Gemini nullable schemas cannot be silently treated as non-nullable JSON Schema.
    if obj.contains_key("nullable") || obj.contains_key("propertyOrdering") {
        return Err(UNSUPPORTED);
    }
    if let Some(properties) = obj.get_mut("properties") {
        for child in properties.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?.values_mut() {
            *child = schema(child)?;
        }
    }
    if let Some(items) = obj.get_mut("items") {
        *items = schema(items)?;
    }
    if let Some(items) = obj.get_mut("anyOf") {
        for child in items.as_array_mut().ok_or(StatusCode::BAD_REQUEST)? {
            *child = schema(child)?;
        }
    }
    Ok(v)
}
pub(super) fn to_chat(body: &Value) -> Result<Value, StatusCode> {
    object_keys(
        body,
        &[
            "model",
            "contents",
            "systemInstruction",
            "tools",
            "toolConfig",
            "generationConfig",
            "stream",
        ],
    )?;
    let mut out = json!({"model":body["model"],"messages":[]});
    let mut messages = Vec::new();
    if let Some(system) = body.get("systemInstruction") {
        object_keys(system, &["role", "parts"])?;
        let mut text = String::new();
        for part in system["parts"].as_array().ok_or(StatusCode::BAD_REQUEST)? {
            object_keys(part, &["text"])?;
            text.push_str(string(part, "text")?);
        }
        messages.push(json!({"role":"system","content":text}));
    }
    let mut pending = BTreeMap::<String, String>::new();
    let mut used = std::collections::BTreeSet::new();
    let contents =
        body["contents"].as_array().filter(|v| !v.is_empty()).ok_or(StatusCode::BAD_REQUEST)?;
    for content in contents {
        object_keys(content, &["role", "parts"])?;
        let role = match content["role"].as_str().unwrap_or("user") {
            "user" => "user",
            "model" => "assistant",
            _ => return Err(UNSUPPORTED),
        };
        let awaiting_results = !pending.is_empty();
        let mut text = String::new();
        let mut calls = Vec::new();
        let mut results = Vec::new();
        for part in content["parts"].as_array().ok_or(StatusCode::BAD_REQUEST)? {
            if let Some(t) = part.get("text") {
                object_keys(part, &["text"])?;
                if awaiting_results {
                    return Err(UNSUPPORTED);
                }
                text.push_str(t.as_str().ok_or(StatusCode::BAD_REQUEST)?);
            } else if let Some(call) = part.get("functionCall") {
                object_keys(part, &["functionCall"])?;
                object_keys(call, &["id", "name", "args"])?;
                let args = call.get("args").cloned().unwrap_or(json!({}));
                if role != "assistant" || !args.is_object() || awaiting_results {
                    return Err(StatusCode::BAD_REQUEST);
                }
                let name = string(call, "name")?;
                let id = match call.get("id") {
                    Some(_) => string(call, "id")?.to_owned(),
                    None => format!("gemini_call_{}", used.len()),
                };
                if !used.insert(id.clone()) {
                    return Err(StatusCode::BAD_REQUEST);
                }
                pending.insert(id.clone(), name.into());
                calls.push(json!({"id":id,"type":"function","function":{"name":name,"arguments":args.to_string()}}));
            } else if let Some(result) = part.get("functionResponse") {
                object_keys(part, &["functionResponse"])?;
                object_keys(result, &["id", "name", "response"])?;
                if role != "user" || !result["response"].is_object() {
                    return Err(StatusCode::BAD_REQUEST);
                }
                let name = string(result, "name")?;
                let id = if result.get("id").is_some() {
                    string(result, "id")?.to_owned()
                } else {
                    let ids = pending
                        .iter()
                        .filter(|(_, n)| n.as_str() == name)
                        .map(|(id, _)| id.clone())
                        .collect::<Vec<_>>();
                    if ids.len() != 1 {
                        return Err(UNSUPPORTED);
                    }
                    ids[0].clone()
                };
                if pending.remove(&id).as_deref() != Some(name) {
                    return Err(StatusCode::BAD_REQUEST);
                }
                results.push(json!({"role":"tool","tool_call_id":id,"content":result["response"].to_string()}));
            } else {
                return Err(UNSUPPORTED);
            }
        }
        if !results.is_empty() {
            if !text.is_empty() || !calls.is_empty() {
                return Err(UNSUPPORTED);
            }
            messages.extend(results);
        } else {
            let mut message = json!({"role":role,"content":text});
            if !calls.is_empty() {
                message["tool_calls"] = calls.into();
            }
            messages.push(message);
        }
    }
    if !pending.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    out["messages"] = messages.into();
    if let Some(config) = body.get("generationConfig") {
        object_keys(
            config,
            &[
                "temperature",
                "topP",
                "maxOutputTokens",
                "stopSequences",
                "candidateCount",
                "responseMimeType",
                "responseSchema",
                "responseJsonSchema",
            ],
        )?;
        if config.get("candidateCount").is_some_and(|v| v != 1) {
            return Err(UNSUPPORTED);
        }
        for (source, target) in [
            ("temperature", "temperature"),
            ("topP", "top_p"),
            ("maxOutputTokens", "max_tokens"),
            ("stopSequences", "stop"),
        ] {
            if let Some(v) = config.get(source) {
                out[target] = v.clone();
            }
        }
        match config["responseMimeType"].as_str() {
            None | Some("text/plain") => {
                if config.get("responseSchema").is_some()
                    || config.get("responseJsonSchema").is_some()
                {
                    return Err(UNSUPPORTED);
                }
            }
            Some("application/json") => {
                if config.get("responseSchema").is_some()
                    && config.get("responseJsonSchema").is_some()
                {
                    return Err(StatusCode::BAD_REQUEST);
                }
                out["response_format"] = if let Some(s) = config.get("responseSchema") {
                    json!({"type":"json_schema","json_schema":{"name":"response","schema":schema(s)?}})
                } else if let Some(s) = config.get("responseJsonSchema") {
                    json!({"type":"json_schema","json_schema":{"name":"response","schema":s}})
                } else {
                    json!({"type":"json_object"})
                };
            }
            _ => return Err(UNSUPPORTED),
        }
    }
    if let Some(tools) = body.get("tools") {
        let mut functions = Vec::new();
        for tool in tools.as_array().ok_or(StatusCode::BAD_REQUEST)? {
            object_keys(tool, &["functionDeclarations"])?;
            for f in tool["functionDeclarations"].as_array().ok_or(StatusCode::BAD_REQUEST)? {
                object_keys(f, &["name", "description", "parameters", "parametersJsonSchema"])?;
                if f.get("parameters").is_some() && f.get("parametersJsonSchema").is_some() {
                    return Err(StatusCode::BAD_REQUEST);
                }
                let mut function = json!({"name":string(f,"name")?});
                if let Some(d) = f.get("description") {
                    function["description"] = d.clone();
                }
                function["parameters"] = if let Some(s) = f.get("parameters") {
                    schema(s)?
                } else {
                    f.get("parametersJsonSchema")
                        .cloned()
                        .unwrap_or(json!({"type":"object","properties":{}}))
                };
                functions.push(json!({"type":"function","function":function}));
            }
        }
        out["tools"] = functions.into();
    }
    if let Some(config) = body.get("toolConfig") {
        object_keys(config, &["functionCallingConfig"])?;
        let c = &config["functionCallingConfig"];
        object_keys(c, &["mode", "allowedFunctionNames"])?;
        out["tool_choice"] = match c["mode"].as_str().unwrap_or("AUTO") {
            "AUTO" => json!("auto"),
            "NONE" => json!("none"),
            "ANY" => {
                if let Some(names) = c.get("allowedFunctionNames") {
                    let names = names.as_array().filter(|v| v.len() == 1).ok_or(UNSUPPORTED)?;
                    json!({"type":"function","function":{"name":names[0].as_str().ok_or(StatusCode::BAD_REQUEST)?}})
                } else {
                    json!("required")
                }
            }
            _ => return Err(UNSUPPORTED),
        };
        if c.get("allowedFunctionNames").is_some() && c["mode"] != "ANY" {
            return Err(UNSUPPORTED);
        }
    }
    if body["stream"] == true {
        out["stream"] = true.into();
        out["stream_options"] = json!({"include_usage":true});
    }
    Ok(out)
}
pub(super) fn from_chat(body: &Value, model: &str) -> Result<Value, StatusCode> {
    let choices =
        body["choices"].as_array().filter(|v| v.len() == 1).ok_or(StatusCode::BAD_GATEWAY)?;
    let choice = &choices[0];
    let message = &choice["message"];
    object_keys(message, &["role", "content", "tool_calls", "refusal"])?;
    if !message["refusal"].is_null() {
        return Err(UNSUPPORTED);
    }
    let mut parts = Vec::new();
    if let Some(text) = message["content"].as_str() {
        if !text.is_empty() {
            parts.push(json!({"text":text}));
        }
    } else if !message["content"].is_null() {
        return Err(UNSUPPORTED);
    }
    if let Some(calls) = message.get("tool_calls") {
        let mut ids = std::collections::BTreeSet::new();
        for call in calls.as_array().ok_or(StatusCode::BAD_GATEWAY)? {
            object_keys(call, &["id", "type", "function"])?;
            object_keys(&call["function"], &["name", "arguments"])?;
            if call["type"] != "function" {
                return Err(UNSUPPORTED);
            }
            let id = string(call, "id")?;
            if !ids.insert(id) {
                return Err(StatusCode::BAD_GATEWAY);
            }
            let args: Value = serde_json::from_str(string(&call["function"], "arguments")?)
                .map_err(|_| StatusCode::BAD_GATEWAY)?;
            if !args.is_object() {
                return Err(UNSUPPORTED);
            }
            parts.push(json!({"functionCall":{"id":id,"name":string(&call["function"],"name")?,"args":args}}));
        }
    }
    let reason = match choice["finish_reason"].as_str() {
        Some("stop" | "tool_calls") => "STOP",
        Some("length") => "MAX_TOKENS",
        Some("content_filter") => "SAFETY",
        _ => return Err(UNSUPPORTED),
    };
    let mut output = json!({"candidates":[{"index":0,"content":{"role":"model","parts":parts},"finishReason":reason}],"modelVersion":model});
    if let Some(usage) = body.get("usage").filter(|v| !v.is_null()) {
        let mut mapped = json!({});
        for (source, target) in [
            ("prompt_tokens", "promptTokenCount"),
            ("completion_tokens", "candidatesTokenCount"),
            ("total_tokens", "totalTokenCount"),
        ] {
            if let Some(n) = usage.get(source) {
                if !n.is_u64() {
                    return Err(StatusCode::BAD_GATEWAY);
                }
                mapped[target] = n.clone();
            }
        }
        for (source, target) in [
            ("prompt_tokens_details", "cachedContentTokenCount"),
            ("completion_tokens_details", "thoughtsTokenCount"),
        ] {
            let key = if source == "prompt_tokens_details" {
                "cached_tokens"
            } else {
                "reasoning_tokens"
            };
            if let Some(n) = usage[source].get(key) {
                if !n.is_u64() {
                    return Err(StatusCode::BAD_GATEWAY);
                }
                mapped[target] = n.clone();
            }
        }
        if let Some(thoughts) = mapped["thoughtsTokenCount"].as_u64() {
            let total = mapped["candidatesTokenCount"].as_u64().ok_or(StatusCode::BAD_GATEWAY)?;
            mapped["candidatesTokenCount"] =
                total.checked_sub(thoughts).ok_or(StatusCode::BAD_GATEWAY)?.into();
        }
        output["usageMetadata"] = mapped;
    }
    Ok(output)
}
pub(super) fn native(
    body: &Value,
    model: &crate::provider::provider::Model,
) -> Result<Value, StatusCode> {
    let mut body = body.clone();
    let obj = body.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
    // Cache names carry upstream tenant state; there is no authorized reference mapping here.
    if obj.contains_key("cachedContent") || obj.contains_key("continuationToken") {
        return Err(UNSUPPORTED);
    }
    let config = &body["generationConfig"];
    if config.get("temperature").is_some() && !model.capabilities.temperature {
        return Err(UNSUPPORTED);
    }
    if let Some(count) = config.get("maxOutputTokens") {
        let count = count.as_u64().filter(|n| *n > 0).ok_or(StatusCode::BAD_REQUEST)?;
        if model.limit.output > 0 && count > model.limit.output {
            return Err(UNSUPPORTED);
        }
    }
    let mut tool_request =
        body.get("tools").is_some_and(|v| v.as_array().is_none_or(|v| !v.is_empty()));
    for content in
        body["contents"].as_array().filter(|v| !v.is_empty()).ok_or(StatusCode::BAD_REQUEST)?
    {
        for part in content["parts"].as_array().ok_or(StatusCode::BAD_REQUEST)? {
            if !part.is_object() {
                return Err(StatusCode::BAD_REQUEST);
            }
            tool_request |=
                part.get("functionCall").is_some() || part.get("functionResponse").is_some();
            if part.get("text").is_some() && !model.capabilities.input.text {
                return Err(UNSUPPORTED);
            }
            // Uploaded Files names are upstream state, just like cachedContent. No tenant mapping.
            if part.get("fileData").is_some() {
                return Err(UNSUPPORTED);
            }
            if let Some(data) = part.get("inlineData") {
                let mime = string(data, "mimeType")?;
                let allowed = if mime.starts_with("image/") {
                    model.capabilities.input.image
                } else if mime.starts_with("audio/") {
                    model.capabilities.input.audio
                } else if mime.starts_with("video/") {
                    model.capabilities.input.video
                } else if mime == "application/pdf" {
                    model.capabilities.input.pdf
                } else {
                    false
                };
                if !allowed {
                    return Err(UNSUPPORTED);
                }
            }
        }
    }
    if tool_request && !model.capabilities.toolcall {
        return Err(UNSUPPORTED);
    }
    let obj = body.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
    obj.remove("model");
    obj.remove("stream");
    Ok(body)
}
pub(super) fn output(mut value: Value, model: &str) -> Result<Value, StatusCode> {
    if value.get("error").is_some()
        || (!value["candidates"].is_array() && value.get("promptFeedback").is_none())
    {
        return Err(StatusCode::BAD_GATEWAY);
    }
    value["modelVersion"] = model.into();
    // Response IDs are not accepted as input; do not reveal reusable upstream reference names.
    value.as_object_mut().ok_or(StatusCode::BAD_GATEWAY)?.remove("responseId");
    Ok(value)
}
pub(super) fn error(status: StatusCode) -> Response {
    let name = match status.as_u16() {
        401 => "UNAUTHENTICATED",
        403 => "PERMISSION_DENIED",
        404 => "NOT_FOUND",
        429 => "RESOURCE_EXHAUSTED",
        502 => "UNAVAILABLE",
        504 => "DEADLINE_EXCEEDED",
        _ => "INVALID_ARGUMENT",
    };
    Response::builder().status(status).header("content-type","application/json").body(Body::from(json!({"error":{"code":status.as_u16(),"status":name,"message":"Gemini model relay request rejected"}}).to_string())).expect("static response")
}

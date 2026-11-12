//! Decode materialized native outputs. Opaque state and partial tools fail closed.
use super::*;
fn tool(id: &str, name: &str, args: Value) -> Result<Value> {
    if id.is_empty() || name.is_empty() || !args.is_object() {
        return Err(UPSTREAM);
    }
    Ok(json!({"id":id,"type":"function","function":{"name":name,"arguments":args.to_string()}}))
}
fn count(v: &Value, key: &str) -> Result<u64> {
    v[key].as_u64().ok_or(UPSTREAM)
}
fn usage(v: &Value, input: &str, output: &str) -> Result<Value> {
    Ok(json!({"prompt_tokens":count(v,input)?,"completion_tokens":count(v,output)?}))
}
pub(super) fn messages(v: &Value) -> Result<Value> {
    if v["role"] != "assistant" || v["type"] != "message" {
        return Err(UPSTREAM);
    }
    let mut content = String::new();
    let mut calls = Vec::new();
    for part in v["content"].as_array().ok_or(UPSTREAM)? {
        match part["type"].as_str() {
            Some("text") => {
                keys(part, &["type", "text", "citations"])?;
                if part.get("citations").is_some_and(|v| v.as_array().is_none_or(|a| !a.is_empty()))
                {
                    return Err(UNSUPPORTED);
                }
                content.push_str(part["text"].as_str().ok_or(UPSTREAM)?);
            }
            Some("tool_use") => {
                keys(part, &["type", "id", "name", "input"])?;
                calls.push(tool(
                    string(part, "id")?,
                    string(part, "name")?,
                    part["input"].clone(),
                )?);
            }
            _ => return Err(UNSUPPORTED),
        }
    }
    let reason = match v["stop_reason"].as_str() {
        Some("tool_use") => "tool_calls",
        Some("end_turn" | "stop_sequence") => "stop",
        Some("max_tokens") => "length",
        Some("refusal") => "content_filter",
        _ => return Err(UPSTREAM),
    };
    let mut mapped = usage(&v["usage"], "input_tokens", "output_tokens")?;
    let mut prompt = count(&v["usage"], "input_tokens")?;
    for field in ["cache_read_input_tokens", "cache_creation_input_tokens"] {
        if let Some(n) = v["usage"].get(field) {
            prompt = prompt.checked_add(n.as_u64().ok_or(UPSTREAM)?).ok_or(UPSTREAM)?;
        }
    }
    mapped["prompt_tokens"] = prompt.into();
    if let Some(n) = v["usage"].get("cache_read_input_tokens") {
        mapped["prompt_tokens_details"] = json!({"cached_tokens":n});
    }
    chat_result(string(v, "id")?, string(v, "model")?, content, calls, reason, mapped)
}
pub(super) fn responses(v: &Value) -> Result<Value> {
    let reason = match v["status"].as_str() {
        Some("completed") => "stop",
        Some("incomplete") if v["incomplete_details"]["reason"] == "max_output_tokens" => "length",
        _ => return Err(UPSTREAM),
    };
    let mut content = String::new();
    let mut calls = Vec::new();
    for item in v["output"].as_array().ok_or(UPSTREAM)? {
        match item["type"].as_str() {
            Some("message") => {
                if item["role"] != "assistant" {
                    return Err(UPSTREAM);
                }
                for part in item["content"].as_array().ok_or(UPSTREAM)? {
                    keys(part, &["type", "text", "annotations", "logprobs"])?;
                    if part["type"] != "output_text"
                        || ["annotations", "logprobs"].iter().any(|k| {
                            part.get(*k).is_some_and(|v| v.as_array().is_none_or(|a| !a.is_empty()))
                        })
                    {
                        return Err(UNSUPPORTED);
                    }
                    content.push_str(part["text"].as_str().ok_or(UPSTREAM)?);
                }
            }
            Some("function_call") => {
                if item.get("namespace").is_some_and(|v| !v.is_null()) {
                    return Err(UNSUPPORTED);
                }
                let args: Value =
                    serde_json::from_str(string(item, "arguments")?).map_err(|_| UPSTREAM)?;
                let mut call = tool(string(item, "call_id")?, string(item, "name")?, args)?;
                // Keep the provider's argument bytes, including number precision/formatting.
                call["function"]["arguments"] = item["arguments"].clone();
                calls.push(call);
            }
            _ => return Err(UNSUPPORTED),
        }
    }
    let reason = if reason == "stop" && !calls.is_empty() { "tool_calls" } else { reason };
    let mut mapped = usage(&v["usage"], "input_tokens", "output_tokens")?;
    for (source, target) in [
        ("input_tokens_details", "prompt_tokens_details"),
        ("output_tokens_details", "completion_tokens_details"),
    ] {
        if let Some(n) = v["usage"].get(source) {
            mapped[target] = n.clone();
        }
    }
    chat_result(string(v, "id")?, string(v, "model")?, content, calls, reason, mapped)
}
pub(super) fn gemini(v: &Value) -> Result<Value> {
    if v.get("promptFeedback").is_some_and(|f| f.get("blockReason").is_some()) {
        return Err(UPSTREAM);
    }
    let candidates = v["candidates"].as_array().filter(|a| a.len() == 1).ok_or(UPSTREAM)?;
    let c = &candidates[0];
    if c.get("index").is_some_and(|n| n != 0) {
        return Err(UNSUPPORTED);
    }
    let mut content = String::new();
    let mut calls = Vec::new();
    for (index, part) in c["content"]["parts"].as_array().ok_or(UPSTREAM)?.iter().enumerate() {
        keys(part, &["text", "functionCall", "thought", "thoughtSignature"])?;
        if part.get("thought").is_some_and(|v| v != false)
            || part.get("thoughtSignature").is_some_and(|v| !v.is_null())
        {
            return Err(UNSUPPORTED);
        }
        if let Some(t) = part.get("text") {
            if part.get("functionCall").is_some() {
                return Err(UPSTREAM);
            }
            content.push_str(t.as_str().ok_or(UPSTREAM)?);
        } else if let Some(call) = part.get("functionCall") {
            keys(call, &["id", "name", "args"])?;
            let id = call["id"].as_str().map(str::to_owned).unwrap_or_else(|| {
                format!("gemini_{}_{}", v["responseId"].as_str().unwrap_or("call"), index)
            });
            calls.push(tool(
                &id,
                string(call, "name")?,
                call.get("args").cloned().unwrap_or(json!({})),
            )?);
        } else {
            return Err(UNSUPPORTED);
        }
    }
    let reason = match c["finishReason"].as_str() {
        Some("STOP") if !calls.is_empty() => "tool_calls",
        Some("STOP") => "stop",
        Some("MAX_TOKENS") => "length",
        Some("SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII") => {
            "content_filter"
        }
        _ => return Err(UPSTREAM),
    };
    let mut mapped = usage(&v["usageMetadata"], "promptTokenCount", "candidatesTokenCount")?;
    if let Some(n) = v["usageMetadata"].get("thoughtsTokenCount") {
        let thoughts = n.as_u64().ok_or(UPSTREAM)?;
        let output = mapped["completion_tokens"].as_u64().ok_or(UPSTREAM)?;
        mapped["completion_tokens"] = output.checked_add(thoughts).ok_or(UPSTREAM)?.into();
        mapped["completion_tokens_details"] = json!({"reasoning_tokens":thoughts});
    }
    if let Some(n) = v["usageMetadata"].get("cachedContentTokenCount") {
        if !n.is_u64() {
            return Err(UPSTREAM);
        }
        mapped["prompt_tokens_details"] = json!({"cached_tokens":n});
    }
    chat_result(
        v["responseId"].as_str().unwrap_or("gemini-response"),
        string(v, "modelVersion")?,
        content,
        calls,
        reason,
        mapped,
    )
}

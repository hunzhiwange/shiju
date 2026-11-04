//! Responses text/function boundary. References are opaque, expiring, and identity-bound.
use super::{RelayIdentity, messages::object_keys};
use axum::{http::StatusCode, response::IntoResponse};
use serde_json::{Value, json};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

const UNSUPPORTED: StatusCode = StatusCode::UNPROCESSABLE_ENTITY;
const REFERENCE_LIMIT: usize = 4096;
struct Reference {
    identity: RelayIdentity,
    public: String,
    upstream: String,
    expires: Instant,
    item: bool,
}
/// No Debug/Serialize: this is transient provider state, never persisted as session history.
#[derive(Default)]
pub(super) struct References(parking_lot::Mutex<Vec<Reference>>);
impl References {
    pub(super) fn resolve(
        &self,
        identity: &RelayIdentity,
        public: &str,
    ) -> Result<String, StatusCode> {
        let mut entries = self.0.lock();
        entries.retain(|entry| entry.expires > Instant::now());
        entries
            .iter()
            .find(|entry| &entry.identity == identity && entry.public == public && !entry.item)
            .map(|entry| entry.upstream.clone())
            .ok_or(StatusCode::FORBIDDEN)
    }
    fn public(&self, identity: &RelayIdentity, upstream: &str) -> Result<String, StatusCode> {
        let mut entries = self.0.lock();
        entries.retain(|entry| entry.expires > Instant::now());
        entries
            .iter()
            .find(|entry| &entry.identity == identity && entry.upstream == upstream && !entry.item)
            .map(|entry| entry.public.clone())
            .ok_or(StatusCode::BAD_GATEWAY)
    }
    pub(super) fn remember(
        &self,
        identity: &RelayIdentity,
        public: &str,
        upstream: &str,
    ) -> Result<(), StatusCode> {
        self.insert(identity, public, upstream, false)
    }
    fn check_item(&self, identity: &RelayIdentity, id: &str) -> Result<(), StatusCode> {
        let mut entries = self.0.lock();
        entries.retain(|entry| entry.expires > Instant::now());
        if entries
            .iter()
            .any(|entry| entry.item && &entry.identity == identity && entry.upstream == id)
        {
            Ok(())
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }
    fn insert(
        &self,
        identity: &RelayIdentity,
        public: &str,
        upstream: &str,
        item: bool,
    ) -> Result<(), StatusCode> {
        if public.len() > 1024 || upstream.is_empty() || upstream.len() > 1024 {
            return Err(StatusCode::BAD_GATEWAY);
        }
        let mut entries = self.0.lock();
        entries.retain(|entry| entry.expires > Instant::now());
        if item
            && entries.iter().any(|entry| {
                entry.item && &entry.identity == identity && entry.upstream == upstream
            })
        {
            return Ok(());
        }
        if entries.len() >= REFERENCE_LIMIT {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        entries.push(Reference {
            identity: identity.clone(),
            public: public.into(),
            upstream: upstream.into(),
            expires: Instant::now() + Duration::from_secs(900),
            item,
        });
        Ok(())
    }
}
pub(super) fn response_id() -> String {
    format!("resp_{}", uuid::Uuid::new_v4().simple())
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, StatusCode> {
    value[key].as_str().filter(|v| !v.is_empty()).ok_or(StatusCode::BAD_REQUEST)
}
fn arguments(value: &Value) -> Result<(), StatusCode> {
    let parsed: Value =
        serde_json::from_str(string(value, "arguments")?).map_err(|_| StatusCode::BAD_REQUEST)?;
    if !parsed.is_object() {
        return Err(UNSUPPORTED);
    }
    Ok(())
}
fn content(value: &Value, role: &str) -> Result<String, StatusCode> {
    if let Some(text) = value.as_str() {
        return Ok(text.into());
    }
    let mut text = String::new();
    for part in value.as_array().ok_or(StatusCode::BAD_REQUEST)? {
        object_keys(part, &["type", "text", "annotations"])?;
        if part["type"] != "input_text" && !(role == "assistant" && part["type"] == "output_text") {
            return Err(UNSUPPORTED);
        }
        if part.get("annotations").is_some_and(|v| v.as_array().is_none_or(|v| !v.is_empty())) {
            return Err(UNSUPPORTED);
        }
        text.push_str(part["text"].as_str().ok_or(StatusCode::BAD_REQUEST)?);
    }
    Ok(text)
}
pub(super) fn to_chat(body: &Value) -> Result<Value, StatusCode> {
    object_keys(
        body,
        &[
            "model",
            "input",
            "instructions",
            "max_output_tokens",
            "stream",
            "temperature",
            "top_p",
            "tools",
            "tool_choice",
            "parallel_tool_calls",
            "store",
            "previous_response_id",
            "text",
        ],
    )?;
    if body.get("previous_response_id").is_some_and(|v| !v.is_null())
        || body.get("store").is_some_and(|v| v != false)
    {
        return Err(UNSUPPORTED);
    }
    let mut output = json!({"model":body["model"],"messages":[]});
    for key in ["stream", "temperature", "top_p", "parallel_tool_calls"] {
        if let Some(value) = body.get(key) {
            output[key] = value.clone();
        }
    }
    for key in ["stream", "parallel_tool_calls"] {
        if output.get(key).is_some_and(|v| !v.is_boolean()) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    if output["stream"] == true {
        output["stream_options"] = json!({"include_usage":true});
    }
    if let Some(max) = body.get("max_output_tokens") {
        output["max_tokens"] = max.clone();
    }
    if let Some(text) = body.get("text") {
        object_keys(text, &["format"])?;
        let format = &text["format"];
        match string(format, "type")? {
            "text" => {
                object_keys(format, &["type"])?;
            }
            "json_object" => {
                object_keys(format, &["type"])?;
                output["response_format"] = format.clone();
            }
            "json_schema" => {
                object_keys(format, &["type", "name", "description", "schema", "strict"])?;
                if !format["schema"].is_object() {
                    return Err(StatusCode::BAD_REQUEST);
                }
                let mut schema = format.clone();
                schema.as_object_mut().unwrap().remove("type");
                output["response_format"] = json!({"type":"json_schema","json_schema":schema});
            }
            _ => return Err(UNSUPPORTED),
        }
    }
    let mut messages = Vec::new();
    if let Some(instructions) = body.get("instructions").filter(|v| !v.is_null()) {
        messages.push(json!({"role":"system","content":instructions.as_str().ok_or(StatusCode::BAD_REQUEST)?}));
    }
    if let Some(input) = body["input"].as_str() {
        messages.push(json!({"role":"user","content":input}));
    } else {
        for item in body["input"].as_array().ok_or(StatusCode::BAD_REQUEST)? {
            match item["type"].as_str() {
                None | Some("message") => {
                    object_keys(item, &["type", "id", "role", "content", "status"])?;
                    let role = string(item, "role")?;
                    if !matches!(role, "system" | "developer" | "user" | "assistant") {
                        return Err(UNSUPPORTED);
                    }
                    messages.push(json!({"role":role,"content":content(&item["content"],role)?}));
                }
                Some("function_call") => {
                    object_keys(item, &["type", "id", "call_id", "name", "arguments", "status"])?;
                    arguments(item)?;
                    let call = json!({"id":string(item,"call_id")?,"type":"function","function":{"name":string(item,"name")?,"arguments":item["arguments"]}});
                    if let Some(last) = messages.last_mut().filter(|m| m["role"] == "assistant") {
                        if last.get("tool_calls").is_none() {
                            last["tool_calls"] = json!([]);
                        }
                        last["tool_calls"]
                            .as_array_mut()
                            .ok_or(StatusCode::BAD_REQUEST)?
                            .push(call);
                    } else {
                        messages
                            .push(json!({"role":"assistant","content":null,"tool_calls":[call]}));
                    }
                }
                Some("function_call_output") => {
                    object_keys(item, &["type", "id", "call_id", "output", "status"])?;
                    messages.push(json!({"role":"tool","tool_call_id":string(item,"call_id")?,"content":item["output"].as_str().ok_or(UNSUPPORTED)?}));
                }
                _ => return Err(UNSUPPORTED),
            }
        }
    }
    if messages.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    output["messages"] = messages.into();
    if let Some(tools) = body.get("tools") {
        let mut converted = Vec::new();
        for tool in tools.as_array().ok_or(StatusCode::BAD_REQUEST)? {
            object_keys(tool, &["type", "name", "description", "parameters", "strict"])?;
            if tool["type"] != "function" {
                return Err(UNSUPPORTED);
            }
            string(tool, "name")?;
            if !tool["parameters"].is_object() {
                return Err(StatusCode::BAD_REQUEST);
            }
            let mut function = tool.clone();
            function.as_object_mut().unwrap().remove("type");
            converted.push(json!({"type":"function","function":function}));
        }
        output["tools"] = converted.into();
    }
    if let Some(choice) = body.get("tool_choice") {
        output["tool_choice"] = if let Some(choice) = choice.as_str() {
            if !matches!(choice, "auto" | "none" | "required") {
                return Err(UNSUPPORTED);
            }
            json!(choice)
        } else {
            object_keys(choice, &["type", "name"])?;
            if choice["type"] != "function" {
                return Err(UNSUPPORTED);
            }
            json!({"type":"function","function":{"name":string(choice,"name")?}})
        };
    }
    Ok(output)
}

pub(super) fn from_chat(body: &Value, model: &str, id: &str) -> Result<Value, StatusCode> {
    // Reuse the strict Chat output validator; reasoning, hosted tools and citations cannot disappear.
    let message = super::messages::from_chat(body, model)?;
    let status = if message["stop_reason"] == "max_tokens" { "incomplete" } else { "completed" };
    let mut output = Vec::new();
    for block in message["content"].as_array().ok_or(StatusCode::BAD_GATEWAY)? {
        let index = output.len();
        output.push(match block["type"].as_str(){
            Some("text")=>json!({"type":"message","id":format!("{id}_msg_{index}"),"role":"assistant","status":status,"content":[{"type":"output_text","text":block["text"],"annotations":[],"logprobs":[]}]}),
            Some("tool_use")=>{
                // Keep the original JSON argument string, including formatting and number precision.
                let call=body["choices"][0]["message"]["tool_calls"].as_array().and_then(|calls|calls.iter().find(|call|call["id"]==block["id"])).ok_or(StatusCode::BAD_GATEWAY)?;
                json!({"type":"function_call","id":format!("{id}_fc_{index}"),"status":status,"call_id":block["id"],"name":block["name"],"arguments":call["function"]["arguments"]})
            },
            _=>return Err(UNSUPPORTED)
        });
    }
    let input = message["usage"]["input_tokens"].as_u64().ok_or(StatusCode::BAD_GATEWAY)?;
    let tokens = message["usage"]["output_tokens"].as_u64().ok_or(StatusCode::BAD_GATEWAY)?;
    let mut usage = json!({"input_tokens":input,"output_tokens":tokens,"total_tokens":input.checked_add(tokens).ok_or(StatusCode::BAD_GATEWAY)?});
    for (source, target) in [
        ("prompt_tokens_details", "input_tokens_details"),
        ("completion_tokens_details", "output_tokens_details"),
    ] {
        if let Some(details) = body["usage"].get(source) {
            usage[target] = details.clone();
        }
    }
    Ok(
        json!({"id":id,"object":"response","created_at":chrono::Utc::now().timestamp(),"model":model,"status":status,"output":output,"error":null,"incomplete_details":if status=="incomplete" {json!({"reason":"max_output_tokens"})}else{Value::Null},"usage":usage,"store":false}),
    )
}

pub(super) fn is_native(model: &crate::provider::provider::Model) -> bool {
    model.api.adapter == "openai-responses"
        || matches!(model.api.adapter.as_str(), "openai" | "openai-compatible")
            && reqwest::Url::parse(&model.api.url)
                .is_ok_and(|url| url.path().trim_end_matches('/').ends_with("/responses"))
}
pub(super) fn native(
    model: &crate::provider::provider::Model,
    identity: &RelayIdentity,
    bytes: &[u8],
    references: &References,
) -> Result<Value, StatusCode> {
    let mut body: Value = serde_json::from_slice(bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    let object = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    if object.get("model").and_then(Value::as_str) != Some(identity.model.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }
    if [
        "base_url",
        "api_base",
        "api_key",
        "authorization",
        "headers",
        "url",
        "provider",
        "tenant",
        "session",
        "generation",
    ]
    .iter()
    .any(|key| object.contains_key(*key))
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    // Conversation/item references provide alternate upstream state lookup paths, so reject them.
    if body.get("conversation").is_some_and(|v| !v.is_null())
        || body.get("background").is_some_and(|v| v != false)
    {
        return Err(UNSUPPORTED);
    }
    if body.get("stream").is_some_and(|v| !v.is_boolean())
        || body.get("store").is_some_and(|v| !v.is_boolean())
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    if body.get("temperature").is_some() && !model.capabilities.temperature
        || body.get("reasoning").is_some() && !model.capabilities.reasoning
        || body.get("tools").is_some() && !model.capabilities.toolcall
    {
        return Err(UNSUPPORTED);
    }
    if let Some(max) = body.get("max_output_tokens") {
        let max = max.as_u64().filter(|n| *n > 0).ok_or(StatusCode::BAD_REQUEST)?;
        if model.limit.output > 0 && max > model.limit.output {
            return Err(UNSUPPORTED);
        }
    }
    if let Some(items) = body["input"].as_array() {
        for item in items {
            if let Some(id) = item.get("id") {
                references.check_item(identity, id.as_str().ok_or(StatusCode::BAD_REQUEST)?)?;
            }
            match item["type"].as_str() {
                Some("item_reference") if !item["id"].is_string() => {
                    return Err(StatusCode::BAD_REQUEST);
                }
                None if !item["role"].is_string() => return Err(UNSUPPORTED),
                Some("reasoning") if !model.capabilities.reasoning => return Err(UNSUPPORTED),
                Some("function_call" | "function_call_output") if !model.capabilities.toolcall => {
                    return Err(UNSUPPORTED);
                }
                _ => {}
            }
            if item["content"].is_string() && !model.capabilities.input.text {
                return Err(UNSUPPORTED);
            }
            if let Some(parts) = item["content"].as_array() {
                for part in parts {
                    let supported = match part["type"].as_str() {
                        Some("input_text" | "output_text") => model.capabilities.input.text,
                        Some("input_image") => model.capabilities.input.image,
                        _ => false,
                    };
                    if !supported {
                        return Err(UNSUPPORTED);
                    }
                }
            }
        }
    } else if !body["input"].is_string() {
        return Err(StatusCode::BAD_REQUEST);
    } else if !model.capabilities.input.text {
        return Err(UNSUPPORTED);
    }
    if let Some(previous) = body.get("previous_response_id").filter(|v| !v.is_null()) {
        body["previous_response_id"] =
            references.resolve(identity, previous.as_str().ok_or(StatusCode::BAD_REQUEST)?)?.into();
    }
    body["model"] = model.api.id.clone().into();
    Ok(body)
}

pub(super) fn native_output(
    mut body: Value,
    identity: &RelayIdentity,
    id: &str,
    references: &Arc<References>,
    remember: bool,
) -> Result<Value, StatusCode> {
    if body["object"] != "response"
        || !matches!(
            body["status"].as_str(),
            Some("queued" | "in_progress" | "completed" | "incomplete")
        )
    {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let upstream = string(&body, "id")?.to_owned();
    if let Some(previous) = body.get("previous_response_id").filter(|v| !v.is_null()) {
        body["previous_response_id"] =
            references.public(identity, previous.as_str().ok_or(StatusCode::BAD_GATEWAY)?)?.into();
    }
    if remember {
        for item in body["output"].as_array().ok_or(StatusCode::BAD_GATEWAY)? {
            if let Some(item_id) = item.get("id") {
                let item_id = item_id.as_str().ok_or(StatusCode::BAD_GATEWAY)?;
                references.insert(identity, item_id, item_id, true)?;
            }
        }
        references.remember(identity, id, &upstream)?;
    }
    body["id"] = id.into();
    body["model"] = identity.model.clone().into();
    Ok(body)
}
pub(super) fn error(status: StatusCode) -> axum::response::Response {
    let kind = match status.as_u16() {
        400 | 413 | 422 => "invalid_request_error",
        401 => "authentication_error",
        403 => "permission_error",
        429 => "rate_limit_error",
        _ => "api_error",
    };
    (status,axum::Json(json!({"error":{"type":kind,"code":if status==UNSUPPORTED{"unsupported_capability"}else{"relay_error"},"param":null,"message":if status==UNSUPPORTED{"Unsupported Responses capability"}else{"Responses relay request failed"}}}))).into_response()
}

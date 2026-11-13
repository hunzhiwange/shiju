//! Bounded, request-local Chat SSE decoder. Parallel tool fragments are serialized at completion.
use super::messages::{finish, from_chat, object_keys};
use axum::{
    body::{Body, Bytes},
    http::StatusCode,
    response::Response,
};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;

const LIMIT: usize = 4 * 1024 * 1024;
#[cfg(test)]
#[path = "messages_stream_tests.rs"]
mod tests;
fn event(value: Value) -> String {
    format!("event: {}\ndata: {value}\n\n", value["type"].as_str().unwrap_or("error"))
}
#[derive(Default)]
struct Decoder {
    observer: super::stream_observer::StreamObserver,
    pending: super::sse_framing::Lines,
    data: String,
    started: bool,
    done: bool,
    text_open: bool,
    id: String,
    tools: BTreeMap<u64, Value>,
    usage: Value,
    reason: Value,
    model: String,
    size: usize,
    native: bool,
    thinking: Option<String>,
    content: String,
}
impl Decoder {
    fn frame(&mut self, data: &str) -> Result<String, StatusCode> {
        if self.done {
            return Err(StatusCode::BAD_GATEWAY);
        }
        if data == "[DONE]" {
            if !self.started || self.reason.is_null() {
                return Err(StatusCode::BAD_GATEWAY);
            }
            let calls = self.tools.values().cloned().collect::<Vec<_>>();
            // Validate the full tool arguments before reporting a successful stream end.
            let final_message = from_chat(
                &json!({"id":self.id,"choices":[{"message":{"role":"assistant","content":null,"tool_calls":calls},"finish_reason":self.reason}],"usage":self.usage}),
                &self.model,
            )?;
            let mut out = String::new();
            let mut index = 0;
            if self.text_open {
                out.push_str(&event(json!({"type":"content_block_stop","index":0})));
                index += 1;
            }
            for block in final_message["content"].as_array().ok_or(StatusCode::BAD_GATEWAY)? {
                out.push_str(&event(json!({"type":"content_block_start","index":index,"content_block":{"type":"tool_use","id":block["id"],"name":block["name"],"input":{}}})));
                out.push_str(&event(json!({"type":"content_block_delta","index":index,"delta":{"type":"input_json_delta","partial_json":block["input"].to_string()}})));
                out.push_str(&event(json!({"type":"content_block_stop","index":index})));
                index += 1;
            }
            out.push_str(&event(json!({"type":"message_delta","delta":{"stop_reason":finish(&self.reason)?,"stop_sequence":null},"usage":final_message["usage"]})));
            out.push_str(&event(json!({"type":"message_stop"})));
            self.done = true;
            return Ok(out);
        }
        let chunk: Value = serde_json::from_str(data).map_err(|_| StatusCode::BAD_GATEWAY)?;
        if chunk.get("error").is_some() {
            return Err(StatusCode::BAD_GATEWAY);
        }
        if self.native {
            let mut chunk = chunk;
            if chunk["type"] == "message_start" {
                chunk["message"]["model"] = self.model.clone().into();
                self.started = true;
            }
            if chunk["type"] == "message_stop" {
                self.done = true;
            }
            if chunk["type"] == "error" {
                return Err(StatusCode::BAD_GATEWAY);
            }
            return Ok(event(chunk));
        }
        let choices = chunk["choices"].as_array().ok_or(StatusCode::BAD_GATEWAY)?;
        if choices.len() > 1 {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let mut out = String::new();
        if !self.started {
            self.id = chunk["id"].as_str().ok_or(StatusCode::BAD_GATEWAY)?.into();
            out.push_str(&event(json!({"type":"message_start","message":{"id":self.id,"type":"message","role":"assistant","model":self.model,"content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":0,"output_tokens":0}}})));
            self.started = true;
        }
        if !chunk["usage"].is_null() {
            self.usage = chunk["usage"].clone();
        }
        for choice in choices {
            if choice["index"] != 0 {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            let delta = &choice["delta"];
            if delta.as_object().is_none_or(|m| {
                m.keys().any(|k| {
                    !["role", "content", "tool_calls", "refusal", "reasoning_content"]
                        .contains(&k.as_str())
                })
            }) || !delta["refusal"].is_null()
            {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            if let Some(thought) = &mut self.thinking {
                if let Some(part) = delta.get("reasoning_content").filter(|v| !v.is_null()) {
                    thought.push_str(part.as_str().ok_or(StatusCode::BAD_GATEWAY)?);
                }
            } else {
                super::messages::validate_chat_reasoning(delta)?;
            }
            if let Some(text) = delta["content"].as_str() {
                if self.thinking.is_some() {
                    self.content.push_str(text);
                }
                if !text.is_empty() {
                    if !self.text_open {
                        out.push_str(&event(json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}})));
                        self.text_open = true;
                    }
                    out.push_str(&event(json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":text}})));
                }
            } else if !delta["content"].is_null() {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            if let Some(calls) = delta.get("tool_calls").filter(|calls| !calls.is_null()) {
                for call in calls.as_array().ok_or(StatusCode::BAD_GATEWAY)? {
                    object_keys(call, &["index", "id", "type", "function"])?;
                    if let Some(function) = call.get("function") {
                        object_keys(function, &["name", "arguments"])?;
                    }
                    let index = call["index"].as_u64().ok_or(StatusCode::BAD_GATEWAY)?;
                    if index > 1024 {
                        return Err(StatusCode::BAD_GATEWAY);
                    }
                    let tool = self.tools.entry(index).or_insert_with(
                        || json!({"id":"","type":"function","function":{"name":"","arguments":""}}),
                    );
                    if call.get("type").is_some_and(|v| v != "function") {
                        return Err(StatusCode::UNPROCESSABLE_ENTITY);
                    }
                    for (source, target) in
                        [("id", "id"), ("name", "name"), ("arguments", "arguments")]
                    {
                        let value = if source == "id" {
                            call.get(source)
                        } else {
                            call["function"].get(source)
                        };
                        if let Some(value) = value {
                            let value = value.as_str().ok_or(StatusCode::BAD_GATEWAY)?;
                            let slot = if target == "id" {
                                &mut tool[target]
                            } else {
                                &mut tool["function"][target]
                            };
                            *slot = json!(format!("{}{value}", slot.as_str().unwrap_or_default()));
                        }
                    }
                }
            }
            if !choice["finish_reason"].is_null() {
                finish(&choice["finish_reason"])?;
                self.reason = choice["finish_reason"].clone();
            }
        }
        Ok(out)
    }
    fn push(&mut self, bytes: &[u8]) -> Result<String, StatusCode> {
        self.observer.push(bytes).map_err(|_| StatusCode::BAD_GATEWAY)?;
        self.size += bytes.len();
        if self.size > LIMIT {
            return Err(StatusCode::BAD_GATEWAY);
        }
        let mut out = String::new();
        for line in self.pending.push(bytes, LIMIT).map_err(|_| StatusCode::BAD_GATEWAY)? {
            if line.is_empty() {
                if !self.data.is_empty() {
                    let data = std::mem::take(&mut self.data);
                    out.push_str(&self.frame(data.trim_end_matches('\n'))?);
                }
            } else if let Some(data) = line.strip_prefix("data:") {
                self.data.push_str(data.strip_prefix(' ').unwrap_or(data));
                self.data.push('\n');
            }
        }
        Ok(out)
    }
}

/// Buffer a thinking round until the mandatory terminal marker and full tools
/// have been validated. Only then may Context issue a replay receipt.
pub(super) async fn collect_thinking(
    response: reqwest::Response,
    model: &str,
    cancel: CancellationToken,
) -> Result<Value, StatusCode> {
    if !response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("text/event-stream"))
    {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let mut decoder =
        Decoder { model: model.into(), thinking: Some(String::new()), ..Default::default() };
    let mut stream = response.bytes_stream();
    while !decoder.done {
        let next = tokio::select! { biased; () = cancel.cancelled() => return Err(StatusCode::REQUEST_TIMEOUT), next = stream.next() => next };
        let bytes = next.ok_or(StatusCode::BAD_GATEWAY)?.map_err(|_| StatusCode::BAD_GATEWAY)?;
        decoder.push(&bytes)?;
    }
    decoder.observer.finish().map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(json!({"id":decoder.id,"choices":[{"message":{
        "role":"assistant","content":decoder.content,"reasoning_content":decoder.thinking,
        "tool_calls":decoder.tools.into_values().collect::<Vec<_>>()},
        "finish_reason":decoder.reason}],"usage":decoder.usage}))
}

pub(super) async fn response(
    response: reqwest::Response,
    model: String,
    streaming: bool,
    native: bool,
    cancel: CancellationToken,
) -> Result<Response, StatusCode> {
    let headers = response.headers().clone();
    let mut adapted = Response::new(Body::from_stream(response.bytes_stream()));
    // The native relay has already validated the upstream status. The shared
    // adapter below only consumes bounded Messages/Chat entities.
    *adapted.headers_mut() = headers;
    response_body(adapted, model, streaming, native, cancel).await
}

pub(crate) async fn response_body(
    response: Response,
    model: String,
    streaming: bool,
    native: bool,
    cancel: CancellationToken,
) -> Result<Response, StatusCode> {
    if !streaming {
        let mut stream = response.into_body().into_data_stream();
        let mut bytes = Vec::new();
        loop {
            let next = tokio::select! { biased; () = cancel.cancelled() => return Err(StatusCode::REQUEST_TIMEOUT), next = stream.next() => next };
            let Some(chunk) = next else {
                break;
            };
            let chunk = chunk.map_err(|_| StatusCode::BAD_GATEWAY)?;
            if bytes.len() + chunk.len() > LIMIT {
                return Err(StatusCode::BAD_GATEWAY);
            }
            bytes.extend_from_slice(&chunk);
        }
        let body: Value = serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_GATEWAY)?;
        let converted = if native {
            let mut body = body;
            if body["type"] != "message" {
                return Err(StatusCode::BAD_GATEWAY);
            }
            body["model"] = model.clone().into();
            body
        } else {
            from_chat(&body, &model)?
        };
        return Response::builder()
            .header("content-type", "application/json")
            .body(Body::from(converted.to_string()))
            .map_err(|_| StatusCode::BAD_GATEWAY);
    }
    if !response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("text/event-stream"))
    {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let stream = futures_util::stream::unfold(
        (
            response.into_body().into_data_stream(),
            Decoder {
                model,
                native,
                observer: super::stream_observer::StreamObserver::new(if native {
                    super::generation::GenerationProtocol::Messages
                } else {
                    super::generation::GenerationProtocol::Chat
                }),
                ..Default::default()
            },
            cancel,
        ),
        |(mut stream, mut decoder, cancel)| async move {
            if decoder.done {
                return None;
            }
            loop {
                let next = tokio::select! { biased; () = cancel.cancelled() => return None, next = stream.next() => next };
                let result = match next {
                    Some(Ok(bytes)) => decoder.push(&bytes),
                    _ => Err(StatusCode::BAD_GATEWAY),
                };
                match result {
                    Ok(out) if out.is_empty() => continue,
                    Ok(out) => {
                        return Some((
                            Ok::<_, std::io::Error>(Bytes::from(out)),
                            (stream, decoder, cancel),
                        ));
                    }
                    Err(_) => {
                        decoder.done = true;
                        let out = event(
                            json!({"type":"error","error":{"type":"api_error","message":"Unsupported or invalid upstream Messages conversion"}}),
                        );
                        return Some((Ok(Bytes::from(out)), (stream, decoder, cancel)));
                    }
                }
            }
        },
    );
    Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from_stream(stream))
        .map_err(|_| StatusCode::BAD_GATEWAY)
}

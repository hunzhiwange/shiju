//! Request-local bounded SSE conversion. Function arguments are emitted only after validation.
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
fn event(v: Value) -> String {
    format!("data: {v}\n\n")
}
#[derive(Default)]
struct Decoder {
    observer: super::stream_observer::StreamObserver,
    pending: Vec<u8>,
    data: String,
    size: usize,
    done: bool,
    native: bool,
    model: String,
    tools: BTreeMap<u64, Value>,
    reason: Value,
    usage: Value,
    finished: bool,
}
impl Decoder {
    fn frame(&mut self, data: &str) -> Result<String, StatusCode> {
        if self.done {
            return Err(StatusCode::BAD_GATEWAY);
        }
        if self.native {
            let chunk: Value = serde_json::from_str(data).map_err(|_| StatusCode::BAD_GATEWAY)?;
            let chunk = super::gemini::output(chunk, &self.model)?;
            if chunk["candidates"]
                .as_array()
                .is_some_and(|c| c.iter().any(|c| c.get("finishReason").is_some()))
                || chunk["promptFeedback"].get("blockReason").is_some()
            {
                self.finished = true;
            }
            return Ok(event(chunk));
        }
        if data == "[DONE]" {
            let output = super::gemini::from_chat(
                &json!({"choices":[{"message":{"role":"assistant","content":null,"tool_calls":self.tools.values().cloned().collect::<Vec<_>>()},"finish_reason":self.reason}],"usage":self.usage}),
                &self.model,
            )?;
            self.done = true;
            return Ok(event(output));
        }
        let chunk: Value = serde_json::from_str(data).map_err(|_| StatusCode::BAD_GATEWAY)?;
        if chunk.get("error").is_some() {
            return Err(StatusCode::BAD_GATEWAY);
        }
        let choices = chunk["choices"].as_array().ok_or(StatusCode::BAD_GATEWAY)?;
        if choices.len() > 1 {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        if let Some(usage) = chunk.get("usage").filter(|v| !v.is_null()) {
            self.usage = usage.clone();
        }
        let mut output = String::new();
        for c in choices {
            if c["index"] != 0 || self.finished {
                return Err(StatusCode::BAD_GATEWAY);
            }
            let d = &c["delta"];
            super::messages::object_keys(
                d,
                &["role", "content", "tool_calls", "refusal", "reasoning_content"],
            )?;
            super::messages::validate_chat_reasoning(d)?;
            if !d["refusal"].is_null() {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            if let Some(text) = d["content"].as_str() {
                if !text.is_empty() {
                    output.push_str(&event(json!({"candidates":[{"index":0,"content":{"role":"model","parts":[{"text":text}]}}],"modelVersion":self.model})));
                }
            } else if !d["content"].is_null() {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            if let Some(calls) = d.get("tool_calls") {
                for call in calls.as_array().ok_or(StatusCode::BAD_GATEWAY)? {
                    super::messages::object_keys(call, &["index", "id", "type", "function"])?;
                    if let Some(f) = call.get("function") {
                        super::messages::object_keys(f, &["name", "arguments"])?;
                    }
                    let index = call["index"]
                        .as_u64()
                        .filter(|n| *n < 1024)
                        .ok_or(StatusCode::BAD_GATEWAY)?;
                    if call.get("type").is_some_and(|v| v != "function") {
                        return Err(StatusCode::UNPROCESSABLE_ENTITY);
                    }
                    let tool = self.tools.entry(index).or_insert_with(
                        || json!({"id":"","type":"function","function":{"name":"","arguments":""}}),
                    );
                    for key in ["id", "name", "arguments"] {
                        let value =
                            if key == "id" { call.get(key) } else { call["function"].get(key) };
                        if let Some(value) = value {
                            let text = value.as_str().ok_or(StatusCode::BAD_GATEWAY)?;
                            let slot = if key == "id" {
                                &mut tool[key]
                            } else {
                                &mut tool["function"][key]
                            };
                            *slot = json!(format!("{}{text}", slot.as_str().unwrap_or_default()));
                        }
                    }
                }
            }
            if !c["finish_reason"].is_null() {
                self.reason = c["finish_reason"].clone();
                self.finished = true;
            }
        }
        Ok(output)
    }
    fn push(&mut self, bytes: &[u8]) -> Result<String, StatusCode> {
        self.observer.push(bytes).map_err(|_| StatusCode::BAD_GATEWAY)?;
        self.size += bytes.len();
        if self.size > LIMIT {
            return Err(StatusCode::BAD_GATEWAY);
        }
        self.pending.extend_from_slice(bytes);
        let mut output = String::new();
        while let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
            let line = self.pending.drain(..=end).collect::<Vec<_>>();
            let line = std::str::from_utf8(&line)
                .map_err(|_| StatusCode::BAD_GATEWAY)?
                .trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                if !self.data.is_empty() {
                    let data = std::mem::take(&mut self.data);
                    output.push_str(&self.frame(data.trim_end_matches('\n'))?);
                }
            } else if let Some(data) = line.strip_prefix("data:") {
                self.data.push_str(data.strip_prefix(' ').unwrap_or(data));
                self.data.push('\n');
            }
        }
        Ok(output)
    }
}
pub(super) async fn response(
    response: reqwest::Response,
    model: String,
    streaming: bool,
    native: bool,
    cancel: CancellationToken,
) -> Result<Response, StatusCode> {
    if !streaming {
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::new();
        loop {
            let next = tokio::select! {biased;()=cancel.cancelled()=>return Err(StatusCode::REQUEST_TIMEOUT),next=stream.next()=>next};
            let Some(chunk) = next else { break };
            let chunk = chunk.map_err(|_| StatusCode::BAD_GATEWAY)?;
            if bytes.len() + chunk.len() > LIMIT {
                return Err(StatusCode::BAD_GATEWAY);
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_GATEWAY)?;
        let value = if native {
            super::gemini::output(value, &model)?
        } else {
            super::gemini::from_chat(&value, &model)?
        };
        return Response::builder()
            .header("content-type", "application/json")
            .body(Body::from(value.to_string()))
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
            response.bytes_stream(),
            Decoder {
                model,
                native,
                observer: super::stream_observer::StreamObserver::new(if native {
                    super::generation::GenerationProtocol::Gemini
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
                let next = tokio::select! {biased;()=cancel.cancelled()=>return None,next=stream.next()=>next};
                let result = match next {
                    Some(Ok(bytes)) => decoder.push(&bytes),
                    None if decoder.native
                        && decoder.finished
                        && decoder.pending.is_empty()
                        && decoder.data.is_empty()
                        && decoder.observer.finish().is_ok() =>
                    {
                        return None;
                    }
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
                            json!({"error":{"code":502,"status":"UNAVAILABLE","message":"Invalid or unsupported Gemini upstream stream"}}),
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

//! Native SSE to Chat deltas; no successful terminal is emitted before validated EOF.
use axum::{
    body::Bytes,
    http::{StatusCode, header},
};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;
use vw_shared::provider::ModelProtocol as P;
const LIMIT: usize = 4 * 1024 * 1024;
const BAD: StatusCode = StatusCode::BAD_GATEWAY;

pub(super) struct Decoder {
    target: P,
    model: String,
    observer: super::stream_observer::StreamObserver,
    pending: super::sse_framing::Lines,
    data: String,
    size: usize,
    native: Value,
    tool_json: BTreeMap<u64, String>,
    text: String,
    started: bool,
}
impl Decoder {
    pub(super) fn new(target: P, model: String) -> Self {
        Self {
            target,
            model,
            observer: super::stream_observer::StreamObserver::new(target),
            pending: super::sse_framing::Lines::default(),
            data: String::new(),
            size: 0,
            native: Value::Null,
            tool_json: BTreeMap::new(),
            text: String::new(),
            started: false,
        }
    }
    pub(super) fn push(&mut self, bytes: &[u8]) -> Result<String, StatusCode> {
        self.observer.push(bytes).map_err(|_| BAD)?;
        self.size = self.size.checked_add(bytes.len()).filter(|n| *n <= LIMIT).ok_or(BAD)?;
        let mut out = String::new();
        for line in self.pending.push(bytes, LIMIT).map_err(|_| BAD)? {
            if line.is_empty() {
                if !self.data.is_empty() {
                    let data = std::mem::take(&mut self.data);
                    let v: Value =
                        serde_json::from_str(data.trim_end_matches('\n')).map_err(|_| BAD)?;
                    out.push_str(&self.frame(v)?);
                }
            } else if let Some(data) = line.strip_prefix("data:") {
                self.data.push_str(data.strip_prefix(' ').unwrap_or(data));
                self.data.push('\n');
            }
        }
        Ok(out)
    }
    fn chunk(&mut self, delta: Value, reason: Value, usage: Value) -> String {
        let mut out = String::new();
        if !self.started {
            self.started = true;
            out.push_str(&format!("data: {}\n\n",json!({"id":"relay-native","object":"chat.completion.chunk","model":self.model,"choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]})));
        }
        out.push_str(&format!("data: {}\n\n",json!({"id":"relay-native","object":"chat.completion.chunk","model":self.model,"choices":[{"index":0,"delta":delta,"finish_reason":reason}],"usage":usage})));
        out
    }
    fn frame(&mut self, v: Value) -> Result<String, StatusCode> {
        let mut text = String::new();
        match self.target {
            P::Messages => match v["type"].as_str() {
                Some("message_start") => {
                    self.native = v["message"].clone();
                    self.native["content"] = json!([]);
                }
                Some("content_block_start") => {
                    let index = v["index"].as_u64().ok_or(BAD)?;
                    let block = &v["content_block"];
                    let parts = self.native["content"].as_array_mut().ok_or(BAD)?;
                    if index as usize != parts.len()
                        || !matches!(block["type"].as_str(), Some("text" | "tool_use"))
                    {
                        return Err(BAD);
                    }
                    if block["type"] == "text" {
                        text.push_str(block["text"].as_str().ok_or(BAD)?);
                    }
                    parts.push(block.clone());
                }
                Some("content_block_delta") => {
                    let index = v["index"].as_u64().ok_or(BAD)?;
                    let part = self.native["content"]
                        .as_array_mut()
                        .and_then(|a| a.get_mut(index as usize))
                        .ok_or(BAD)?;
                    match v["delta"]["type"].as_str() {
                        Some("text_delta") if part["type"] == "text" => {
                            let t = v["delta"]["text"].as_str().ok_or(BAD)?;
                            text.push_str(t);
                            part["text"] =
                                format!("{}{t}", part["text"].as_str().ok_or(BAD)?).into();
                        }
                        Some("input_json_delta") if part["type"] == "tool_use" => self
                            .tool_json
                            .entry(index)
                            .or_default()
                            .push_str(v["delta"]["partial_json"].as_str().ok_or(BAD)?),
                        _ => return Err(BAD),
                    }
                }
                Some("content_block_stop") => {
                    let index = v["index"].as_u64().ok_or(BAD)?;
                    if let Some(args) = self.tool_json.remove(&index) {
                        self.native["content"][index as usize]["input"] =
                            serde_json::from_str::<Value>(&args).map_err(|_| BAD)?;
                    }
                }
                Some("message_delta") => {
                    self.native["stop_reason"] = v["delta"]["stop_reason"].clone();
                    for (key, n) in v["usage"].as_object().ok_or(BAD)? {
                        self.native["usage"][key] = n.clone();
                    }
                }
                Some("message_stop" | "ping") => {}
                _ => return Err(BAD),
            },
            P::Responses => match v["type"].as_str() {
                Some("response.output_text.delta") => {
                    text.push_str(v["delta"].as_str().ok_or(BAD)?)
                }
                Some("response.completed" | "response.incomplete") => {
                    self.native = v["response"].clone()
                }
                Some(
                    "response.created"
                    | "response.in_progress"
                    | "response.output_item.added"
                    | "response.output_item.done"
                    | "response.content_part.added"
                    | "response.content_part.done"
                    | "response.output_text.done"
                    | "response.function_call_arguments.delta"
                    | "response.function_call_arguments.done",
                ) => {}
                // Reasoning/hosted tools and opaque events cannot be silently dropped.
                _ => return Err(BAD),
            },
            P::Gemini => {
                if self.native.is_null() {
                    self.native = json!({"candidates":[{"index":0,"content":{"role":"model","parts":[]}}],"modelVersion":v["modelVersion"]});
                }
                for field in ["usageMetadata", "responseId", "modelVersion"] {
                    if let Some(value) = v.get(field) {
                        self.native[field] = value.clone();
                    }
                }
                if let Some(candidates) = v.get("candidates") {
                    let candidates = candidates.as_array().filter(|a| a.len() <= 1).ok_or(BAD)?;
                    for c in candidates {
                        if let Some(parts) = c["content"].get("parts") {
                            for part in parts.as_array().ok_or(BAD)? {
                                if part["thought"] == true
                                    || part.get("thoughtSignature").is_some_and(|v| !v.is_null())
                                {
                                    return Err(BAD);
                                }
                                if let Some(t) = part.get("text") {
                                    text.push_str(t.as_str().ok_or(BAD)?);
                                }
                                self.native["candidates"][0]["content"]["parts"]
                                    .as_array_mut()
                                    .ok_or(BAD)?
                                    .push(part.clone());
                            }
                        }
                        if let Some(reason) = c.get("finishReason") {
                            self.native["candidates"][0]["finishReason"] = reason.clone();
                        }
                    }
                }
            }
            P::Chat => return Err(BAD),
        }
        if text.is_empty() {
            return Ok(String::new());
        }
        self.text.push_str(&text);
        Ok(self.chunk(json!({"content":text}), Value::Null, Value::Null))
    }
    pub(super) fn finish(&mut self) -> Result<String, StatusCode> {
        self.observer.finish().map_err(|_| BAD)?;
        if !self.pending.is_empty() || !self.data.is_empty() || !self.tool_json.is_empty() {
            return Err(BAD);
        }
        let chat = super::protocol_bridge::response(self.target, &self.native)?;
        let message = &chat["choices"][0]["message"];
        if message["content"].as_str() != Some(self.text.as_str()) {
            return Err(BAD);
        }
        let mut delta = json!({});
        if let Some(calls) = message.get("tool_calls") {
            let mut calls = calls.as_array().ok_or(BAD)?.clone();
            for (index, call) in calls.iter_mut().enumerate() {
                call["index"] = index.into();
            }
            delta["tool_calls"] = calls.into();
        }
        let mut out =
            self.chunk(delta, chat["choices"][0]["finish_reason"].clone(), chat["usage"].clone());
        out.push_str("data: [DONE]\n\n");
        Ok(out)
    }
}

/// Preserve HTTP errors at the transport boundary; this adapter sees successful bodies only.
pub(super) async fn response(
    response: reqwest::Response,
    target: P,
    model: String,
    streaming: bool,
    cancel: CancellationToken,
) -> Result<reqwest::Response, StatusCode> {
    if !streaming {
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::new();
        loop {
            let next = tokio::select! {biased;()=cancel.cancelled()=>return Err(StatusCode::REQUEST_TIMEOUT),next=stream.next()=>next};
            let Some(chunk) = next else { break };
            let chunk = chunk.map_err(|_| BAD)?;
            if bytes.len() + chunk.len() > LIMIT {
                return Err(BAD);
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| BAD)?;
        let chat = super::protocol_bridge::response(target, &value)?;
        let result = axum::http::Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(chat.to_string())
            .map_err(|_| BAD)?;
        return Ok(reqwest::Response::from(result));
    }
    if !response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("text/event-stream"))
    {
        return Err(BAD);
    }
    let stream = futures_util::stream::unfold(
        (response.bytes_stream(), Decoder::new(target, model), cancel, false),
        |(mut stream, mut decoder, cancel, done)| async move {
            if done {
                return None;
            }
            loop {
                let next = tokio::select! {biased;()=cancel.cancelled()=>return Some((Err(std::io::Error::other("relay cancelled")),(stream,decoder,cancel,true))),next=stream.next()=>next};
                let (out, done) = match next {
                    Some(Ok(bytes)) => match decoder.push(&bytes) {
                        Ok(out) if out.is_empty() => continue,
                        Ok(out) => (Ok(Bytes::from(out)), false),
                        Err(_) => (Err(std::io::Error::other("invalid native stream")), true),
                    },
                    Some(Err(_)) => (Err(std::io::Error::other("upstream stream failed")), true),
                    None => match decoder.finish() {
                        Ok(out) => (Ok(Bytes::from(out)), true),
                        Err(_) => (Err(std::io::Error::other("incomplete native stream")), true),
                    },
                };
                return Some((out, (stream, decoder, cancel, done)));
            }
        },
    );
    let result = axum::http::Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(reqwest::Body::wrap_stream(stream))
        .map_err(|_| BAD)?;
    Ok(reqwest::Response::from(result))
}
#[cfg(test)]
#[path = "protocol_bridge_stream_tests.rs"]
mod tests;

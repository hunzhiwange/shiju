//! Request-local, size-bounded SSE conversion; cancellation drops the upstream stream.
use super::{
    RelayIdentity,
    responses::{self, ChatTools, References},
};
use axum::{
    body::{Body, Bytes},
    http::StatusCode,
    response::Response,
};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
use tokio_util::sync::CancellationToken;
const LIMIT: usize = 4 * 1024 * 1024;
fn event(value: Value) -> String {
    format!("event: {}\ndata: {value}\n\n", value["type"].as_str().unwrap_or("error"))
}
struct Decoder {
    pending: Vec<u8>,
    data: String,
    size: usize,
    done: bool,
    started: bool,
    sequence: u64,
    id: String,
    identity: RelayIdentity,
    native: bool,
    codex_chat: bool,
    tool_names: ChatTools,
    reasoning: String,
    upstream_id: Option<String>,
    store: bool,
    references: Arc<References>,
    text: String,
    text_open: bool,
    tools: BTreeMap<u64, Value>,
    usage: Value,
    reason: Value,
}
impl Decoder {
    fn emit(&mut self, mut value: Value) -> String {
        value["sequence_number"] = self.sequence.into();
        self.sequence += 1;
        event(value)
    }
    fn initial(&self) -> Value {
        json!({"id":self.id,"object":"response","created_at":chrono::Utc::now().timestamp(),"model":self.identity.model,"status":"in_progress","output":[],"usage":null,"error":null,"incomplete_details":null})
    }
    fn failure(&mut self) -> String {
        self.done = true;
        let mut response = self.initial();
        response["status"] = "failed".into();
        response["error"] = json!({"code":"relay_error","message":"Unsupported or invalid upstream Responses conversion"});
        self.emit(json!({"type":"response.failed","response":response}))
    }
    fn finish(&mut self) -> Result<String, StatusCode> {
        if !self.started || self.reason.is_null() {
            return Err(StatusCode::BAD_GATEWAY);
        }
        let calls = self.tools.values().cloned().collect::<Vec<_>>();
        let mut body = json!({"id":self.id,"choices":[{"message":{"role":"assistant","content":if self.text_open{json!(self.text)}else{Value::Null},"tool_calls":calls},"finish_reason":self.reason}],"usage":self.usage});
        if !self.reasoning.is_empty() {
            body["choices"][0]["message"]["reasoning_content"] = self.reasoning.as_str().into();
        }
        let response = responses::from_chat(
            &body,
            &self.identity.model,
            &self.id,
            &self.tool_names,
            self.codex_chat,
        )?;
        let mut out = String::new();
        for (index, item) in
            response["output"].as_array().ok_or(StatusCode::BAD_GATEWAY)?.iter().enumerate()
        {
            let item_id = &item["id"];
            if item["type"] == "message" {
                out.push_str(&self.emit(json!({"type":"response.output_text.done","item_id":item_id,"output_index":index,"content_index":0,"text":self.text,"logprobs":[]})));
                out.push_str(&self.emit(json!({"type":"response.content_part.done","item_id":item_id,"output_index":index,"content_index":0,"part":item["content"][0]})));
            } else if item["type"] == "reasoning" {
                let mut start = item.clone();
                start["summary"] = json!([]);
                out.push_str(&self.emit(
                    json!({"type":"response.output_item.added","output_index":index,"item":start}),
                ));
            } else {
                let mut start = item.clone();
                let (field, delta_event, done_event) = if item["type"] == "custom_tool_call" {
                    (
                        "input",
                        "response.custom_tool_call_input.delta",
                        "response.custom_tool_call_input.done",
                    )
                } else {
                    (
                        "arguments",
                        "response.function_call_arguments.delta",
                        "response.function_call_arguments.done",
                    )
                };
                start[field] = "".into();
                start["status"] = "in_progress".into();
                out.push_str(&self.emit(
                    json!({"type":"response.output_item.added","output_index":index,"item":start}),
                ));
                out.push_str(&self.emit(json!({"type":delta_event,"item_id":item_id,"output_index":index,"delta":item[field]})));
                out.push_str(&self.emit(json!({"type":done_event,"item_id":item_id,"output_index":index,(field):item[field]})));
            }
            out.push_str(&self.emit(
                json!({"type":"response.output_item.done","output_index":index,"item":item}),
            ));
        }
        let kind = if response["status"] == "incomplete" {
            "response.incomplete"
        } else {
            "response.completed"
        };
        out.push_str(&self.emit(json!({"type":kind,"response":response})));
        self.done = true;
        Ok(out)
    }
    fn frame(&mut self, data: &str) -> Result<String, StatusCode> {
        if self.done {
            return Err(StatusCode::BAD_GATEWAY);
        }
        if data == "[DONE]" {
            if self.native {
                return Err(StatusCode::BAD_GATEWAY);
            }
            return self.finish();
        }
        let mut chunk: Value = serde_json::from_str(data).map_err(|_| StatusCode::BAD_GATEWAY)?;
        if chunk.get("error").is_some()
            || matches!(chunk["type"].as_str(), Some("error" | "response.failed"))
        {
            return Err(StatusCode::BAD_GATEWAY);
        }
        if self.native {
            let kind = chunk["type"].as_str().ok_or(StatusCode::BAD_GATEWAY)?.to_owned();
            if !kind.starts_with("response.")
                || !self.started && kind != "response.created"
                || self.started && kind == "response.created"
            {
                return Err(StatusCode::BAD_GATEWAY);
            }
            if chunk.get("response").is_some() {
                let upstream_id =
                    chunk["response"]["id"].as_str().ok_or(StatusCode::BAD_GATEWAY)?;
                if self.upstream_id.as_deref().is_some_and(|id| id != upstream_id) {
                    return Err(StatusCode::BAD_GATEWAY);
                }
                self.upstream_id = Some(upstream_id.into());
                let terminal =
                    matches!(kind.as_str(), "response.completed" | "response.incomplete");
                if terminal
                    && chunk["response"]["status"]
                        != if kind == "response.completed" { "completed" } else { "incomplete" }
                {
                    return Err(StatusCode::BAD_GATEWAY);
                }
                chunk["response"] = responses::native_output(
                    chunk["response"].take(),
                    &self.identity,
                    &self.id,
                    &self.references,
                    terminal && self.store,
                )?;
                if terminal {
                    self.done = true;
                }
            }
            if let Some(response_id) = chunk.get("response_id") {
                if response_id.as_str() != self.upstream_id.as_deref() {
                    return Err(StatusCode::BAD_GATEWAY);
                }
                chunk["response_id"] = self.id.clone().into();
            }
            self.started = true;
            // Native event order, optional fields and sequence numbers remain untouched.
            return Ok(event(chunk));
        }
        let choices = chunk["choices"].as_array().ok_or(StatusCode::BAD_GATEWAY)?;
        if choices.len() > 1 {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let mut out = String::new();
        if !self.started {
            out.push_str(&self.emit(json!({"type":"response.created","response":self.initial()})));
            out.push_str(
                &self.emit(json!({"type":"response.in_progress","response":self.initial()})),
            );
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
            if self.codex_chat {
                super::messages::object_keys(
                    delta,
                    &["role", "content", "tool_calls", "refusal", "reasoning_content"],
                )?;
                if let Some(reasoning) =
                    delta.get("reasoning_content").filter(|value| !value.is_null())
                {
                    self.reasoning.push_str(reasoning.as_str().ok_or(StatusCode::BAD_GATEWAY)?);
                }
            } else {
                super::messages::object_keys(delta, &["role", "content", "tool_calls", "refusal"])?;
            }
            if !delta["refusal"].is_null() || delta.get("role").is_some_and(|v| v != "assistant") {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            if let Some(text) = delta["content"].as_str() {
                if !text.is_empty() {
                    let item_id = format!("{}_msg_0", self.id);
                    if !self.text_open {
                        out.push_str(&self.emit(json!({"type":"response.output_item.added","output_index":0,"item":{"id":item_id,"type":"message","role":"assistant","status":"in_progress","content":[]}})));
                        out.push_str(&self.emit(json!({"type":"response.content_part.added","item_id":item_id,"output_index":0,"content_index":0,"part":{"type":"output_text","text":"","annotations":[],"logprobs":[]}})));
                        self.text_open = true;
                    }
                    self.text.push_str(text);
                    out.push_str(&self.emit(json!({"type":"response.output_text.delta","item_id":item_id,"output_index":0,"content_index":0,"delta":text,"logprobs":[]})));
                }
            } else if !delta["content"].is_null() {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            if let Some(calls) = delta.get("tool_calls") {
                for call in calls.as_array().ok_or(StatusCode::BAD_GATEWAY)? {
                    super::messages::object_keys(call, &["index", "id", "type", "function"])?;
                    if call.get("type").is_some_and(|v| v != "function") {
                        return Err(StatusCode::UNPROCESSABLE_ENTITY);
                    }
                    if let Some(function) = call.get("function") {
                        super::messages::object_keys(function, &["name", "arguments"])?;
                    }
                    let index = call["index"]
                        .as_u64()
                        .filter(|n| *n < 1024)
                        .ok_or(StatusCode::BAD_GATEWAY)?;
                    let tool = self.tools.entry(index).or_insert_with(
                        || json!({"id":"","type":"function","function":{"name":"","arguments":""}}),
                    );
                    for key in ["id", "name", "arguments"] {
                        let fragment =
                            if key == "id" { call.get(key) } else { call["function"].get(key) };
                        if let Some(fragment) = fragment {
                            let fragment = fragment.as_str().ok_or(StatusCode::BAD_GATEWAY)?;
                            let target = if key == "id" {
                                &mut tool[key]
                            } else {
                                &mut tool["function"][key]
                            };
                            *target =
                                json!(format!("{}{fragment}", target.as_str().unwrap_or_default()));
                        }
                    }
                }
            }
            if !choice["finish_reason"].is_null() {
                super::messages::finish(&choice["finish_reason"])?;
                self.reason = choice["finish_reason"].clone();
            }
        }
        Ok(out)
    }
    fn push(&mut self, bytes: &[u8]) -> Result<String, StatusCode> {
        self.size += bytes.len();
        if self.size > LIMIT {
            return Err(StatusCode::BAD_GATEWAY);
        }
        self.pending.extend_from_slice(bytes);
        let mut out = String::new();
        while let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
            let line = self.pending.drain(..=end).collect::<Vec<_>>();
            let line = std::str::from_utf8(&line)
                .map_err(|_| StatusCode::BAD_GATEWAY)?
                .trim_end_matches(['\r', '\n']);
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
pub(super) struct Output {
    pub identity: RelayIdentity,
    pub native: bool,
    pub codex_chat: bool,
    pub tool_names: ChatTools,
    pub streaming: bool,
    pub store: bool,
    pub references: Arc<References>,
}
pub(super) async fn response(
    response: reqwest::Response,
    output: Output,
    cancel: CancellationToken,
) -> Result<Response, StatusCode> {
    let id = responses::response_id();
    if !output.streaming {
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::new();
        loop {
            let next = tokio::select! {biased;()=cancel.cancelled()=>return Err(StatusCode::REQUEST_TIMEOUT),next=stream.next()=>next};
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
        let body = if output.native {
            responses::native_output(body, &output.identity, &id, &output.references, output.store)?
        } else {
            responses::from_chat(
                &body,
                &output.identity.model,
                &id,
                &output.tool_names,
                output.codex_chat,
            )?
        };
        return Response::builder()
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
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
    let decoder = Decoder {
        pending: Vec::new(),
        data: String::new(),
        size: 0,
        done: false,
        started: false,
        sequence: 0,
        id,
        identity: output.identity,
        native: output.native,
        codex_chat: output.codex_chat,
        tool_names: output.tool_names,
        reasoning: String::new(),
        upstream_id: None,
        store: output.store,
        references: output.references,
        text: String::new(),
        text_open: false,
        tools: BTreeMap::new(),
        usage: Value::Null,
        reason: Value::Null,
    };
    let stream = futures_util::stream::unfold(
        (response.bytes_stream(), decoder, cancel),
        |(mut stream, mut decoder, cancel)| async move {
            if decoder.done {
                return None;
            }
            loop {
                let next = tokio::select! {biased;()=cancel.cancelled()=>return None,next=stream.next()=>next};
                let result = match next {
                    Some(Ok(bytes)) => decoder.push(&bytes),
                    _ => Err(StatusCode::BAD_GATEWAY),
                };
                let out = match result {
                    Ok(out) if out.is_empty() => continue,
                    Ok(out) => out,
                    Err(_) => decoder.failure(),
                };
                return Some((
                    Ok::<_, std::io::Error>(Bytes::from(out)),
                    (stream, decoder, cancel),
                ));
            }
        },
    );
    Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from_stream(stream))
        .map_err(|_| StatusCode::BAD_GATEWAY)
}

//! Request-local, size-bounded SSE conversion; cancellation drops the upstream stream.
use super::{
    RelayIdentity,
    responses::{self, ChatTools},
};
use crate::app_routing::reference_scope::References;
use axum::{
    body::{Body, Bytes},
    http::StatusCode,
    response::Response,
};
use futures_util::{Stream, StreamExt};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
use tokio_util::sync::CancellationToken;
const LIMIT: usize = 4 * 1024 * 1024;
fn event(value: Value) -> String {
    format!("event: {}\ndata: {value}\n\n", value["type"].as_str().unwrap_or("error"))
}
struct Decoder {
    signed_blocks: Option<Arc<std::sync::Mutex<Vec<Value>>>>,
    resident: Option<super::responses_chat_stream::State>,
    observer: super::stream_observer::StreamObserver,
    pending: super::sse_framing::Lines,
    data: String,
    retained: usize,
    done: bool,
    started: bool,
    sequence: u64,
    id: String,
    model: String,
    native: Option<Native>,
    codex_chat: bool,
    compact: bool,
    tool_names: ChatTools,
    reasoning: String,
    late_reasoning: String,
    upstream_id: Option<String>,
    text: String,
    text_open: bool,
    tools: BTreeMap<u64, Value>,
    usage: Value,
    reason: Value,
}

struct Native {
    identity: RelayIdentity,
    store: bool,
    references: Arc<References>,
}

impl Decoder {
    fn new(model: String, tool_names: ChatTools, codex_chat: bool, native: Option<Native>) -> Self {
        let protocol = if native.is_some() {
            super::generation::GenerationProtocol::Responses
        } else {
            super::generation::GenerationProtocol::Chat
        };
        Self {
            resident: None,
            observer: super::stream_observer::StreamObserver::new(protocol),
            pending: super::sse_framing::Lines::default(),
            data: String::new(),
            retained: 0,
            done: false,
            started: false,
            sequence: 0,
            id: responses::response_id(),
            model,
            native,
            codex_chat,
            compact: false,
            tool_names,
            signed_blocks: None,
            reasoning: String::new(),
            late_reasoning: String::new(),
            upstream_id: None,
            text: String::new(),
            text_open: false,
            tools: BTreeMap::new(),
            usage: Value::Null,
            reason: Value::Null,
        }
    }

    fn emit(&mut self, mut value: Value) -> String {
        if let Some(state) = &mut self.resident {
            state.decorate(&mut value);
        }
        value["sequence_number"] = self.sequence.into();
        self.sequence += 1;
        event(value)
    }
    fn initial(&self) -> Value {
        json!({"id":self.id,"object":"response","created_at":chrono::Utc::now().timestamp(),"model":self.model,"status":"in_progress","output":[],"usage":null,"error":null,"incomplete_details":null})
    }
    fn failure(&mut self, message: &str) -> String {
        self.done = true;
        let mut response = self.initial();
        response["status"] = "failed".into();
        response["error"] = json!({"code":"relay_error","message":message});
        self.emit(json!({"type":"response.failed","response":response}))
    }
    fn emit_reasoning_delta(
        &mut self,
        out: &mut String,
        item_id: &str,
        index: usize,
        delta: &str,
        first: bool,
    ) {
        if first {
            out.push_str(&self.emit(json!({"type":"response.output_item.added","output_index":index,"item":{"type":"reasoning","id":item_id,"summary":[]}})));
            out.push_str(&self.emit(json!({"type":"response.reasoning_summary_part.added","item_id":item_id,"output_index":index,"summary_index":0,"part":{"type":"summary_text","text":""}})));
        }
        out.push_str(&self.emit(json!({"type":"response.reasoning_summary_text.delta","item_id":item_id,"output_index":index,"summary_index":0,"delta":delta})));
    }
    fn finish_reasoning(&mut self, out: &mut String, index: usize, item: &Value) {
        let item_id = &item["id"];
        let part = &item["summary"][0];
        out.push_str(&self.emit(json!({"type":"response.reasoning_summary_text.done","item_id":item_id,"output_index":index,"summary_index":0,"text":part["text"]})));
        out.push_str(&self.emit(json!({"type":"response.reasoning_summary_part.done","item_id":item_id,"output_index":index,"summary_index":0,"part":part})));
        out.push_str(
            &self
                .emit(json!({"type":"response.output_item.done","output_index":index,"item":item})),
        );
    }
    fn finish(&mut self) -> Result<String, StatusCode> {
        if !self.started || self.reason.is_null() {
            return Err(StatusCode::BAD_GATEWAY);
        }
        if self.compact {
            return self.finish_compaction();
        }
        let calls = self.tools.values().cloned().collect::<Vec<_>>();
        let body = json!({"id":self.id,"choices":[{"message":{"role":"assistant","content":if self.text_open{json!(self.text)}else{Value::Null},"tool_calls":calls},"finish_reason":self.reason}],"usage":self.usage});
        let mut response = responses::from_chat(
            &body,
            &self.model,
            &self.id,
            &self.tool_names,
            self.codex_chat && self.resident.is_none(),
        )?;
        let items = response["output"].as_array_mut().ok_or(StatusCode::BAD_GATEWAY)?;
        let reasoning_id = format!("{}_rs", self.id);
        if !self.reasoning.is_empty() {
            items.insert(0, responses::reasoning_item(&reasoning_id, &self.reasoning));
        }
        let late_id = format!("{}_rs_late", self.id);
        let late_reasoning = std::mem::take(&mut self.late_reasoning);
        if !late_reasoning.is_empty() {
            let index = usize::from(!self.reasoning.is_empty()) + usize::from(self.text_open);
            items.insert(index, responses::reasoning_item(&late_id, &late_reasoning));
        }
        let signed = self
            .signed_blocks
            .as_ref()
            .map(|blocks| {
                blocks.lock().map(|blocks| blocks.clone()).map_err(|_| StatusCode::BAD_GATEWAY)
            })
            .transpose()?
            .unwrap_or_default();
        for block in signed {
            if let Some(item) =
                super::reasoning_bridge::responses_reasoning_item_from_anthropic_block(
                    &format!("{}_signed_{}", self.id, items.len()),
                    &block,
                )
            {
                items.push(item);
            }
        }
        let mut out = String::new();
        for item in items.iter_mut() {
            if let Some(state) = &self.resident {
                state.final_item(item);
            }
        }
        for (index, item) in items.iter().enumerate() {
            let item_id = &item["id"];
            if item["type"] == "message" {
                out.push_str(&self.emit(json!({"type":"response.output_text.done","item_id":item_id,"output_index":index,"content_index":0,"text":self.text,"logprobs":[]})));
                out.push_str(&self.emit(json!({"type":"response.content_part.done","item_id":item_id,"output_index":index,"content_index":0,"part":item["content"][0]})));
            } else if item["type"] == "reasoning" {
                if item.get("encrypted_content").is_some() {
                    out.push_str(&self.emit(json!({"type":"response.output_item.added","output_index":index,"item":item})));
                    out.push_str(&self.emit(json!({"type":"response.output_item.done","output_index":index,"item":item})));
                    continue;
                }
                if item_id == &late_id {
                    self.emit_reasoning_delta(&mut out, &late_id, index, &late_reasoning, true);
                    self.finish_reasoning(&mut out, index, item);
                } else if !self.text_open {
                    self.finish_reasoning(&mut out, index, item);
                }
                continue;
            } else if item["type"] == "tool_search_call" {
                // Search arguments are structured JSON; publish the complete client call.
                let mut start = item.clone();
                start["status"] = "in_progress".into();
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
                if !self.resident.as_ref().is_some_and(|state| state.streamed(item)) {
                    out.push_str(&self.emit(
                        json!({"type":"response.output_item.added","output_index":index,"item":start}),
                    ));
                    out.push_str(&self.emit(json!({"type":delta_event,"item_id":item_id,"output_index":index,"delta":item[field]})));
                }
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

    fn finish_compaction(&mut self) -> Result<String, StatusCode> {
        let mut response = self.initial();
        self.done = true;
        if let (Some(input), Some(output)) =
            (self.usage["prompt_tokens"].as_u64(), self.usage["completion_tokens"].as_u64())
            && let Some(total) = input.checked_add(output)
        {
            response["usage"] =
                json!({"input_tokens":input,"output_tokens":output,"total_tokens":total});
        }
        let incomplete = match self.reason.as_str() {
            Some("stop") => None,
            Some("length") => Some("max_output_tokens"),
            Some("content_filter") => Some("content_filter"),
            _ => return Err(StatusCode::BAD_GATEWAY),
        };
        if let Some(reason) = incomplete {
            response["status"] = "incomplete".into();
            response["incomplete_details"] = json!({"reason":reason});
            return Ok(self.emit(json!({"type":"response.incomplete","response":response})));
        }
        if self.text.trim().is_empty() {
            return Err(StatusCode::BAD_GATEWAY);
        }
        let item = json!({
            "type": "compaction",
            "id": format!("cmp_{}", uuid::Uuid::new_v4().simple()),
            "encrypted_content": vw_agent_apps::routing::compaction_replay::encode_summary(&self.text),
        });
        let mut out = self.emit(json!({
            "type":"response.output_item.done","output_index":0,"item":item
        }));
        response["status"] = "completed".into();
        response["output"] = json!([item]);
        out.push_str(&self.emit(json!({"type":"response.completed","response":response})));
        Ok(out)
    }

    fn frame(&mut self, data: &str) -> Result<String, StatusCode> {
        if self.done {
            return Err(StatusCode::BAD_GATEWAY);
        }
        if data == "[DONE]" {
            if self.native.is_some() {
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
        if let Some(native) = &self.native {
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
                    &native.identity,
                    &self.id,
                    &native.references,
                    terminal && native.store,
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
        if let Some(state) = &mut self.resident {
            state.normalize(&mut chunk)?;
        }
        let choices = chunk["choices"].as_array().ok_or(StatusCode::BAD_GATEWAY)?;
        if choices.len() > 1 {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        if (self.compact || self.resident.is_some())
            && !self.reason.is_null()
            && !choices.is_empty()
        {
            return Err(StatusCode::BAD_GATEWAY);
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
            super::messages::object_keys(
                delta,
                &["role", "content", "tool_calls", "refusal", "reasoning_content"],
            )?;
            if let Some(reasoning) = delta.get("reasoning_content").filter(|value| !value.is_null())
            {
                let reasoning = reasoning.as_str().ok_or(StatusCode::BAD_GATEWAY)?;
                if !self.compact && !reasoning.is_empty() {
                    self.retained += reasoning.len();
                    if self.text_open {
                        // Codex tracks one active item. Defer late reasoning until text closes.
                        self.late_reasoning.push_str(reasoning);
                    } else {
                        let item_id = format!("{}_rs", self.id);
                        self.emit_reasoning_delta(
                            &mut out,
                            &item_id,
                            0,
                            reasoning,
                            self.reasoning.is_empty(),
                        );
                        self.reasoning.push_str(reasoning);
                    }
                }
            }
            if !delta["refusal"].is_null() || delta.get("role").is_some_and(|v| v != "assistant") {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            if let Some(text) = delta["content"].as_str() {
                if !text.is_empty() {
                    self.retained += text.len();
                    let item_id = format!("{}_msg_0", self.id);
                    let index = usize::from(!self.reasoning.is_empty());
                    if !self.compact && !self.text_open {
                        if index == 1 {
                            let item = responses::reasoning_item(
                                &format!("{}_rs", self.id),
                                &self.reasoning,
                            );
                            self.finish_reasoning(&mut out, 0, &item);
                        }
                        out.push_str(&self.emit(json!({"type":"response.output_item.added","output_index":index,"item":{"id":item_id,"type":"message","role":"assistant","status":"in_progress","content":[]}})));
                        out.push_str(&self.emit(json!({"type":"response.content_part.added","item_id":item_id,"output_index":index,"content_index":0,"part":{"type":"output_text","text":"","annotations":[],"logprobs":[]}})));
                        self.text_open = true;
                    }
                    self.text.push_str(text);
                    if !self.compact {
                        out.push_str(&self.emit(json!({"type":"response.output_text.delta","item_id":item_id,"output_index":index,"content_index":0,"delta":text,"logprobs":[]})));
                    }
                }
            } else if !delta["content"].is_null() {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            if let Some(calls) = delta.get("tool_calls") {
                for call in calls.as_array().ok_or(StatusCode::BAD_GATEWAY)? {
                    if self.compact {
                        return Err(StatusCode::BAD_GATEWAY);
                    }
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
                            self.retained += fragment.len();
                            let target = if key == "id" {
                                &mut tool[key]
                            } else {
                                &mut tool["function"][key]
                            };
                            let Value::String(target) = target else {
                                return Err(StatusCode::BAD_GATEWAY);
                            };
                            target.push_str(fragment);
                        }
                    }
                }
            }
            if !choice["finish_reason"].is_null() {
                if !(self.compact && choice["finish_reason"] == "content_filter") {
                    super::messages::finish(&choice["finish_reason"])?;
                }
                self.reason = choice["finish_reason"].clone();
            }
        }
        if self.retained > LIMIT {
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }
        if let Some(state) = &mut self.resident {
            let events = state.deltas(&self.tools, &self.tool_names, &self.id)?;
            for event in events {
                out.push_str(&self.emit(event));
            }
        }
        Ok(out)
    }
    fn push(&mut self, bytes: &[u8]) -> Result<String, StatusCode> {
        if self.resident.is_none() {
            self.observer.push(bytes).map_err(|error| match error {
                super::stream_observer::InvalidStream::LimitExceeded => {
                    StatusCode::PAYLOAD_TOO_LARGE
                }
                super::stream_observer::InvalidStream::Malformed => StatusCode::BAD_GATEWAY,
            })?;
        }
        let mut out = String::new();
        for line in self.pending.push(bytes, LIMIT).map_err(|error| match error {
            super::stream_observer::InvalidStream::LimitExceeded => StatusCode::PAYLOAD_TOO_LARGE,
            super::stream_observer::InvalidStream::Malformed => StatusCode::BAD_GATEWAY,
        })? {
            if line.is_empty() {
                if !self.data.is_empty() {
                    let data = std::mem::take(&mut self.data);
                    out.push_str(&self.frame(data.trim_end_matches('\n'))?);
                }
            } else if let Some(data) = line.strip_prefix("data:") {
                if self.data.len().saturating_add(data.len()).saturating_add(1) > LIMIT {
                    return Err(StatusCode::PAYLOAD_TOO_LARGE);
                }
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
    if !output.streaming {
        let id = responses::response_id();
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
    let model = output.identity.model.clone();
    let native = output.native.then_some(Native {
        identity: output.identity,
        store: output.store,
        references: output.references,
    });
    let decoder = Decoder::new(model, output.tool_names, output.codex_chat, native);
    Response::builder()
        .header("content-type", "text/event-stream")
        .body(decoded_body(response.bytes_stream(), decoder, cancel))
        .map_err(|_| StatusCode::BAD_GATEWAY)
}

pub(super) fn codex_chat_body(
    body: Body,
    model: String,
    tools: ChatTools,
    compact: bool,
    cancel: CancellationToken,
) -> Body {
    let mut decoder = Decoder::new(model, tools, true, None);
    decoder.compact = compact;
    decoded_body(body.into_data_stream(), decoder, cancel)
}

pub(super) fn resident_chat_body(
    body: Body,
    model: String,
    tools: ChatTools,
    cancel: CancellationToken,
) -> Body {
    let mut decoder = Decoder::new(model, tools, true, None);
    decoder.resident = Some(super::responses_chat_stream::State::default());
    decoded_body(body.into_data_stream(), decoder, cancel)
}

fn decoded_body<E: Send + 'static>(
    upstream: impl Stream<Item = Result<Bytes, E>> + Send + 'static,
    decoder: Decoder,
    cancel: CancellationToken,
) -> Body {
    let stream = futures_util::stream::unfold(
        (Box::pin(upstream), decoder, cancel),
        |(mut stream, mut decoder, cancel)| async move {
            if decoder.done {
                return None;
            }
            loop {
                let next = tokio::select! {biased;()=cancel.cancelled()=>return None,next=stream.next()=>next};
                let out = match next {
                    Some(Ok(bytes)) => match decoder.push(&bytes) {
                        Ok(out) if out.is_empty() => continue,
                        Ok(out) => out,
                        Err(StatusCode::PAYLOAD_TOO_LARGE) => decoder.failure(
                            "Upstream Responses event or output exceeded the relay size limit",
                        ),
                        Err(_) => {
                            decoder.failure("Unsupported or invalid upstream Responses conversion")
                        }
                    },
                    Some(Err(_)) => decoder.failure("Upstream Responses stream read failed"),
                    None => decoder
                        .failure("Upstream Responses stream ended without a completion event"),
                };
                return Some((
                    Ok::<_, std::io::Error>(Bytes::from(out)),
                    (stream, decoder, cancel),
                ));
            }
        },
    );
    Body::from_stream(stream)
}

pub(super) fn anthropic_chat_body(
    body: Body,
    model: String,
    tools: ChatTools,
    cancel: CancellationToken,
    blocks: Arc<std::sync::Mutex<Vec<Value>>>,
) -> Body {
    let mut decoder = Decoder::new(model, tools, true, None);
    decoder.resident = Some(super::responses_chat_stream::State::default());
    decoder.signed_blocks = Some(blocks);
    decoded_body(body.into_data_stream(), decoder, cancel)
}

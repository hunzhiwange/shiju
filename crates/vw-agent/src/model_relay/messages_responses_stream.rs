//! Resident Responses SSE differences. Framing and final result validation stay shared.
use axum::{
    body::{Body, Bytes},
    http::StatusCode,
    response::Response,
};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;

const INVALID: StatusCode = StatusCode::BAD_GATEWAY;
const LIMIT: usize = 4 * 1024 * 1024;

fn event(value: Value) -> String {
    format!("event: {}\ndata: {value}\n\n", value["type"].as_str().unwrap_or("error"))
}

struct Block {
    index: usize,
    kind: &'static str,
    text: String,
    closed: bool,
}

pub(super) struct Decoder {
    model: String,
    id: Option<String>,
    pub(super) done: bool,
    lines: super::sse_framing::Lines,
    data: String,
    size: usize,
    items: BTreeMap<u64, Value>,
    blocks: BTreeMap<(u64, u64), Block>,
}

impl Decoder {
    pub(super) fn new(model: String) -> Self {
        Self {
            model,
            id: None,
            done: false,
            lines: Default::default(),
            data: String::new(),
            size: 0,
            items: BTreeMap::new(),
            blocks: BTreeMap::new(),
        }
    }

    fn start(&mut self, response: &Value) -> Result<String, StatusCode> {
        let id = response["id"].as_str().filter(|id| !id.is_empty()).ok_or(INVALID)?;
        if let Some(previous) = &self.id {
            return if previous == id { Ok(String::new()) } else { Err(INVALID) };
        }
        self.id = Some(id.into());
        Ok(event(json!({"type":"message_start","message":{"id":id,"type":"message",
            "role":"assistant","model":self.model,"content":[],"stop_reason":null,
            "stop_sequence":null,"usage":{"input_tokens":0,"output_tokens":0}}})))
    }

    fn block(
        &mut self,
        key: (u64, u64),
        kind: &'static str,
        out: &mut String,
    ) -> Result<(), StatusCode> {
        if let Some(block) = self.blocks.get(&key) {
            return if block.kind == kind { Ok(()) } else { Err(INVALID) };
        }
        if self.blocks.len() >= 1024 {
            return Err(INVALID);
        }
        let index = self.blocks.len();
        let content = match kind {
            "text" => json!({"type":"text","text":""}),
            "thinking" => json!({"type":"thinking","thinking":""}),
            "tool_use" => {
                let item = self.items.get(&key.0).ok_or(INVALID)?;
                let id = item["call_id"].as_str().filter(|s| !s.is_empty()).ok_or(INVALID)?;
                let name = item["name"].as_str().filter(|s| !s.is_empty()).ok_or(INVALID)?;
                json!({"type":"tool_use","id":id,"name":name,"input":{}})
            }
            _ => return Err(INVALID),
        };
        out.push_str(&event(
            json!({"type":"content_block_start","index":index,"content_block":content}),
        ));
        self.blocks.insert(key, Block { index, kind, text: String::new(), closed: false });
        Ok(())
    }

    fn text(
        &mut self,
        key: (u64, u64),
        kind: &'static str,
        text: &str,
        snapshot: bool,
        out: &mut String,
    ) -> Result<(), StatusCode> {
        self.block(key, kind, out)?;
        let block = self.blocks.get_mut(&key).ok_or(INVALID)?;
        let suffix = if snapshot { text.strip_prefix(&block.text).ok_or(INVALID)? } else { text };
        if suffix.is_empty() {
            return Ok(());
        }
        if block.closed {
            return Err(INVALID);
        }
        let delta = match kind {
            "text" => json!({"type":"text_delta","text":suffix}),
            "thinking" => json!({"type":"thinking_delta","thinking":suffix}),
            "tool_use" => json!({"type":"input_json_delta","partial_json":suffix}),
            _ => return Err(INVALID),
        };
        block.text.push_str(suffix);
        out.push_str(&event(
            json!({"type":"content_block_delta","index":block.index,"delta":delta}),
        ));
        Ok(())
    }

    fn close(&mut self, key: (u64, u64), out: &mut String) -> Result<(), StatusCode> {
        let block = self.blocks.get_mut(&key).ok_or(INVALID)?;
        if !block.closed {
            if block.kind == "tool_use" {
                let input: Value = serde_json::from_str(&block.text).map_err(|_| INVALID)?;
                if !input.is_object() {
                    return Err(INVALID);
                }
            }
            out.push_str(&event(json!({"type":"content_block_stop","index":block.index})));
            block.closed = true;
        }
        Ok(())
    }

    fn item(
        &mut self,
        index: u64,
        item: &Value,
        complete: bool,
        out: &mut String,
    ) -> Result<(), StatusCode> {
        if index > 1024 {
            return Err(INVALID);
        }
        if let Some(previous) = self.items.get(&index) {
            for field in ["id", "type", "call_id", "name"] {
                if previous.get(field).is_some_and(|v| !v.is_null() && item.get(field) != Some(v)) {
                    return Err(INVALID);
                }
            }
        }
        self.items.insert(index, item.clone());
        match item["type"].as_str() {
            Some("function_call") => {
                self.block((index, 0), "tool_use", out)?;
                if let Some(args) = item["arguments"].as_str() {
                    self.text((index, 0), "tool_use", args, true, out)?;
                } else if complete {
                    return Err(INVALID);
                }
            }
            Some("message" | "reasoning") => {
                let thinking = item["type"] == "reasoning";
                let parts = &item[if thinking { "summary" } else { "content" }];
                if let Some(parts) = parts.as_array() {
                    if complete
                        && self
                            .blocks
                            .keys()
                            .any(|key| key.0 == index && key.1 >= parts.len() as u64)
                    {
                        return Err(INVALID);
                    }
                    for (part_index, part) in parts.iter().enumerate() {
                        let (kind, text) = match part["type"].as_str() {
                            Some("output_text") if !thinking => ("text", &part["text"]),
                            Some("refusal") if !thinking => ("text", &part["refusal"]),
                            Some("summary_text") if thinking => ("thinking", &part["text"]),
                            _ => return Err(INVALID),
                        };
                        self.text(
                            (index, part_index as u64),
                            kind,
                            text.as_str().ok_or(INVALID)?,
                            true,
                            out,
                        )?;
                    }
                } else if complete {
                    return Err(INVALID);
                }
            }
            _ => return Err(INVALID),
        }
        if complete {
            let keys: Vec<_> = self.blocks.keys().copied().filter(|key| key.0 == index).collect();
            for key in keys {
                self.close(key, out)?;
            }
        }
        Ok(())
    }

    pub(super) fn frame(&mut self, data: &str) -> Result<String, StatusCode> {
        let value: Value = serde_json::from_str(data).map_err(|_| INVALID)?;
        let ty = value["type"].as_str().ok_or(INVALID)?;
        if self.done {
            return Err(INVALID);
        }
        if matches!(ty, "error" | "response.failed" | "response.cancelled") {
            return Err(INVALID);
        }
        let mut out = String::new();
        if matches!(ty, "response.created" | "response.in_progress") {
            return self.start(&value["response"]);
        }
        if matches!(ty, "response.completed" | "response.incomplete") {
            let response = &value["response"];
            out.push_str(&self.start(response)?);
            if response["status"] != ty.trim_start_matches("response.") {
                return Err(INVALID);
            }
            let output = response["output"].as_array().ok_or(INVALID)?;
            // Reuse the result owner's status, tools and cache accounting. Reasoning
            // summaries are streamed separately and never become executable tools.
            let mut normalized = response.clone();
            normalized["output"] =
                output.iter().filter(|v| v["type"] != "reasoning").cloned().collect();
            let final_message =
                super::messages_responses_result::from_responses(&normalized, &self.model)?;
            if self.items.keys().any(|index| *index as usize >= output.len()) {
                return Err(INVALID);
            }
            for (index, item) in output.iter().enumerate() {
                self.item(index as u64, item, true, &mut out)?;
            }
            out.push_str(&event(json!({"type":"message_delta","delta":{
                "stop_reason":final_message["stop_reason"],"stop_sequence":null},"usage":final_message["usage"]})));
            out.push_str(&event(json!({"type":"message_stop"})));
            self.done = true;
            return Ok(out);
        }
        if self.id.is_none() {
            return Err(INVALID);
        }
        if matches!(ty, "response.queued") {
            return Ok(out);
        }
        let index = value["output_index"].as_u64().ok_or(INVALID)?;
        if matches!(ty, "response.output_item.added" | "response.output_item.done") {
            self.item(index, &value["item"], ty.ends_with(".done"), &mut out)?;
            return Ok(out);
        }
        let item = self.items.get(&index).ok_or(INVALID)?;
        if value.get("item_id").is_some_and(|id| item.get("id") != Some(id)) {
            return Err(INVALID);
        }
        let (kind, part, field, snapshot, close) = match ty {
            "response.output_text.delta" | "response.refusal.delta" => {
                ("text", "content_index", "delta", false, false)
            }
            "response.output_text.done" => ("text", "content_index", "text", true, true),
            "response.refusal.done" => ("text", "content_index", "refusal", true, true),
            "response.reasoning_summary_text.delta" => {
                ("thinking", "summary_index", "delta", false, false)
            }
            "response.reasoning_summary_text.done" => {
                ("thinking", "summary_index", "text", true, true)
            }
            "response.function_call_arguments.delta" => ("tool_use", "", "delta", false, false),
            "response.function_call_arguments.done" => ("tool_use", "", "arguments", true, true),
            "response.content_part.added"
            | "response.content_part.done"
            | "response.reasoning_summary_part.added"
            | "response.reasoning_summary_part.done" => {
                let thinking = ty.starts_with("response.reasoning_");
                let part_index = value[if thinking { "summary_index" } else { "content_index" }]
                    .as_u64()
                    .ok_or(INVALID)?;
                let part = &value["part"];
                if item["type"] != if thinking { "reasoning" } else { "message" } {
                    return Err(INVALID);
                }
                let text = match part["type"].as_str() {
                    Some("output_text") if !thinking => &part["text"],
                    Some("refusal") if !thinking => &part["refusal"],
                    Some("summary_text") if thinking => &part["text"],
                    _ => return Err(INVALID),
                };
                self.text(
                    (index, part_index),
                    if thinking { "thinking" } else { "text" },
                    text.as_str().ok_or(INVALID)?,
                    true,
                    &mut out,
                )?;
                if ty.ends_with(".done") {
                    self.close((index, part_index), &mut out)?;
                }
                return Ok(out);
            }
            _ => return Err(INVALID),
        };
        let expected = match kind {
            "text" => "message",
            "thinking" => "reasoning",
            _ => "function_call",
        };
        if item["type"] != expected {
            return Err(INVALID);
        }
        let part = if part.is_empty() { 0 } else { value[part].as_u64().ok_or(INVALID)? };
        self.text((index, part), kind, value[field].as_str().ok_or(INVALID)?, snapshot, &mut out)?;
        if close {
            self.close((index, part), &mut out)?;
        }
        Ok(out)
    }

    pub(super) fn push(&mut self, bytes: &[u8]) -> Result<String, StatusCode> {
        let mut out = String::new();
        for fragment in bytes.split_inclusive(|b| *b == b'\n') {
            if self.done {
                break;
            }
            self.size = self.size.checked_add(fragment.len()).ok_or(INVALID)?;
            if self.size > LIMIT {
                return Err(INVALID);
            }
            for line in self.lines.push(fragment, LIMIT).map_err(|_| INVALID)? {
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
        }
        Ok(out)
    }
}

pub(crate) async fn response_body(
    response: Response,
    model: String,
    cancel: CancellationToken,
) -> Result<Response, StatusCode> {
    if !response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("text/event-stream"))
    {
        return Err(INVALID);
    }
    let stream = futures_util::stream::unfold(
        (response.into_body().into_data_stream(), Decoder::new(model), cancel),
        |(mut stream, mut decoder, cancel)| async move {
            if decoder.done {
                return None;
            }
            loop {
                let next = tokio::select! { biased; () = cancel.cancelled() => return None, next = stream.next() => next };
                let result = match next {
                    Some(Ok(bytes)) => decoder.push(&bytes),
                    _ => Err(INVALID),
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
                        let out = event(json!({"type":"error","error":{"type":"api_error",
                            "message":"Unsupported or invalid upstream Responses conversion"}}));
                        return Some((Ok(Bytes::from(out)), (stream, decoder, cancel)));
                    }
                }
            }
        },
    );
    Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from_stream(stream))
        .map_err(|_| INVALID)
}

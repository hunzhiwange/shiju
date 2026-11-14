//! Anthropic content-block deltas feed the shared resident Responses lifecycle.
use axum::{body::Body, http::StatusCode};
use futures_util::StreamExt;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

const BAD: StatusCode = StatusCode::BAD_GATEWAY;
const LIMIT: usize = 4 * 1024 * 1024;

#[derive(Default)]
struct Decoder {
    lines: super::sse_framing::Lines,
    data: String,
    started: bool,
    stopped: bool,
    reason: Option<String>,
    active: Option<(u64, String)>,
    next_index: u64,
    tool_has_args: bool,
    usage: Value,
    retained: usize,
}

impl Decoder {
    fn chunk(delta: Value, reason: Value, usage: Value) -> String {
        format!("data: {}\n\n", json!({"choices":[{"index":0,"delta":delta,"finish_reason":reason}],"usage":usage}))
    }

    fn frame(&mut self, v: Value) -> Result<String, StatusCode> {
        if self.stopped { return Err(BAD); }
        let kind = v["type"].as_str().ok_or(BAD)?;
        if kind == "ping" { return Ok(String::new()); }
        if !self.started && kind != "message_start" { return Err(BAD); }
        let delta = match kind {
            "message_start" => {
                if self.started || v["message"]["role"] != "assistant"
                    || !v["message"]["content"].as_array().is_some_and(Vec::is_empty) { return Err(BAD); }
                self.started = true;
                self.usage = v["message"]["usage"].clone();
                json!({"role":"assistant"})
            }
            "content_block_start" => {
                if self.active.is_some() || self.reason.is_some() || v["index"].as_u64() != Some(self.next_index) { return Err(BAD); }
                let block = &v["content_block"];
                let ty = block["type"].as_str().ok_or(BAD)?;
                let delta = match ty {
                    "text" => json!({"content":block["text"].as_str().ok_or(BAD)?}),
                    "thinking" => json!({"reasoning_content":block["thinking"].as_str().ok_or(BAD)?}),
                    "tool_use" => {
                        if !block["input"].as_object().is_some_and(|m| m.is_empty()) { return Err(BAD); }
                        json!({"tool_calls":[{"index":self.next_index,"id":block["id"].as_str().filter(|s| !s.is_empty()).ok_or(BAD)?,"type":"function","function":{"name":block["name"].as_str().filter(|s| !s.is_empty()).ok_or(BAD)?,"arguments":""}}]})
                    }
                    _ => return Err(BAD),
                };
                self.tool_has_args = false;
                self.active = Some((self.next_index, ty.into()));
                delta
            }
            "content_block_delta" => {
                let (index, ty) = self.active.as_ref().ok_or(BAD)?;
                if v["index"].as_u64() != Some(*index) { return Err(BAD); }
                let d = &v["delta"];
                if d["type"] == "input_json_delta" { self.tool_has_args = true; }
                match (ty.as_str(), d["type"].as_str()) {
                    ("text", Some("text_delta")) => json!({"content":d["text"].as_str().ok_or(BAD)?}),
                    ("thinking", Some("thinking_delta")) => json!({"reasoning_content":d["thinking"].as_str().ok_or(BAD)?}),
                    ("thinking", Some("signature_delta")) => {
                        d["signature"].as_str().ok_or(BAD)?;
                        // Signatures are provider replay credentials, not reasoning text.
                        return Ok(String::new());
                    }
                    ("tool_use", Some("input_json_delta")) => json!({"tool_calls":[{"index":index,"function":{"arguments":d["partial_json"].as_str().ok_or(BAD)?}}]}),
                    _ => return Err(BAD),
                }
            }
            "content_block_stop" => {
                let (index, ty) = self.active.take().ok_or(BAD)?;
                if v["index"].as_u64() != Some(index) { return Err(BAD); }
                self.next_index += 1;
                if ty == "tool_use" && !self.tool_has_args {
                    return Ok(Self::chunk(json!({"tool_calls":[{"index":index,"function":{"arguments":"{}"}}]}), Value::Null, Value::Null));
                }
                return Ok(String::new());
            }
            "message_delta" => {
                if self.active.is_some() || self.reason.is_some() { return Err(BAD); }
                let reason = match v["delta"]["stop_reason"].as_str() {
                    Some("end_turn" | "stop_sequence") => "stop",
                    Some("tool_use") => "tool_calls",
                    Some("max_tokens" | "model_context_window_exceeded") => "length",
                    _ => return Err(BAD),
                };
                self.reason = Some(reason.into());
                if let Some(usage) = v["usage"].as_object() {
                    let target = self.usage.as_object_mut().ok_or(BAD)?;
                    target.extend(usage.clone());
                }
                return Ok(String::new());
            }
            "message_stop" => {
                if self.active.is_some() { return Err(BAD); }
                let reason = self.reason.as_ref().ok_or(BAD)?;
                let input = self.usage["input_tokens"].as_u64().ok_or(BAD)?;
                let output = self.usage["output_tokens"].as_u64().ok_or(BAD)?;
                let cached = self.usage["cache_read_input_tokens"].as_u64().unwrap_or(0);
                let write = self.usage["cache_creation_input_tokens"].as_u64().unwrap_or(0);
                let input = input.checked_add(cached).and_then(|n| n.checked_add(write)).ok_or(BAD)?;
                self.stopped = true;
                let mut out = Self::chunk(json!({}), json!(reason), json!({"prompt_tokens":input,"completion_tokens":output,"prompt_tokens_details":{"cached_tokens":cached}}));
                out.push_str("data: [DONE]\n\n");
                return Ok(out);
            }
            _ => return Err(BAD),
        };
        Ok(Self::chunk(delta, Value::Null, Value::Null))
    }

    fn push(&mut self, bytes: &[u8]) -> Result<String, StatusCode> {
        self.retained = self.retained.checked_add(bytes.len()).filter(|n| *n <= LIMIT).ok_or(BAD)?;
        let mut out = String::new();
        for line in self.lines.push(bytes, LIMIT).map_err(|_| BAD)? {
            if line.is_empty() {
                if !self.data.is_empty() {
                    let data = std::mem::take(&mut self.data);
                    out.push_str(&self.frame(serde_json::from_str(&data).map_err(|_| BAD)?)?);
                }
            } else if let Some(data) = line.strip_prefix("data:") {
                self.data.push_str(data.strip_prefix(' ').unwrap_or(data));
                self.data.push('\n');
            }
        }
        Ok(out)
    }
}

pub(super) fn response_body(body: Body, model: String, tools: super::responses::ChatTools, cancel: CancellationToken) -> Body {
    let upstream = futures_util::stream::unfold((body.into_data_stream(), Decoder::default(), false), |(mut stream, mut decoder, mut ended)| async move {
        if ended { return None; }
        loop {
            let next = stream.next().await;
            let result = match next {
                Some(Ok(bytes)) => decoder.push(&bytes),
                Some(Err(_)) => Err(BAD),
                None if decoder.stopped => return None,
                None => Err(BAD),
            };
            match result {
                Ok(out) if out.is_empty() => continue,
                Ok(out) => return Some((Ok::<_, std::io::Error>(out), (stream, decoder, ended))),
                Err(_) => {
                    ended = true;
                    return Some((Err(std::io::Error::other("Invalid Anthropic stream")), (stream, decoder, ended)));
                }
            }
        }
    });
    super::responses_stream::resident_chat_body(Body::from_stream(upstream), model, tools, cancel)
}

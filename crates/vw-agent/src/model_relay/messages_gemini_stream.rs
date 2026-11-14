//! Resident Gemini SSE differences. Tools remain client output, never executable work.
use axum::{
    body::{Body, Bytes},
    http::StatusCode,
};
use futures_util::StreamExt;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

const LIMIT: usize = 4 * 1024 * 1024;
const BAD: StatusCode = StatusCode::BAD_GATEWAY;

fn event(value: Value) -> String {
    format!("event: {}\ndata: {value}\n\n", value["type"].as_str().unwrap_or("error"))
}

struct Tool {
    call: Value,
    generated: bool,
    signature: Option<Value>,
}

pub(super) struct Decoder {
    shadow: Option<super::gemini_shadow::Context>,
    observer: super::stream_observer::StreamObserver,
    lines: super::sse_framing::Lines,
    data: String,
    event_name: String,
    size: usize,
    model: String,
    id: String,
    upstream_id: Option<String>,
    started: bool,
    done: bool,
    text_open: bool,
    text: String,
    tools: Vec<Tool>,
    reason: Option<String>,
    usage: Option<Value>,
}

impl Decoder {
    pub(super) fn new(model: String) -> Self {
        Self {
            shadow: None,
            observer: super::stream_observer::StreamObserver::new(
                super::generation::GenerationProtocol::Gemini,
            ),
            lines: Default::default(),
            data: String::new(),
            event_name: String::new(),
            size: 0,
            model,
            id: format!("msg_{}", uuid::Uuid::new_v4().simple()),
            upstream_id: None,
            started: false,
            done: false,
            text_open: false,
            text: String::new(),
            tools: Vec::new(),
            reason: None,
            usage: None,
        }
    }

    pub(super) fn push(&mut self, bytes: &[u8]) -> Result<String, StatusCode> {
        self.size = self.size.checked_add(bytes.len()).filter(|size| *size <= LIMIT).ok_or(BAD)?;
        let mut output = String::new();
        for line in self.lines.push(bytes, LIMIT).map_err(|_| BAD)? {
            if line.is_empty() {
                if !self.data.is_empty() {
                    let data = std::mem::take(&mut self.data);
                    if self.event_name == "error" {
                        return Err(BAD);
                    }
                    output.push_str(&self.frame(data.trim_end_matches('\n'))?);
                }
                self.event_name.clear();
            } else if let Some(data) = line.strip_prefix("data:") {
                self.data.push_str(data.strip_prefix(' ').unwrap_or(data));
                self.data.push('\n');
            } else if let Some(name) = line.strip_prefix("event:") {
                self.event_name = name.trim_start_matches(' ').into();
            }
        }
        Ok(output)
    }

    fn start(&mut self) -> String {
        if self.started {
            return String::new();
        }
        self.started = true;
        event(json!({"type":"message_start","message":{"id":self.id,
            "type":"message","role":"assistant","model":self.model,"content":[],
            "stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":0,"output_tokens":0}}}))
    }

    fn frame(&mut self, data: &str) -> Result<String, StatusCode> {
        if self.done {
            return Err(BAD);
        }
        let mut value: Value = serde_json::from_str(data).map_err(|_| BAD)?;
        if !value.is_object() || value.get("error").is_some_and(|v| !v.is_null()) {
            return Err(BAD);
        }
        if let Some(id) = value.get("responseId") {
            let id = id.as_str().filter(|id| !id.is_empty()).ok_or(BAD)?;
            if self.upstream_id.as_deref().is_some_and(|previous| previous != id) {
                return Err(BAD);
            }
            if !self.started {
                self.id = id.into();
            }
            self.upstream_id = Some(id.into());
        }
        // The resident JSON policy represents a prompt block as a refusal, while
        // the strict native observer deliberately rejects promptFeedback blocks.
        if let Some(reason) = value.pointer("/promptFeedback/blockReason") {
            reason.as_str().filter(|reason| !reason.is_empty()).ok_or(BAD)?;
            value["candidates"] =
                json!([{"index":0,"finishReason":"SAFETY","content":{"parts":[]}}]);
            value.as_object_mut().ok_or(BAD)?.remove("promptFeedback");
        }
        let mut observed = value.clone();
        if self.shadow.is_some() {
            if let Some(parts) = observed.pointer_mut("/candidates/0/content/parts").and_then(Value::as_array_mut) {
                for part in parts {
                    if let Some(object) = part.as_object_mut() {
                        object.remove("thoughtSignature");
                        object.remove("thought_signature");
                    }
                }
            }
        }
        self.observer.push(format!("data: {observed}\n\n").as_bytes()).map_err(|_| BAD)?;
        if let Some(usage) = value.get("usageMetadata") {
            if !usage.is_object() {
                return Err(BAD);
            }
            self.usage = Some(usage.clone());
        }
        let mut output = self.start();
        let Some(candidates) = value.get("candidates") else {
            return Ok(output);
        };
        let candidates = candidates.as_array().filter(|c| c.len() <= 1).ok_or(BAD)?;
        let Some(candidate) = candidates.first() else {
            return Ok(output);
        };
        if !candidate.is_object() || candidate.get("index").is_some_and(|index| index != 0) {
            return Err(BAD);
        }
        if candidate["content"].get("role").is_some_and(|role| role != "model") {
            return Err(BAD);
        }
        let refusal = matches!(
            candidate["finishReason"].as_str(),
            Some("SAFETY" | "RECITATION" | "SPII" | "BLOCKLIST" | "PROHIBITED_CONTENT")
        );
        if let Some(reason) = candidate.get("finishReason") {
            let reason = reason.as_str().ok_or(BAD)?;
            if !refusal && !matches!(reason, "STOP" | "MAX_TOKENS") {
                return Err(BAD);
            }
            self.reason = Some(reason.into());
        }
        if refusal {
            return Ok(output);
        }
        let Some(parts) = candidate["content"].get("parts") else {
            return Ok(output);
        };
        let mut visible = String::new();
        let mut position = 0;
        let mut ids = std::collections::BTreeSet::new();
        for part in parts.as_array().ok_or(BAD)? {
            super::messages::object_keys(
                part,
                &["text", "functionCall", "thought", "thoughtSignature", "thought_signature"],
            )?;
            if part.get("thought").is_some_and(|v| !v.is_boolean()) {
                return Err(BAD);
            }
            for key in ["thoughtSignature", "thought_signature"] {
                if part.get(key).is_some_and(|v| !v.is_null() && !v.is_string()) {
                    return Err(BAD);
                }
            }
            match (part.get("text"), part.get("functionCall")) {
                (Some(text), None) => {
                    let text = text.as_str().ok_or(BAD)?;
                    // Source thoughts and opaque signatures are not client-visible
                    // text. Replay receipts remain owned by the thinking task.
                    if part["thought"] != true {
                        visible.push_str(text);
                    }
                }
                (None, Some(call)) if part["thought"] != true => {
                    super::messages::object_keys(call, &["id", "name", "args"])?;
                    let name = call["name"].as_str().filter(|name| !name.is_empty()).ok_or(BAD)?;
                    let args = call.get("args").cloned().unwrap_or_else(|| json!({}));
                    if !args.is_object() {
                        return Err(BAD);
                    }
                    let id = match call.get("id") {
                        None | Some(Value::Null) => None,
                        Some(Value::String(id)) => (!id.is_empty()).then_some(id.as_str()),
                        _ => return Err(BAD),
                    };
                    if let Some(id) = id {
                        if !ids.insert(id.to_owned()) {
                            return Err(BAD);
                        }
                    }
                    let existing = id
                        .and_then(|id| self.tools.iter().position(|t| t.call["id"] == id))
                        .or_else(|| {
                            self.tools.get(position).filter(|t| t.generated).map(|_| position)
                        });
                    let signature = part.get("thoughtSignature").or_else(|| part.get("thought_signature")).filter(|v| !v.is_null()).cloned();
                    let mut call = json!({"name":name,"args":args});
                    if let Some(index) = existing {
                        let tool = &mut self.tools[index];
                        if tool.call["name"] != name {
                            return Err(BAD);
                        }
                        call["id"] = id.map_or_else(|| tool.call["id"].clone(), |id| json!(id));
                        if signature.is_some() { tool.signature = signature; }
                        tool.generated &= id.is_none();
                        tool.call = call;
                    } else {
                        if self.tools.len() >= 1023 {
                            return Err(BAD);
                        }
                        call["id"] =
                            json!(id.map(str::to_owned).unwrap_or_else(|| format!(
                                "toolu_{}",
                                uuid::Uuid::new_v4().simple()
                            )));
                        self.tools.push(Tool { call, generated: id.is_none(), signature });
                    }
                    position += 1;
                }
                _ => return Err(BAD),
            }
        }
        // Source relays can send cumulative text or ordinary incremental chunks.
        let delta = visible.strip_prefix(self.text.as_str()).unwrap_or(&visible);
        if !delta.is_empty() {
            if !self.text_open {
                output.push_str(&event(json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}})));
                self.text_open = true;
            }
            output.push_str(&event(json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":delta}})));
            self.text.push_str(delta);
        }
        Ok(output)
    }

    pub(super) fn finish(&mut self) -> Result<String, StatusCode> {
        self.observer.finish().map_err(|_| BAD)?;
        if self.done || !self.lines.is_empty() || !self.data.is_empty() {
            return Err(BAD);
        }
        let parts = self.tools.iter().map(|t| {
            let mut call = t.call.clone();
            if t.generated && self.shadow.is_some() { call.as_object_mut().expect("validated call").remove("id"); }
            let mut part = json!({"functionCall":call});
            if let Some(signature) = &t.signature { part["thoughtSignature"] = signature.clone(); }
            part
        }).collect::<Vec<_>>();
        let mut body = json!({"responseId":self.id,"candidates":[{"index":0,
            "content":{"parts":parts},"finishReason":self.reason.as_deref().ok_or(BAD)?}]});
        if let Some(usage) = &self.usage {
            body["usageMetadata"] = usage.clone();
        }
        if let Some(shadow) = &self.shadow { shadow.capture(&mut body)?; }
        let result = super::messages_gemini_result::convert(&body, &self.model)?;
        let mut output = self.start();
        let mut index = 0;
        if self.text_open {
            output.push_str(&event(json!({"type":"content_block_stop","index":index})));
            index += 1;
        }
        for block in result["content"].as_array().ok_or(BAD)? {
            output.push_str(&event(json!({"type":"content_block_start","index":index,
                "content_block":{"type":"tool_use","id":block["id"],"name":block["name"],"input":{}}})));
            output.push_str(&event(json!({"type":"content_block_delta","index":index,
                "delta":{"type":"input_json_delta","partial_json":block["input"].to_string()}})));
            output.push_str(&event(json!({"type":"content_block_stop","index":index})));
            index += 1;
        }
        output.push_str(&event(json!({"type":"message_delta","delta":{"stop_reason":result["stop_reason"],"stop_sequence":null},"usage":result["usage"]})));
        output.push_str(&event(json!({"type":"message_stop"})));
        self.done = true;
        Ok(output)
    }
}

/// Select the SSE action on the same authorized Gemini resource.
pub(crate) fn stream_url(target: &reqwest::Url) -> Result<reqwest::Url, StatusCode> {
    if target.query().is_some_and(|query| query != "alt=sse") {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let resource = target.path().rsplit_once("/models/").ok_or(BAD)?.1;
    let model = resource
        .strip_suffix(":generateContent")
        .or_else(|| resource.strip_suffix(":streamGenerateContent"))
        .ok_or(BAD)?;
    let request = super::native_gemini::Request { model: model.into(), streaming: true };
    let mut base = target.clone();
    base.set_query(None);
    request.url(&base, &request.model)
}

#[cfg(test)]
pub(crate) fn response_body(body: Body, model: String, cancel: CancellationToken) -> Body {
    response_body_with_shadow(body, model, cancel, None)
}

pub(crate) fn response_body_with_shadow(body: Body, model: String, cancel: CancellationToken, shadow: Option<super::gemini_shadow::Context>) -> Body {
    let mut decoder = Decoder::new(model);
    decoder.shadow = shadow;
    Body::from_stream(futures_util::stream::unfold(
        (body.into_data_stream(), decoder, cancel),
        |(mut stream, mut decoder, cancel)| async move {
            if decoder.done {
                return None;
            }
            loop {
                let next = tokio::select! { biased; () = cancel.cancelled() => return None, next = stream.next() => next };
                let output = match next {
                    Some(Ok(bytes)) => decoder.push(&bytes),
                    Some(Err(_)) => Err(BAD),
                    None => decoder.finish(),
                };
                let output = match output {
                    Ok(output) if output.is_empty() => continue,
                    Ok(output) => output,
                    Err(_) => {
                        decoder.done = true;
                        event(
                            json!({"type":"error","error":{"type":"api_error","message":"Invalid or incomplete Gemini upstream stream"}}),
                        )
                    }
                };
                return Some((
                    Ok::<_, std::io::Error>(Bytes::from(output)),
                    (stream, decoder, cancel),
                ));
            }
        },
    ))
}

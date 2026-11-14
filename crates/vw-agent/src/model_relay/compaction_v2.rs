//! Resident-only V2 summary turns. Credentials and cancellation stay with dispatch.
use axum::{body::{Body, Bytes}, http::StatusCode};
use futures_util::StreamExt;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

const LIMIT: usize = 4 * 1024 * 1024;

pub(crate) fn is_request(body: &Value) -> bool {
    body["input"].as_array().is_some_and(|items| {
        items.iter().any(|item| item["type"] == "compaction_trigger")
    })
}

pub(crate) fn prepare_native(body: &mut Value) -> Result<(), StatusCode> {
    if !is_request(body) { return Ok(()); }
    if body["stream"] != true { return Err(StatusCode::UNPROCESSABLE_ENTITY); }
    let items = body["input"].as_array_mut().ok_or(StatusCode::BAD_REQUEST)?;
    let mut count = 0;
    for item in items {
        if item["type"] == "compaction_trigger" {
            super::messages::object_keys(item, &["type"])?;
            count += 1;
            *item = json!({"type":"message","role":"user","content":super::responses_compaction::PROMPT});
        }
    }
    if count != 1 { return Err(StatusCode::BAD_REQUEST); }
    let object = body.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
    if object.contains_key("tools") {
        object.insert("tool_choice".into(), json!("none"));
    } else {
        object.remove("tool_choice");
        object.remove("parallel_tool_calls");
    }
    if let Some(text) = object.get_mut("text").and_then(Value::as_object_mut) {
        text.remove("format");
    }
    Ok(())
}

#[derive(Default)]
struct Decoder {
    lines: super::sse_framing::Lines,
    data: String,
    items: Vec<Value>,
    retained: usize,
    next_index: u64,
    done: bool,
}

fn event(kind: &str, value: Value) -> String {
    format!("event: {kind}\ndata: {value}\n\n")
}

impl Decoder {
    fn frame(&mut self, mut value: Value) -> Result<String, std::io::Error> {
        let kind = value["type"].as_str().unwrap_or("").to_owned();
        if kind == "response.output_item.done" {
            self.next_index = self.next_index.max(value["output_index"].as_u64().unwrap_or(0) + 1);
            self.items.push(value["item"].clone());
        }
        if matches!(kind.as_str(), "response.incomplete" | "response.failed") {
            self.done = true;
        }
        if kind != "response.completed" { return Ok(event(&kind, value)); }
        self.done = true;
        let response = &mut value["response"];
        if response["status"] == "incomplete" {
            return Ok(event("response.incomplete", json!({"type":"response.incomplete","response":response})));
        }
        if response["status"] != "completed" {
            return Err(std::io::Error::other("Invalid compaction terminal status"));
        }
        let items = response["output"].as_array().filter(|items| !items.is_empty()).unwrap_or(&self.items);
        let mut summary = String::new();
        for item in items {
            if item["type"] != "message" { continue; }
            if let Some(content) = item["content"].as_array() {
                for part in content {
                    if part["type"] == "output_text" {
                        summary.push_str(part["text"].as_str().unwrap_or(""));
                    }
                }
            }
        }
        if summary.trim().is_empty() {
            response["status"] = json!("failed");
            response["error"] = json!({"type":"compaction_summary_empty","message":"Upstream returned no summary text for the compaction turn"});
            return Ok(event("response.failed", json!({"type":"response.failed","response":response})));
        }
        let item = json!({"type":"compaction","id":format!("cmp_{}",uuid::Uuid::new_v4().simple()),
            "encrypted_content":vw_agent_apps::routing::compaction_replay::encode_summary(&summary)});
        let index = self.next_index.max(response["output"].as_array().map_or(0, |v| v.len() as u64));
        let mut out = event("response.output_item.done", json!({"type":"response.output_item.done","output_index":index,"item":item}));
        if !response["output"].is_array() { response["output"] = json!([]); }
        response["output"].as_array_mut().unwrap().push(item);
        out.push_str(&event("response.completed", value));
        Ok(out)
    }

    fn push(&mut self, bytes: &[u8]) -> Result<String, std::io::Error> {
        self.retained = self.retained.checked_add(bytes.len()).filter(|n| *n <= LIMIT)
            .ok_or_else(|| std::io::Error::other("Compaction stream exceeded size limit"))?;
        let mut out = String::new();
        for line in self.lines.push(bytes, LIMIT).map_err(|_| std::io::Error::other("Invalid compaction framing"))? {
            if self.done { break; }
            if line.is_empty() && !self.data.is_empty() {
                let data = std::mem::take(&mut self.data);
                let value = serde_json::from_str(&data).map_err(std::io::Error::other)?;
                out.push_str(&self.frame(value)?);
            } else if let Some(data) = line.strip_prefix("data:") {
                self.data.push_str(data.strip_prefix(' ').unwrap_or(data));
                self.data.push('\n');
            }
        }
        Ok(out)
    }
}

/// Dropping this body drops the upstream; the existing request token cancels a pending read.
pub(crate) fn response_body(body: Body, cancel: CancellationToken) -> Body {
    Body::from_stream(futures_util::stream::unfold(
        (body.into_data_stream(), Decoder::default(), cancel),
        |(mut stream, mut decoder, cancel)| async move {
            if decoder.done { return None; }
            loop {
                let next = tokio::select! { biased; () = cancel.cancelled() => return None, next = stream.next() => next };
                let result = match next {
                    Some(Ok(bytes)) => decoder.push(&bytes),
                    Some(Err(_)) => Err(std::io::Error::other("Compaction upstream read failed")),
                    None => decoder.push(b"\n\n").and_then(|out| {
                        if decoder.done { Ok(out) } else { Err(std::io::Error::other("Compaction stream ended before completion")) }
                    }),
                };
                match result {
                    Ok(out) if out.is_empty() => continue,
                    result => {
                        if result.is_err() { decoder.done = true; }
                        return Some((result.map(Bytes::from), (stream, decoder, cancel)));
                    }
                }
            }
        },
    ))
}

//! Transient history for the authorized resident Responses-to-Chat bridge only.
use crate::app_routing::reference_scope::ReferenceScope;
use axum::{body::Body, http::StatusCode};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};

const TTL: Duration = Duration::from_secs(1800);
const LIMIT: usize = 512;
const MAX_BYTES: usize = 4 * 1024 * 1024;
const TOTAL_BYTES: usize = 16 * 1024 * 1024;
struct Entry {
    scope: ReferenceScope,
    id: String,
    input: Vec<Value>,
    born: Instant,
    bytes: usize,
}
#[derive(Default)]
pub(crate) struct Store(parking_lot::Mutex<VecDeque<Entry>>);
impl Store {
    pub(crate) fn clear(&self) {
        self.0.lock().clear();
    }
}
#[derive(Clone)]
pub(crate) struct Context {
    pub(crate) store: Arc<Store>,
    pub(crate) scope: ReferenceScope,
    input: Vec<Value>,
}
impl Context {
    pub(crate) fn prepare(
        store: Arc<Store>,
        scope: ReferenceScope,
        body: &mut Value,
    ) -> Result<Self, StatusCode> {
        let mut input = match &body["input"] {
            Value::String(text) => vec![json!({"role":"user","content":text})],
            Value::Array(items) => items.clone(),
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        if let Some(previous) = body.get("previous_response_id").filter(|v| !v.is_null()) {
            let id = previous.as_str().ok_or(StatusCode::BAD_REQUEST)?;
            let mut entries = store.0.lock();
            entries.retain(|entry| entry.born.elapsed() < TTL);
            let entry = entries
                .iter()
                .find(|entry| entry.scope == scope && entry.id == id)
                .ok_or(StatusCode::FORBIDDEN)?;
            let mut history = entry.input.clone();
            history.append(&mut input);
            input = history;
        }
        body.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?.remove("previous_response_id");
        body["input"] = input.clone().into();
        Ok(Self { store, scope, input })
    }
    pub(crate) fn remember(&self, response: &Value) -> Result<(), StatusCode> {
        let id = response["id"]
            .as_str()
            .filter(|id| !id.is_empty() && id.len() <= 1024)
            .ok_or(StatusCode::BAD_GATEWAY)?;
        if !matches!(response["status"].as_str(), Some("completed" | "incomplete")) {
            return Ok(());
        }
        let mut input = self.input.clone();
        input.extend(response["output"].as_array().ok_or(StatusCode::BAD_GATEWAY)?.iter().cloned());
        let bytes = serde_json::to_vec(&input).map_err(|_| StatusCode::BAD_GATEWAY)?.len();
        if bytes > MAX_BYTES {
            return Err(StatusCode::BAD_GATEWAY);
        }
        let mut entries = self.store.0.lock();
        entries.retain(|entry| {
            entry.born.elapsed() < TTL && !(entry.scope == self.scope && entry.id == id)
        });
        while entries.len() >= LIMIT
            || entries.iter().map(|e| e.bytes).sum::<usize>() + bytes > TOTAL_BYTES
        {
            entries.pop_front();
        }
        entries.push_back(Entry {
            scope: self.scope.clone(),
            id: id.into(),
            input,
            born: Instant::now(),
            bytes,
        });
        Ok(())
    }
    pub(crate) fn stream(self, body: Body) -> Body {
        Body::from_stream(futures_util::stream::unfold(
            (
                body.into_data_stream(),
                super::sse_framing::Lines::default(),
                String::new(),
                self,
                false,
            ),
            |(mut stream, mut lines, mut data, context, done)| async move {
                if done {
                    return None;
                }
                let bytes = match stream.next().await {
                    Some(Ok(bytes)) => bytes,
                    Some(Err(_)) => {
                        return Some((
                            Err(std::io::Error::other("Responses history stream failed")),
                            (stream, lines, data, context, true),
                        ));
                    }
                    None => return None,
                };
                let result = (|| {
                    for line in
                        lines.push(&bytes, MAX_BYTES).map_err(|_| StatusCode::BAD_GATEWAY)?
                    {
                        if line.is_empty() {
                            if !data.is_empty() && data.trim() != "[DONE]" {
                                let event: Value = serde_json::from_str(&data)
                                    .map_err(|_| StatusCode::BAD_GATEWAY)?;
                                if matches!(
                                    event["type"].as_str(),
                                    Some("response.completed" | "response.incomplete")
                                ) {
                                    context.remember(&event["response"])?;
                                }
                            }
                            data.clear();
                        } else if let Some(value) = line.strip_prefix("data:") {
                            if data.len() + value.len() + 1 > MAX_BYTES {
                                return Err(StatusCode::BAD_GATEWAY);
                            }
                            if !data.is_empty() {
                                data.push('\n');
                            }
                            data.push_str(value.trim_start_matches(' '));
                        }
                    }
                    Ok::<_, StatusCode>(())
                })();
                let done = result.is_err();
                Some((
                    result
                        .map(|()| bytes)
                        .map_err(|_| std::io::Error::other("Invalid Responses history")),
                    (stream, lines, data, context, done),
                ))
            },
        ))
    }
}

/// Keep malformed historical calls and their results as context, never executable calls.
pub(super) fn preserve_invalid(items: &mut Vec<Value>) {
    let mut owners = std::collections::HashMap::<String, usize>::new();
    let mut output: Vec<Value> = Vec::new();
    for mut item in std::mem::take(items) {
        if item["type"] == "function_call" {
            let id = item["call_id"].as_str().unwrap_or("").to_owned();
            owners.remove(&id);
            let valid = item["arguments"]
                .as_str()
                .is_some_and(|s| serde_json::from_str::<Value>(s).is_ok_and(|v| v.is_object()));
            if !valid {
                if !id.is_empty() {
                    owners.insert(id, output.len());
                }
                item = json!({"role":"assistant","content":format!("[Historical tool call with invalid arguments; context only, not a new tool invocation]\n{}", item)});
            }
        } else if item["type"] == "function_call_output" {
            if let Some(owner) = item["call_id"].as_str().and_then(|id| owners.get(id)) {
                let content = output[*owner]["content"].as_str().unwrap_or("");
                output[*owner]["content"] = format!(
                    "{content}\n[Historical tool result for invalid call; context only]\n{item}"
                )
                .into();
                continue;
            }
        }
        output.push(item);
    }
    *items = output;
}

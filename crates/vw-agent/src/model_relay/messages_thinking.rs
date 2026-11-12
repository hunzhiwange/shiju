//! Chat reasoning uses bridge receipts, never fabricated vendor signatures.
use super::{RelayIdentity, messages};
use crate::app_routing::reference_scope::{ReferenceScope, References};
use axum::{body::Body, http::StatusCode, response::Response};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(super) struct Context {
    identity: RelayIdentity,
    deployment: String,
    references: Arc<References>,
}
impl Context {
    pub(super) fn new(
        identity: &RelayIdentity,
        deployment: String,
        references: Arc<References>,
    ) -> Self {
        Self { identity: identity.clone(), deployment, references }
    }
    fn digest(&self, message: &Value) -> String {
        let mut normalized = json!([self.deployment, message]);
        normalized.sort_all_objects();
        let bytes = normalized.to_string();
        hex::encode(ring::digest::digest(&ring::digest::SHA256, bytes.as_bytes()))
    }
    pub(super) fn request(&self, input: &Value) -> Result<Value, StatusCode> {
        let explicitly_thinking =
            matches!(input["thinking"]["type"].as_str(), Some("enabled" | "adaptive"));
        let mut input = input.clone();
        if let Some(config) = input.get_mut("output_config") {
            let config = config.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
            if let Some(effort) = config.remove("effort")
                && !effort.is_string()
            {
                return Err(StatusCode::BAD_REQUEST);
            }
            if config.is_empty() {
                input.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?.remove("output_config");
            }
        }
        let history = input["messages"].as_array_mut().ok_or(StatusCode::BAD_REQUEST)?;
        for message in history.iter().rev() {
            let blocks = message["content"].as_array().into_iter().flatten();
            if message["role"] == "user" && !blocks.clone().any(|b| b["type"] == "tool_result") {
                break;
            }
            if message["role"] == "assistant"
                && blocks.clone().any(|b| b["type"] == "tool_use")
                && !blocks.clone().any(|b| b["type"] == "thinking")
                && (explicitly_thinking
                    || blocks.clone().any(|block| {
                        block["type"] == "tool_use"
                            && block["id"].as_str().is_some_and(|id| {
                                self.references
                                    .check_item(
                                        &ReferenceScope::Session(self.identity.clone()),
                                        &format!("shiju-thinking-tool:{id}"),
                                    )
                                    .is_ok()
                            })
                    }))
            {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
        }
        let mut reasoning = Vec::new();
        let mut pending_tools = std::collections::HashSet::new();
        for message in history {
            let Some(blocks) = message["content"].as_array() else {
                reasoning.push(None);
                continue;
            };
            for block in blocks {
                if message["role"] == "assistant" && block["type"] == "tool_use" {
                    let id = block["id"]
                        .as_str()
                        .filter(|id| !id.is_empty())
                        .ok_or(StatusCode::BAD_REQUEST)?;
                    if !pending_tools.insert(id.to_owned()) {
                        return Err(StatusCode::BAD_REQUEST);
                    }
                }
                if message["role"] == "user" && block["type"] == "tool_result" {
                    let id = block["tool_use_id"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
                    if !pending_tools.remove(id) {
                        return Err(StatusCode::UNPROCESSABLE_ENTITY);
                    }
                }
            }
            let thoughts: Vec<_> = blocks.iter().filter(|b| b["type"] == "thinking").collect();
            if thoughts.is_empty() {
                reasoning.push(None);
                continue;
            }
            if message["role"] != "assistant"
                || thoughts.len() != 1
                || blocks[0]["type"] != "thinking"
            {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            messages::object_keys(thoughts[0], &["type", "thinking", "signature"])?;
            let receipt = thoughts[0]["signature"].as_str().ok_or(StatusCode::FORBIDDEN)?;
            if !receipt.starts_with("shiju-bridge-v1:") {
                return Err(StatusCode::FORBIDDEN);
            }
            let saved = self
                .references
                .resolve(&ReferenceScope::Session(self.identity.clone()), receipt)?;
            let (deadline, expected) = saved.split_once(':').ok_or(StatusCode::FORBIDDEN)?;
            if deadline.parse::<i64>().map_err(|_| StatusCode::FORBIDDEN)?
                < chrono::Utc::now().timestamp()
            {
                return Err(StatusCode::FORBIDDEN);
            }
            let thought =
                thoughts[0]["thinking"].as_str().ok_or(StatusCode::BAD_REQUEST)?.to_owned();
            let mut normalized = message.clone();
            normalized["content"][0]
                .as_object_mut()
                .ok_or(StatusCode::BAD_REQUEST)?
                .remove("signature");
            if self.digest(&normalized) != expected {
                return Err(StatusCode::FORBIDDEN);
            }
            message["content"].as_array_mut().ok_or(StatusCode::BAD_REQUEST)?.remove(0);
            reasoning.push(Some(thought));
        }
        let mut converted = messages::to_chat_thinking(&input)?;
        // Rebuild per-message association; tool_result may expand to several Chat messages.
        let offset = usize::from(input.get("system").is_some());
        let mut cursor = offset;
        for (message, thought) in
            input["messages"].as_array().ok_or(StatusCode::BAD_REQUEST)?.iter().zip(reasoning)
        {
            let count = if message["role"] == "assistant" {
                1
            } else {
                let one = json!({"model":input["model"],"max_tokens":input["max_tokens"],"messages":[message]});
                messages::to_chat(&one)?["messages"]
                    .as_array()
                    .ok_or(StatusCode::BAD_REQUEST)?
                    .len()
            };
            if let Some(thought) = thought {
                converted["messages"][cursor]["reasoning_content"] = thought.into();
            }
            cursor += count;
        }
        Ok(converted)
    }
    pub(super) fn result(&self, body: &Value, model: &str) -> Result<Value, StatusCode> {
        let mut body = body.clone();
        if body["choices"].as_array().is_none_or(|choices| choices.len() != 1)
            || !body["choices"][0]["message"].is_object()
        {
            return Err(StatusCode::BAD_GATEWAY);
        }
        let message = &mut body["choices"][0]["message"];
        let thought = message
            .get("reasoning_content")
            .filter(|v| !v.is_null())
            .map(|v| v.as_str().ok_or(StatusCode::BAD_GATEWAY))
            .transpose()?
            .unwrap_or("")
            .to_owned();
        message.as_object_mut().ok_or(StatusCode::BAD_GATEWAY)?.remove("reasoning_content");
        let mut result = messages::from_chat(&body, model)?;
        let mut tools = std::collections::HashSet::new();
        for block in result["content"].as_array().ok_or(StatusCode::BAD_GATEWAY)? {
            if block["type"] == "tool_use" {
                let id = block["id"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .ok_or(StatusCode::BAD_GATEWAY)?;
                if !tools.insert(id.to_owned()) {
                    return Err(StatusCode::BAD_GATEWAY);
                }
            }
        }
        if !thought.is_empty() {
            result["content"]
                .as_array_mut()
                .ok_or(StatusCode::BAD_GATEWAY)?
                .insert(0, json!({"type":"thinking","thinking":thought}));
            let normalized = json!({"role":"assistant","content":result["content"]});
            let digest = self.digest(&normalized);
            let mut nonce = [0; 32];
            getrandom::fill(&mut nonce).map_err(|_| StatusCode::BAD_GATEWAY)?;
            let receipt = format!("shiju-bridge-v1:{}", hex::encode(nonce));
            self.references.remember(
                &ReferenceScope::Session(self.identity.clone()),
                &receipt,
                &format!("{}:{digest}", chrono::Utc::now().timestamp() + 900),
            )?;
            for id in tools {
                self.references.remember_item(
                    &ReferenceScope::Session(self.identity.clone()),
                    &format!("shiju-thinking-tool:{id}"),
                )?;
            }
            result["content"][0]["signature"] = receipt.into();
        }
        Ok(result)
    }
}

pub(super) use super::messages_thinking_config::configure;
pub(crate) use super::messages_thinking_config::supports;

pub(super) async fn response(
    response: reqwest::Response,
    context: Context,
    model: &str,
    streaming: bool,
    cancel: CancellationToken,
) -> Result<Response, StatusCode> {
    if streaming {
        let model = model.to_owned();
        // Dropping the client body drops this future and the upstream response.
        let stream = futures_util::stream::once(async move {
            let result = async {
                let body =
                    super::messages_stream::collect_thinking(response, &model, cancel.clone())
                        .await?;
                encode_events(&context.result(&body, &model)?)
            }
            .await;
            let output = match result {
                Ok(output) => output,
                Err(StatusCode::REQUEST_TIMEOUT) => String::new(),
                Err(_) => "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"api_error\",\"message\":\"Invalid upstream thinking stream\"}}\n\n".into(),
            };
            Ok::<_, std::io::Error>(axum::body::Bytes::from(output))
        });
        return Response::builder()
            .header("content-type", "text/event-stream")
            .body(Body::from_stream(stream))
            .map_err(|_| StatusCode::BAD_GATEWAY);
    }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    loop {
        let next = tokio::select! { biased; () = cancel.cancelled() => return Err(StatusCode::REQUEST_TIMEOUT), next = stream.next() => next };
        let Some(chunk) = next else { break };
        let chunk = chunk.map_err(|_| StatusCode::BAD_GATEWAY)?;
        if bytes.len() + chunk.len() > 4 * 1024 * 1024 {
            return Err(StatusCode::BAD_GATEWAY);
        }
        bytes.extend_from_slice(&chunk);
    }
    let result = context
        .result(&serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_GATEWAY)?, model)?;
    Response::builder()
        .header("content-type", "application/json")
        .body(Body::from(result.to_string()))
        .map_err(|_| StatusCode::BAD_GATEWAY)
}

fn encode_events(result: &Value) -> Result<String, StatusCode> {
    // Thinking is buffered until its full scoped receipt can authenticate the tool round.
    let event = |value: Value| {
        format!("event: {}\ndata: {value}\n\n", value["type"].as_str().unwrap_or("error"))
    };
    let mut start = result.clone();
    start["content"] = json!([]);
    start["stop_reason"] = Value::Null;
    start["usage"]["output_tokens"] = 0.into();
    let mut output = event(json!({"type":"message_start","message":start}));
    for (index, block) in
        result["content"].as_array().ok_or(StatusCode::BAD_GATEWAY)?.iter().enumerate()
    {
        let (empty, deltas) = match block["type"].as_str() {
            Some("thinking") => (
                json!({"type":"thinking","thinking":""}),
                vec![
                    json!({"type":"thinking_delta","thinking":block["thinking"]}),
                    json!({"type":"signature_delta","signature":block["signature"]}),
                ],
            ),
            Some("text") => (
                json!({"type":"text","text":""}),
                vec![json!({"type":"text_delta","text":block["text"]})],
            ),
            Some("tool_use") => (
                json!({"type":"tool_use","id":block["id"],"name":block["name"],"input":{}}),
                vec![json!({"type":"input_json_delta","partial_json":block["input"].to_string()})],
            ),
            _ => return Err(StatusCode::BAD_GATEWAY),
        };
        output.push_str(&event(
            json!({"type":"content_block_start","index":index,"content_block":empty}),
        ));
        for delta in deltas {
            output.push_str(&event(
                json!({"type":"content_block_delta","index":index,"delta":delta}),
            ));
        }
        output.push_str(&event(json!({"type":"content_block_stop","index":index})));
    }
    output.push_str(&event(json!({"type":"message_delta","delta":{"stop_reason":result["stop_reason"],"stop_sequence":null},"usage":result["usage"]})));
    output.push_str(&event(json!({"type":"message_stop"})));
    Ok(output)
}

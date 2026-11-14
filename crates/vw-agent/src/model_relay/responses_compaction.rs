//! Codex Responses compaction through the existing Chat conversion.

use super::responses::ChatTools;
use axum::{body::Body, http::StatusCode};
use serde_json::{Map, Value, json};
use tokio_util::sync::CancellationToken;

const PROMPT: &str = "You are performing a CONTEXT CHECKPOINT COMPACTION. Create a handoff summary for another LLM that will resume the task.

Include:
- Current progress and key decisions made
- Important context, constraints, or user preferences
- What remains to be done (clear next steps)
- Any critical data, examples, or references needed to continue

Be concise, structured, and focused on helping the next LLM seamlessly continue the work.
";

pub(crate) struct Request {
    pub(crate) chat: Value,
    model: String,
    tools: ChatTools,
    compact: bool,
}

impl Request {
    pub(crate) fn from_responses(mut body: Value) -> Result<Self, StatusCode> {
        if body.get("stream").is_some_and(|v| !v.is_boolean()) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let model = body["model"]
            .as_str()
            .filter(|name| !name.trim().is_empty())
            .ok_or(StatusCode::BAD_REQUEST)?
            .to_owned();
        let mut empty = Vec::new();
        let items = body["input"].as_array_mut().unwrap_or(&mut empty);
        let mut triggers = 0;
        for item in items {
            match item["type"].as_str() {
                Some("compaction_trigger") => {
                    super::messages::object_keys(item, &["type"])?;
                    triggers += 1;
                    *item = json!({"type":"message","role":"user","content":PROMPT});
                }
                _ => {}
            }
        }
        if triggers > 1 {
            return Err(StatusCode::BAD_REQUEST);
        }
        let compact = triggers == 1;
        if compact && body["stream"] != true {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        if compact {
            let body = body.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
            body.remove("text");
            body.remove("parallel_tool_calls");
            body.insert("tool_choice".into(), json!("none"));
        }
        let (chat, tools) = super::responses_chat_request::convert(body)?;
        Ok(Self { chat, model, tools, compact })
    }

    /// The bridge owns its wire shape even when the provider has request overrides.
    pub(crate) fn apply_wire_controls(&self, body: &mut Map<String, Value>) {
        for field in [
            "stream",
            "stream_options",
            "model",
            "messages",
            "tools",
            "tool_choice",
            "parallel_tool_calls",
            "reasoning_effort",
        ] {
            if let Some(value) = self.chat.get(field) {
                body.insert(field.into(), value.clone());
            } else {
                body.remove(field);
            }
        }
        for field in ["n", "functions", "function_call"] {
            body.remove(field);
        }
        if self.compact {
            body.remove("response_format");
        }
    }

    pub(crate) fn response_json(&self, body: &Value) -> Result<Value, StatusCode> {
        super::responses_chat_result::from_chat(
            body,
            &self.model,
            &super::responses::response_id(),
            &self.tools,
        )
    }

    pub(crate) fn response_body(self, body: Body, cancel: CancellationToken) -> Body {
        if self.compact {
            super::responses_stream::codex_chat_body(body, self.model, self.tools, true, cancel)
        } else {
            super::responses_stream::resident_chat_body(body, self.model, self.tools, cancel)
        }
    }
}

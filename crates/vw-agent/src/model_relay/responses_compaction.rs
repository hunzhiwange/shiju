//! Resident Responses requests using shared Chat/Anthropic encoders and scoped history.

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
    anthropic: bool,
    history: Option<super::responses_history::Context>,
}

impl Request {
    pub(crate) fn from_responses(body: Value) -> Result<Self, StatusCode> {
        Self::from_responses_inner(body, false)
    }

    pub(crate) fn from_responses_anthropic(body: Value) -> Result<Self, StatusCode> {
        // Native Anthropic SSE adaptation is delivered separately.
        if body["stream"] == true {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        Self::from_responses_inner(body, true)
    }

    fn from_responses_inner(mut body: Value, anthropic: bool) -> Result<Self, StatusCode> {
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
        let (chat, tools) = if anthropic {
            super::responses_anthropic_request::convert(body, 4096)?
        } else {
            super::responses_chat_request::convert(body)?
        };
        Ok(Self { chat, model, tools, compact, anthropic, history: None })
    }

    pub(crate) fn with_history(mut self, history: super::responses_history::Context) -> Self {
        self.history = Some(history);
        self
    }

    /// The bridge owns its wire shape even when the provider has request overrides.
    pub(crate) fn apply_wire_controls(&self, body: &mut Map<String, Value>) {
        if self.anthropic {
            // Provider overrides cannot reintroduce a Responses/Chat wire shape.
            *body = self.chat.as_object().expect("converted Messages object").clone();
            return;
        }
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
        let chat;
        let body = if self.anthropic {
            chat = super::protocol_bridge::response(
                vw_shared::provider::ModelProtocol::Messages,
                body,
            )?;
            &chat
        } else {
            body
        };
        let response = super::responses_chat_result::from_chat(
            body,
            &self.model,
            &super::responses::response_id(),
            &self.tools,
        )?;
        if let Some(history) = &self.history {
            history.remember(&response)?;
        }
        Ok(response)
    }

    pub(crate) fn response_body(self, body: Body, cancel: CancellationToken) -> Body {
        let history = self.history;
        let body = if self.compact {
            super::responses_stream::codex_chat_body(body, self.model, self.tools, true, cancel)
        } else {
            super::responses_stream::resident_chat_body(body, self.model, self.tools, cancel)
        };
        match history {
            Some(history) => history.stream(body),
            None => body,
        }
    }
}

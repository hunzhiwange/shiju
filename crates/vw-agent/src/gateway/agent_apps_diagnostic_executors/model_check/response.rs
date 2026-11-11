//! Fixed request and bounded public evidence for a model connectivity probe.

use serde_json::{Value, json};
use vw_api_types::{agent_apps_diagnostics::DiagnosticFailureCode, id::ModelId};

use crate::model_relay::generation::GenerationProtocol;

pub(super) trait GenerationEvidence {
    fn prompt(self, model: &str) -> Value;
    fn actual_model(self, response: &Value) -> Result<ModelId, DiagnosticFailureCode>;
}

impl GenerationEvidence for GenerationProtocol {
    fn prompt(self, model: &str) -> Value {
        const PROMPT: &str = "Reply with OK.";
        const LIMIT: u32 = 32;
        match self {
            Self::Chat => json!({
                "model": model, "messages": [{"role": "user", "content": PROMPT}],
                "max_tokens": LIMIT, "stream": false,
            }),
            Self::Messages => json!({
                "model": model, "messages": [{"role": "user", "content": PROMPT}],
                "max_tokens": LIMIT, "stream": false,
            }),
            Self::Responses => json!({
                "model": model, "input": PROMPT, "max_output_tokens": LIMIT,
                "stream": false, "store": false,
            }),
            Self::Gemini => json!({
                "contents": [{"role": "user", "parts": [{"text": PROMPT}]}],
                "generationConfig": {"maxOutputTokens": LIMIT},
            }),
        }
    }

    fn actual_model(self, response: &Value) -> Result<ModelId, DiagnosticFailureCode> {
        let invalid = DiagnosticFailureCode::InvalidResponse;
        if response.get("error").is_some_and(|error| !error.is_null()) {
            return Err(invalid);
        }
        let text = |value: &Value| value.as_str().is_some_and(|text| !text.trim().is_empty());
        let any = |items: &Value, predicate: &dyn Fn(&Value) -> bool| {
            items.as_array().is_some_and(|items| items.iter().any(predicate))
        };
        let valid = match self {
            Self::Chat => {
                response["object"] == "chat.completion"
                    && any(&response["choices"], &|choice| {
                        choice["message"]["role"] == "assistant"
                            && text(&choice["message"]["content"])
                    })
            }
            Self::Messages => {
                response["type"] == "message"
                    && response["role"] == "assistant"
                    && any(&response["content"], &|block| {
                        block["type"] == "text" && text(&block["text"])
                    })
            }
            Self::Responses => {
                response["object"] == "response"
                    && matches!(response["status"].as_str(), Some("completed" | "incomplete"))
                    && any(&response["output"], &|message| {
                        message["type"] == "message"
                            && message["role"] == "assistant"
                            && any(&message["content"], &|block| {
                                block["type"] == "output_text" && text(&block["text"])
                            })
                    })
            }
            Self::Gemini => any(&response["candidates"], &|candidate| {
                candidate["content"]["role"] == "model"
                    && any(&candidate["content"]["parts"], &|part| {
                        part["thought"] != true && text(&part["text"])
                    })
            }),
        };
        if !valid {
            return Err(invalid);
        }
        let model_field = if self == Self::Gemini { "modelVersion" } else { "model" };
        let model_id = response[model_field].as_str().map(ModelId::from).ok_or(invalid)?;
        vw_agent_apps::models::edits::validate_model_id(&model_id).map_err(|_| invalid)?;
        Ok(model_id)
    }
}

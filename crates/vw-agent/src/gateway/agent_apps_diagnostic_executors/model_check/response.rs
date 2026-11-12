//! Fixed request and bounded public evidence for a model connectivity probe.

use serde_json::{Value, json};
use vw_api_types::{
    agent_apps_diagnostics::{DiagnosticFailureCode, DiagnosticModelProtocol},
    id::ModelId,
};

use crate::model_relay::generation::GenerationProtocol;

pub(in crate::gateway::agent_apps_diagnostic_executors) trait GenerationEvidence {
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
                            && choice["finish_reason"] == "stop"
                            && text(&choice["message"]["content"])
                    })
            }
            Self::Messages => {
                response["type"] == "message"
                    && response["role"] == "assistant"
                    && response["stop_reason"] == "end_turn"
                    && any(&response["content"], &|block| {
                        block["type"] == "text" && text(&block["text"])
                    })
            }
            Self::Responses => {
                response["object"] == "response"
                    && response["status"] == "completed"
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
                    && candidate["finishReason"] == "STOP"
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

pub(super) fn internal_protocol(value: DiagnosticModelProtocol) -> GenerationProtocol {
    match value {
        DiagnosticModelProtocol::Chat => GenerationProtocol::Chat,
        DiagnosticModelProtocol::Messages => GenerationProtocol::Messages,
        DiagnosticModelProtocol::Responses => GenerationProtocol::Responses,
        DiagnosticModelProtocol::Gemini => GenerationProtocol::Gemini,
    }
}

pub(super) fn public_protocol(protocol: GenerationProtocol) -> DiagnosticModelProtocol {
    match protocol {
        GenerationProtocol::Chat => DiagnosticModelProtocol::Chat,
        GenerationProtocol::Messages => DiagnosticModelProtocol::Messages,
        GenerationProtocol::Responses => DiagnosticModelProtocol::Responses,
        GenerationProtocol::Gemini => DiagnosticModelProtocol::Gemini,
    }
}

/// Use the same converters as the relay, restricted to the fixed text probe.
pub(super) fn request(
    source: GenerationProtocol,
    target: GenerationProtocol,
    model: &str,
) -> Result<Value, DiagnosticFailureCode> {
    let input = source.prompt(model);
    if source == target {
        return Ok(input);
    }
    if target != GenerationProtocol::Chat {
        return Err(DiagnosticFailureCode::Unsupported);
    }
    let mut body = crate::model_relay::generation::text_probe_request(source, target, &input)
        .map_err(|_| DiagnosticFailureCode::Unsupported)?;
    body["model"] = model.into();
    Ok(body)
}

pub(super) fn actual_model(
    source: GenerationProtocol,
    target: GenerationProtocol,
    body: &Value,
) -> Result<ModelId, DiagnosticFailureCode> {
    let actual = target.actual_model(body)?;
    if source == target {
        return Ok(actual);
    }
    let converted =
        crate::model_relay::generation::text_probe_response(source, target, body, actual.as_ref())
            .map_err(|_| DiagnosticFailureCode::InvalidResponse)?;
    source.actual_model(&converted)
}

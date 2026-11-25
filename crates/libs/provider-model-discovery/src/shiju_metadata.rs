//! Normalize the metadata returned with the account's Shiju model IDs.

use model_types::{CapabilityIO, Model};
use serde_json::Value;

use crate::http::ModelEntry;

pub(super) fn apply(model: &mut Model, entry: &ModelEntry, configured: Option<&Value>) -> bool {
    let metadata = &entry.metadata;
    let modalities = metadata.get("modalities");
    let capabilities = metadata.get("capabilities");
    let declared = |side| {
        let field = match side {
            "input" => "input_modalities",
            _ => "output_modalities",
        };
        declared_io(modalities.and_then(|m| m.get(side)))
            .or_else(|| declared_io(metadata.get(field)))
    };
    let supports = |key: &str| {
        let prefixed = format!("supports_{key}");
        let get = |field: &str| match capabilities {
            Some(caps) => caps.get(field),
            None => metadata.get(field),
        };
        get(&prefixed).or_else(|| get(key)).and_then(Value::as_bool)
    };
    let endpoints = entry.supported_endpoint_types.as_ref().and_then(Value::as_array);
    let has_endpoint = |names: &[&str]| {
        endpoints.is_some_and(|endpoints| {
            endpoints.iter().any(|endpoint| endpoint.as_str().is_some_and(|s| names.contains(&s)))
        })
    };
    let transcription =
        supports("transcription").unwrap_or_else(|| has_endpoint(&["audio-transcription"]));
    let dedicated_only = endpoints.is_some_and(|endpoints| {
        !endpoints.is_empty() && endpoints.iter().all(is_dedicated_endpoint)
    });
    let modalities_config = configured.and_then(|model| model.get("modalities"));
    let mut changed = false;

    if !modalities_config.and_then(|m| m.get("input")).is_some_and(Value::is_array) {
        let input = &mut model.capabilities.input;
        let old = io_flags(input);
        if let Some(declared) = declared("input") {
            *input = declared;
        } else {
            input.image = supports("image").unwrap_or(input.image);
            input.video = supports("video").unwrap_or(input.video);
            input.audio = supports("audio").unwrap_or(input.audio || transcription);
        }
        if old != io_flags(input) {
            changed = true;
            if !configured.and_then(|m| m.get("attachment")).is_some_and(Value::is_boolean) {
                model.capabilities.attachment =
                    input.image || input.video || input.audio || input.pdf;
            }
        }
    }

    if !modalities_config.and_then(|m| m.get("output")).is_some_and(Value::is_array) {
        let output = &mut model.capabilities.output;
        let old = io_flags(output);
        if let Some(declared) = declared("output") {
            *output = declared;
        } else {
            output.image = supports("image_generation")
                .unwrap_or(output.image || has_endpoint(&["image-generation", "image-edit"]));
            output.video = supports("video_generation")
                .unwrap_or(output.video || has_endpoint(&["openai-video", "video-generation"]));
            output.audio =
                supports("speech").unwrap_or(output.audio || has_endpoint(&["audio-speech"]));
            if transcription {
                output.text = true;
            } else if dedicated_only {
                output.text = false;
            }
        }
        changed |= old != io_flags(output);
    }

    if !configured.and_then(|m| m.get("tool_call")).is_some_and(Value::is_boolean)
        && let Some(tools) = supports("tool_calling").or(dedicated_only.then_some(false))
    {
        changed |= model.capabilities.toolcall != tools;
        model.capabilities.toolcall = tools;
    }

    let limit = metadata.get("limit");
    let number = |key: &str, aliases: &[&str]| {
        limit.and_then(|l| l.get(key)).and_then(Value::as_u64).filter(|&v| v > 0).or_else(|| {
            aliases.iter().find_map(|&key| metadata.get(key)?.as_u64().filter(|&v| v > 0))
        })
    };
    let configured_limit = configured.and_then(|m| m.get("limit"));
    let explicit = |key: &str| configured_limit.and_then(|l| l.get(key)).and_then(Value::as_u64);
    let current = &mut model.limit;
    let old = (current.context, current.input, current.output);
    let context = number(
        "context",
        &["context_length", "max_model_len", "max_context_length", "context_window"],
    )
    .filter(|_| explicit("context").unwrap_or_default() == 0);
    if let Some(context) = context {
        current.context = context;
    }
    if explicit("input").unwrap_or_default() == 0 {
        current.input = number("input", &["max_input_tokens"]).or(current.input);
        if context.is_some() {
            current.input = current.input.map(|input| input.min(current.context));
        }
    }
    if explicit("output").is_none() {
        current.output = number("output", &["max_output_tokens"]).unwrap_or(current.output);
        if context.is_some() {
            current.output = current.output.min(current.context);
        }
    }
    changed || old != (current.context, current.input, current.output)
}

fn declared_io(value: Option<&Value>) -> Option<CapabilityIO> {
    let values = value?.as_array()?;
    if !values.iter().all(Value::is_string) {
        return None;
    }
    let has = |kind| values.iter().any(|value| value.as_str() == Some(kind));
    Some(CapabilityIO {
        text: has("text"),
        audio: has("audio"),
        image: has("image"),
        video: has("video"),
        pdf: has("pdf"),
    })
}

fn io_flags(io: &CapabilityIO) -> [bool; 5] {
    [io.text, io.audio, io.image, io.video, io.pdf]
}

fn is_dedicated_endpoint(endpoint: &Value) -> bool {
    matches!(
        endpoint.as_str(),
        Some(
            "image-generation"
                | "image-edit"
                | "openai-video"
                | "video-generation"
                | "audio-speech"
                | "audio-transcription"
                | "embedding"
                | "embeddings"
                | "rerank"
        )
    )
}

#[cfg(test)]
#[path = "shiju_metadata_tests.rs"]
mod tests;

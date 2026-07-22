//! Prompt 内容块的解析、校验与归一化。

use agent_client_protocol::{ContentBlock, ImageContent};
use serde_json::{Map, Value, json};

pub type PromptInput = Vec<ContentBlock>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct PromptInputValidationError {
    message: String,
}

impl PromptInputValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptContentBlockKind {
    Text,
    Image,
    ResourceLink,
    Resource,
}

fn is_non_empty_string(value: Option<&Value>) -> bool {
    value.and_then(Value::as_str).is_some_and(|value| !value.trim().is_empty())
}

fn is_base64_data(value: &str) -> bool {
    if value.is_empty() || !value.len().is_multiple_of(4) {
        return false;
    }

    let padding_start = value.find('=').unwrap_or(value.len());
    if value[padding_start..].chars().any(|ch| ch != '=') {
        return false;
    }

    let padding_len = value.len() - padding_start;
    if padding_len > 2 {
        return false;
    }

    value[..padding_start].chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '/'))
}

fn is_image_mime_type(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("image/").or_else(|| {
        let lower = value.to_ascii_lowercase();
        lower.strip_prefix("image/").map(|_| &value[6..])
    }) else {
        return false;
    };

    !rest.is_empty()
        && rest.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '+' | '-'))
}

fn is_resource_payload(value: Option<&Value>) -> bool {
    let Some(record) = value.and_then(Value::as_object) else {
        return false;
    };

    if !is_non_empty_string(record.get("uri")) {
        return false;
    }

    record.get("text").is_none_or(Value::is_string)
}

fn validate_content_block_value(
    value: &Value,
    index: usize,
) -> Result<(&Map<String, Value>, PromptContentBlockKind), String> {
    let Some(record) = value.as_object() else {
        return Err(format!("prompt[{index}] must be an ACP content block object"));
    };

    let Some(block_type) = record.get("type").and_then(Value::as_str) else {
        return Err(format!("prompt[{index}] must be an ACP content block object"));
    };

    let kind = match block_type {
        "text" => {
            if !record.get("text").is_some_and(Value::is_string) {
                return Err(format!("prompt[{index}] text block must include a string text field"));
            }
            PromptContentBlockKind::Text
        }
        "image" => {
            if !is_non_empty_string(record.get("mimeType")) {
                return Err(format!(
                    "prompt[{index}] image block must include a non-empty mimeType"
                ));
            }

            let mime_type = record.get("mimeType").and_then(Value::as_str).unwrap_or_default();
            if !is_image_mime_type(mime_type) {
                return Err(format!("prompt[{index}] image block mimeType must start with image/"));
            }

            let Some(data) = record.get("data").and_then(Value::as_str) else {
                return Err(format!(
                    "prompt[{index}] image block must include non-empty base64 data"
                ));
            };
            if data.is_empty() {
                return Err(format!(
                    "prompt[{index}] image block must include non-empty base64 data"
                ));
            }
            if !is_base64_data(data) {
                return Err(format!("prompt[{index}] image block data must be valid base64"));
            }

            PromptContentBlockKind::Image
        }
        "resource_link" => {
            if !is_non_empty_string(record.get("uri")) {
                return Err(format!(
                    "prompt[{index}] resource_link block must include a non-empty uri"
                ));
            }
            if !record.get("title").is_none_or(Value::is_string) {
                return Err(format!(
                    "prompt[{index}] resource_link block title must be a string when present"
                ));
            }
            if !record.get("name").is_none_or(Value::is_string) {
                return Err(format!(
                    "prompt[{index}] resource_link block name must be a string when present"
                ));
            }
            PromptContentBlockKind::ResourceLink
        }
        "resource" => {
            if !record.get("resource").is_some_and(Value::is_object) {
                return Err(format!(
                    "prompt[{index}] resource block must include a resource object"
                ));
            }
            if !is_resource_payload(record.get("resource")) {
                return Err(format!(
                    "prompt[{index}] resource block resource must include a non-empty uri and optional text"
                ));
            }
            PromptContentBlockKind::Resource
        }
        _ => {
            return Err(format!(
                "prompt[{index}] has unsupported content block type {block_type:?}"
            ));
        }
    };

    Ok((record, kind))
}

fn get_content_block_validation_error(value: &Value, index: usize) -> Option<String> {
    validate_content_block_value(value, index).err()
}

fn content_block_from_value(
    value: &Value,
    index: usize,
) -> Result<ContentBlock, PromptInputValidationError> {
    let (record, block_type) =
        validate_content_block_value(value, index).map_err(PromptInputValidationError::new)?;

    match block_type {
        PromptContentBlockKind::Text => Ok(record
            .get("text")
            .and_then(Value::as_str)
            .expect("validation guarantees text block text is a string")
            .to_string()
            .into()),
        PromptContentBlockKind::Image => Ok(ContentBlock::Image(ImageContent::new(
            record.get("data").and_then(Value::as_str).expect("validation guarantees image data"),
            record
                .get("mimeType")
                .and_then(Value::as_str)
                .expect("validation guarantees image mimeType"),
        ))),
        PromptContentBlockKind::ResourceLink => {
            let mut block = Map::new();
            block.insert("type".to_string(), Value::String("resource_link".to_string()));
            block.insert(
                "uri".to_string(),
                Value::String(
                    record
                        .get("uri")
                        .and_then(Value::as_str)
                        .expect("validation guarantees resource_link uri")
                        .to_string(),
                ),
            );

            if let Some(title) = record.get("title").and_then(Value::as_str) {
                block.insert("title".to_string(), Value::String(title.to_string()));
            }
            if let Some(name) = record.get("name").and_then(Value::as_str) {
                block.insert("name".to_string(), Value::String(name.to_string()));
            }

            serde_json::from_value(Value::Object(block)).map_err(|error| {
                PromptInputValidationError::new(format!(
                    "prompt[{index}] could not be converted to ACP content block: {error}"
                ))
            })
        }
        PromptContentBlockKind::Resource => {
            let resource_record = record
                .get("resource")
                .and_then(Value::as_object)
                .expect("validation guarantees resource block has a resource object");
            let mut resource = Map::new();
            resource.insert(
                "uri".to_string(),
                Value::String(
                    resource_record
                        .get("uri")
                        .and_then(Value::as_str)
                        .expect("validation guarantees resource uri")
                        .to_string(),
                ),
            );
            if let Some(text) = resource_record.get("text").and_then(Value::as_str) {
                resource.insert("text".to_string(), Value::String(text.to_string()));
            }

            let block = json!({
                "type": "resource",
                "resource": Value::Object(resource),
            });
            serde_json::from_value(block).map_err(|error| {
                PromptInputValidationError::new(format!(
                    "prompt[{index}] could not be converted to ACP content block: {error}"
                ))
            })
        }
    }
}

fn parse_prompt_input_value(value: &Value) -> Result<PromptInput, PromptInputValidationError> {
    let blocks = value.as_array().ok_or_else(|| {
        PromptInputValidationError::new(
            "Structured prompt JSON must be an array of valid ACP content blocks",
        )
    })?;

    blocks.iter().enumerate().map(|(index, value)| content_block_from_value(value, index)).collect()
}

fn parse_structured_prompt(
    source: &str,
) -> Result<Option<PromptInput>, PromptInputValidationError> {
    if !source.starts_with('[') {
        return Ok(None);
    }

    let parsed = match serde_json::from_str::<Value>(source) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };

    parse_prompt_input_value(&parsed).map(Some)
}

fn display_text_from_content_block(block: &ContentBlock) -> String {
    match block {
        ContentBlock::Text(content) => content.text.clone(),
        ContentBlock::ResourceLink(resource) => resource
            .title
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .or(Some(resource.name.as_str()))
            .filter(|value| !value.trim().is_empty())
            .or(Some(resource.uri.as_str()))
            .unwrap_or_default()
            .to_string(),
        ContentBlock::Resource(resource) => {
            let value = serde_json::to_value(&resource.resource).unwrap_or_default();
            value
                .get("text")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .or_else(|| value.get("uri").and_then(Value::as_str))
                .unwrap_or_default()
                .to_string()
        }
        ContentBlock::Image(image) => format!("[image] {}", image.mime_type),
        _ => String::new(),
    }
}

pub fn is_prompt_input(value: &Value) -> bool {
    value.as_array().is_some_and(|entries| {
        entries
            .iter()
            .enumerate()
            .all(|(index, entry)| get_content_block_validation_error(entry, index).is_none())
    })
}

pub fn text_prompt(text: impl Into<String>) -> PromptInput {
    vec![text.into().into()]
}

pub fn parse_prompt_source(source: &str) -> Result<PromptInput, PromptInputValidationError> {
    let trimmed = source.trim();
    if let Some(structured) = parse_structured_prompt(trimmed)? {
        return Ok(structured);
    }
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    Ok(text_prompt(trimmed))
}

pub fn merge_prompt_source_with_text(
    source: &str,
    suffix_text: &str,
) -> Result<PromptInput, PromptInputValidationError> {
    let mut prompt = parse_prompt_source(source)?;
    let appended = suffix_text.trim();
    if appended.is_empty() {
        return Ok(prompt);
    }
    prompt.extend(text_prompt(appended));
    Ok(prompt)
}

pub fn prompt_to_display_text(prompt: &[ContentBlock]) -> String {
    prompt
        .iter()
        .map(display_text_from_content_block)
        .filter(|entry| !entry.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
        .trim()
        .to_string()
}

#[cfg(test)]
#[path = "prompt_content_tests.rs"]
mod prompt_content_tests;

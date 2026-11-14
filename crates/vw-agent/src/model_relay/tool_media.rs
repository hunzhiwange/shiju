//! Shared media handling for tool outputs.
//!
//! Responses and Anthropic tool outputs may carry structured media blocks.
//! Chat Completions tool messages are text-only, so protocol bridges extract
//! those blocks and re-emit them in a synthetic user message. The media
//! sanitizer reuses the same recognition and traversal rules when it needs to
//! remove images for a text-only upstream.

fn canonical_json_string(value: &serde_json::Value) -> String {
    value.to_string()
}
use serde_json::{Map, Value, json};

pub(crate) const WHOLE_DATA_URL_MIN_BYTES: usize = 8 * 1024;
pub(crate) const TOOL_RESULT_MEDIA_MOVED_MARKER: &str =
    "[shiju: tool result media moved to the following user message]";
const BASE64ISH_MIN_BYTES: usize = 16 * 1024;
const MAX_MEDIA_TRAVERSAL_DEPTH: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolMediaScope {
    AllSupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolMediaKind {
    Image,
    File,
    Audio,
}

pub(crate) struct ChatToolOutputMediaPlan {
    pub(crate) tool_content: String,
    pub(crate) media_parts: Vec<Value>,
}

impl ToolMediaScope {
    fn allows(self, _kind: ToolMediaKind) -> bool {
        true
    }
    fn accepts_chat_part(self, _part: &Value) -> bool {
        true
    }
}

/// Build a Chat-compatible tool-output plan without changing no-media output.
///
/// Scalar strings remain scalar strings after replacement. This matters for a
/// raw image data URL and for JSON encoded inside a tool-output string: adding
/// another layer of JSON string quotes would change what the model sees.
///
/// The placeholder says the media was "moved to the following user message".
/// Callers that re-attach the extracted media inside the same message must use
/// [`plan_chat_tool_output_media_with_marker`] so the placeholder matches where
/// the media actually lands.
pub(crate) fn plan_chat_tool_output_media(output: Value) -> Option<ChatToolOutputMediaPlan> {
    plan_chat_tool_output_media_with_marker(output, TOOL_RESULT_MEDIA_MOVED_MARKER)
}

/// Same extraction as [`plan_chat_tool_output_media`], with a caller-chosen
/// placeholder text for the stripped media blocks.
pub(crate) fn plan_chat_tool_output_media_with_marker(
    mut output: Value,
    replacement_marker: &str,
) -> Option<ChatToolOutputMediaPlan> {
    let output_was_string = output.is_string();
    let replacement_block = json!({
        "type": "text",
        "text": replacement_marker
    });
    let mut media_parts = Vec::new();
    let replaced = strip_and_clamp_media_from_tool_value(
        &mut output,
        &mut media_parts,
        ToolMediaScope::AllSupported,
        &replacement_block,
        replacement_marker,
    );
    if replaced == 0 {
        return None;
    }

    let tool_content = if output_was_string {
        output.as_str().unwrap_or_default().to_string()
    } else {
        canonical_json_string(&output)
    };

    Some(ChatToolOutputMediaPlan { tool_content, media_parts })
}

pub(crate) fn queue_chat_tool_output_media(
    pending_media: &mut Vec<Value>,
    call_id: &str,
    media_parts: Vec<Value>,
) {
    if media_parts.is_empty() {
        return;
    }

    pending_media.push(json!({
        "type": "text",
        "text": format!("[shiju: media output of tool call {call_id}]")
    }));
    pending_media.extend(media_parts);
}

pub(crate) fn flush_pending_chat_tool_media(
    messages: &mut Vec<Value>,
    pending_media: &mut Vec<Value>,
) {
    if pending_media.is_empty() {
        return;
    }

    messages.push(json!({
        "role": "user",
        "content": std::mem::take(pending_media)
    }));
}

/// Convert one recognized tool media block to a Chat user content part. This
/// is the single shape-recognition entry point used by extraction.
pub(crate) fn chat_media_part_from_tool_part(part: &Value, scope: ToolMediaScope) -> Option<Value> {
    let kind = tool_media_kind(part)?;
    if !scope.allows(kind) {
        return None;
    }

    let chat_part = match kind {
        ToolMediaKind::Image => chat_image_part(part),
        ToolMediaKind::File => chat_file_from_input_file(part).map(|file| {
            json!({
                "type": "file",
                "file": file
            })
        }),
        ToolMediaKind::Audio => part.get("input_audio").map(|input_audio| {
            json!({
                "type": "input_audio",
                "input_audio": input_audio.clone()
            })
        }),
    }?;

    scope.accepts_chat_part(&chat_part).then_some(chat_part)
}

/// Map a Responses `input_file` block to the Chat file payload. Kept here so
/// top-level content and tool-output extraction share the exact same rules.
pub(crate) fn chat_file_from_input_file(part: &Value) -> Option<Value> {
    let mut file = Map::new();
    let has_supported_file_ref = part.get("file_id").is_some() || part.get("file_data").is_some();
    if !has_supported_file_ref {
        return None;
    }

    for key in ["file_id", "file_data", "filename"] {
        if let Some(value) = part.get(key) {
            file.insert(key.to_string(), value.clone());
        }
    }
    Some(Value::Object(file))
}

/// Recognize a complete image data URL stored as a scalar string.
///
/// Only whole-string matches are accepted. Embedded data URLs in HTML/CSS/SVG
/// source are deliberately left alone. Small values remain text as well, which
/// preserves workflows that intentionally inspect tiny inline icons.
pub(crate) fn whole_string_image_data_url(value: &str) -> Option<Value> {
    let trimmed = value.trim();
    if trimmed.len() < WHOLE_DATA_URL_MIN_BYTES || !is_image_base64_data_url(trimmed) {
        return None;
    }

    Some(json!({
        "type": "image_url",
        "image_url": {
            "url": trimmed
        }
    }))
}

/// Extract media and clamp residual large data/base64 scalars on media-bearing
/// outputs. Parseable JSON strings are clamped while still represented as a
/// JSON tree, before they are canonicalized back into their original string
/// container.
pub(crate) fn strip_and_clamp_media_from_tool_value(
    value: &mut Value,
    media_parts: &mut Vec<Value>,
    scope: ToolMediaScope,
    replacement_block: &Value,
    replacement_text: &str,
) -> usize {
    let replaced = strip_media_from_tool_value_at_depth(
        value,
        media_parts,
        scope,
        replacement_block,
        replacement_text,
        true,
        0,
    );
    if replaced > 0 {
        clamp_base64ish_strings(value);
    }
    replaced
}

/// Remove residual data/base64 payloads only after a tool output has already
/// been positively identified as media-bearing. Ordinary long text is kept.
pub(crate) fn clamp_base64ish_strings(value: &mut Value) {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            let should_omit = (trimmed.len() >= WHOLE_DATA_URL_MIN_BYTES
                && trimmed.get(..5).is_some_and(|prefix| prefix.eq_ignore_ascii_case("data:")))
                || looks_like_base64_payload(trimmed);
            if should_omit {
                let byte_len = text.len();
                *text = format!("[shiju: omitted {byte_len} bytes]");
            }
        }
        Value::Array(items) => {
            for item in items {
                clamp_base64ish_strings(item);
            }
        }
        Value::Object(object) => {
            for nested in object.values_mut() {
                clamp_base64ish_strings(nested);
            }
        }
        _ => {}
    }
}

fn strip_media_from_tool_value_at_depth(
    value: &mut Value,
    media_parts: &mut Vec<Value>,
    scope: ToolMediaScope,
    replacement_block: &Value,
    replacement_text: &str,
    clamp_parsed_strings: bool,
    depth: usize,
) -> usize {
    if depth > MAX_MEDIA_TRAVERSAL_DEPTH {
        return 0;
    }

    match value {
        Value::String(text) => {
            if scope.allows(ToolMediaKind::Image) {
                if let Some(media_part) = whole_string_image_data_url(text) {
                    media_parts.push(media_part);
                    *text = replacement_text.to_string();
                    return 1;
                }
            }

            let trimmed = text.trim();
            if trimmed.is_empty() {
                return 0;
            }
            let Ok(mut parsed) = serde_json::from_str::<Value>(trimmed) else {
                return 0;
            };
            let replaced = strip_media_from_tool_value_at_depth(
                &mut parsed,
                media_parts,
                scope,
                replacement_block,
                replacement_text,
                clamp_parsed_strings,
                depth + 1,
            );
            if replaced > 0 {
                if clamp_parsed_strings {
                    clamp_base64ish_strings(&mut parsed);
                }
                *text = canonical_json_string(&parsed);
            }
            replaced
        }
        Value::Array(items) => items
            .iter_mut()
            .map(|item| {
                strip_media_from_tool_value_at_depth(
                    item,
                    media_parts,
                    scope,
                    replacement_block,
                    replacement_text,
                    clamp_parsed_strings,
                    depth + 1,
                )
            })
            .sum(),
        Value::Object(_) => {
            if let Some(media_part) = chat_media_part_from_tool_part(value, scope) {
                media_parts.push(media_part);
                *value = replacement_block.clone();
                return 1;
            }

            value
                .as_object_mut()
                .expect("object match arm must remain an object")
                .get_mut("content")
                .map(|content| {
                    strip_media_from_tool_value_at_depth(
                        content,
                        media_parts,
                        scope,
                        replacement_block,
                        replacement_text,
                        clamp_parsed_strings,
                        depth + 1,
                    )
                })
                .unwrap_or(0)
        }
        _ => 0,
    }
}

fn tool_media_kind(part: &Value) -> Option<ToolMediaKind> {
    let object = part.as_object()?;
    let part_type = object.get("type").and_then(Value::as_str);

    match part_type {
        Some("input_image" | "image_url") if normalized_image_url(part).is_some() => {
            Some(ToolMediaKind::Image)
        }
        Some("input_file") if part.get("file_id").is_some() || part.get("file_data").is_some() => {
            Some(ToolMediaKind::File)
        }
        Some("input_audio") if part.get("input_audio").is_some_and(Value::is_object) => {
            Some(ToolMediaKind::Audio)
        }
        Some("image") if typed_image_has_payload(part) => Some(ToolMediaKind::Image),
        None if loose_data_image_url(part).is_some() => Some(ToolMediaKind::Image),
        _ => None,
    }
}

fn chat_image_part(part: &Value) -> Option<Value> {
    match part.get("type").and_then(Value::as_str) {
        Some("input_image" | "image_url") => normalized_image_url(part).map(image_url_content_part),
        Some("image") => typed_image_url(part).map(image_url_content_part),
        None => loose_data_image_url(part).map(image_url_content_part),
        _ => None,
    }
}

fn normalized_image_url(part: &Value) -> Option<Value> {
    let image_url = part.get("image_url")?;
    let mut object = match image_url {
        Value::String(url) if !url.trim().is_empty() => {
            let mut object = Map::new();
            object.insert("url".to_string(), Value::String(url.clone()));
            object
        }
        Value::Object(object)
            if object
                .get("url")
                .and_then(Value::as_str)
                .is_some_and(|url| !url.trim().is_empty()) =>
        {
            object.clone()
        }
        _ => return None,
    };
    merge_top_level_detail(part, &mut object);
    Some(Value::Object(object))
}

fn loose_data_image_url(part: &Value) -> Option<Value> {
    if part.get("type").is_some() {
        return None;
    }
    let normalized = normalized_image_url(part)?;
    let url = normalized.get("url").and_then(Value::as_str)?;
    if !url.get(..5).is_some_and(|prefix| prefix.eq_ignore_ascii_case("data:")) {
        return None;
    }
    Some(normalized)
}

fn typed_image_has_payload(part: &Value) -> bool {
    let Some(object) = part.as_object() else {
        return false;
    };

    if let Some(source) = object.get("source").and_then(Value::as_object) {
        if source_media_type_is_image(source) {
            let has_url =
                source.get("url").and_then(Value::as_str).is_some_and(|url| !url.trim().is_empty());
            let has_data =
                source.get("data").and_then(Value::as_str).is_some_and(|data| !data.is_empty());
            if has_url || has_data {
                return true;
            }
        }
    }

    object.get("data").and_then(Value::as_str).is_some_and(|data| !data.is_empty())
        && object
            .get("mimeType")
            .or_else(|| object.get("mime_type"))
            .and_then(Value::as_str)
            .is_some_and(is_image_mime_type)
}

fn typed_image_url(part: &Value) -> Option<Value> {
    let object = part.as_object()?;

    if let Some(source) = object.get("source").and_then(Value::as_object) {
        if !source_media_type_is_image(source) {
            return None;
        }

        if let Some(url) =
            source.get("url").and_then(Value::as_str).filter(|url| !url.trim().is_empty())
        {
            let mut image_url = Map::new();
            image_url.insert("url".to_string(), Value::String(url.to_string()));
            merge_top_level_detail(part, &mut image_url);
            return Some(Value::Object(image_url));
        }

        if let Some(data) =
            source.get("data").and_then(Value::as_str).filter(|data| !data.is_empty())
        {
            let media_type = source
                .get("media_type")
                .or_else(|| source.get("mime_type"))
                .or_else(|| source.get("mimeType"))
                .and_then(Value::as_str)
                .unwrap_or("image/png");
            let url = if data
                .get(..11)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("data:image/"))
            {
                data.to_string()
            } else {
                format!("data:{media_type};base64,{data}")
            };
            let mut image_url = Map::new();
            image_url.insert("url".to_string(), Value::String(url));
            merge_top_level_detail(part, &mut image_url);
            return Some(Value::Object(image_url));
        }
    }

    let data = object.get("data").and_then(Value::as_str).filter(|data| !data.is_empty())?;
    let media_type = object
        .get("mimeType")
        .or_else(|| object.get("mime_type"))
        .and_then(Value::as_str)
        .filter(|media_type| is_image_mime_type(media_type))?;
    let mut image_url = Map::new();
    image_url.insert("url".to_string(), Value::String(format!("data:{media_type};base64,{data}")));
    merge_top_level_detail(part, &mut image_url);
    Some(Value::Object(image_url))
}

fn image_url_content_part(image_url: Value) -> Value {
    let mut content_part = Map::new();
    content_part.insert("type".to_string(), Value::String("image_url".to_string()));
    content_part.insert("image_url".to_string(), image_url);
    Value::Object(content_part)
}

fn merge_top_level_detail(part: &Value, image_url: &mut Map<String, Value>) {
    if image_url.get("detail").is_none() {
        if let Some(detail) = part.get("detail") {
            image_url.insert("detail".to_string(), detail.clone());
        }
    }

    // OpenAI-compatible Chat gateways only accept `auto` / `low` / `high`
    // here. Codex emits `original` (full-resolution) for models whose catalog
    // advertises `supports_image_detail_original`, and strict gateways reject
    // the entire request with `400 invalid_request_error`, param
    // `messages.N.content`. Downgrade instead of forwarding it verbatim.
    if image_url.get("detail").and_then(Value::as_str) == Some("original") {
        image_url.insert("detail".to_string(), Value::String("auto".to_string()));
    }
}

fn source_media_type_is_image(source: &Map<String, Value>) -> bool {
    source
        .get("media_type")
        .or_else(|| source.get("mime_type"))
        .or_else(|| source.get("mimeType"))
        .and_then(Value::as_str)
        .is_none_or(is_image_mime_type)
}

fn is_image_mime_type(value: &str) -> bool {
    value.get(..6).is_some_and(|prefix| prefix.eq_ignore_ascii_case("image/"))
}

fn is_image_base64_data_url(value: &str) -> bool {
    let Some(comma_index) = value.find(',') else {
        return false;
    };
    let header = &value[..comma_index];
    let header = header.to_ascii_lowercase();
    header.starts_with("data:image/") && header.ends_with(";base64")
}

fn looks_like_base64_payload(value: &str) -> bool {
    if value.len() < BASE64ISH_MIN_BYTES {
        return false;
    }

    value
        .bytes()
        .all(|byte| matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'+' | b'/' | b'='))
}

pub(super) use super::tool_media_request::{attach, messages, request, responses};

//! Bounded observation of upstream SSE before any client model mapping.
//!
//! Conversion and diagnostics share this protocol validation. Observation never
//! retains model output, tool arguments, credentials or request-local references.
use std::collections::{BTreeMap, BTreeSet};

use axum::body::Body;
use futures_util::StreamExt;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::generation::GenerationProtocol;

const MAX_FRAME: usize = 4 * 1024 * 1024;
const MAX_BLOCKS: usize = 1024;

#[derive(Default)]
pub(crate) struct StreamUpdate {
    pub(crate) text_events: u32,
    pub(crate) completed: bool,
}

#[derive(Clone, Copy)]
enum BlockKind {
    Text,
    Thinking,
    Tool,
    Other,
}

pub(crate) struct StreamObserver {
    protocol: GenerationProtocol,
    pending: super::sse_framing::Lines,
    data: String,
    event: String,
    started: bool,
    stopped: bool,
    completed: bool,
    blocks: BTreeSet<u64>,
    finished_choices: BTreeSet<u64>,
    message_blocks: BTreeMap<u64, BlockKind>,
    model: Option<String>,
}

#[derive(Debug)]
pub(crate) enum InvalidStream {
    Malformed,
    LimitExceeded,
}

type Result<T> = std::result::Result<T, InvalidStream>;

impl Default for StreamObserver {
    fn default() -> Self {
        Self::new(GenerationProtocol::Chat)
    }
}

impl StreamObserver {
    pub(crate) fn new(protocol: GenerationProtocol) -> Self {
        Self {
            protocol,
            pending: super::sse_framing::Lines::default(),
            data: String::new(),
            event: String::new(),
            started: false,
            stopped: false,
            completed: false,
            blocks: BTreeSet::new(),
            finished_choices: BTreeSet::new(),
            message_blocks: BTreeMap::new(),
            model: None,
        }
    }

    pub(crate) fn actual_model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// Incomplete frames are bounded and carried across network chunk boundaries.
    pub(crate) fn push(&mut self, bytes: &[u8]) -> Result<StreamUpdate> {
        let mut update = StreamUpdate::default();
        for line in self.pending.push(bytes, MAX_FRAME)? {
            if self.data.len() + self.event.len() + line.len() > MAX_FRAME {
                return Err(InvalidStream::LimitExceeded);
            }
            if line.is_empty() {
                if !self.data.is_empty() {
                    let data = std::mem::take(&mut self.data);
                    let event = std::mem::take(&mut self.event);
                    let text = self.frame(data.trim_end_matches('\n'), &event)?;
                    update.text_events += u32::from(text);
                }
                self.event.clear();
            } else {
                if let Some(data) = field(&line, "data") {
                    self.data.push_str(data);
                    self.data.push('\n');
                }
                if let Some(event) = field(&line, "event") {
                    self.event = event.to_owned();
                }
            }
        }
        if self.pending.len() + self.data.len() + self.event.len() > MAX_FRAME {
            return Err(InvalidStream::LimitExceeded);
        }
        if self.completed
            && self.protocol != GenerationProtocol::Gemini
            && (!self.pending.is_empty() || !self.data.is_empty() || !self.event.is_empty())
        {
            return Err(InvalidStream::Malformed);
        }
        update.completed = self.completed;
        Ok(update)
    }

    /// EOF is never completion by itself, including a partially received frame.
    pub(crate) fn finish(&self) -> Result<()> {
        if self.completed
            && self.pending.is_empty()
            && self.data.is_empty()
            && self.event.is_empty()
        {
            Ok(())
        } else {
            Err(InvalidStream::Malformed)
        }
    }

    fn model(&mut self, value: &Value) -> Result<()> {
        if value.is_null() {
            return Ok(());
        }
        let model = value.as_str().ok_or(InvalidStream::Malformed)?;
        if model.trim().is_empty()
            || model.len() > 1024
            || model.chars().any(char::is_control)
            || self.model.as_deref().is_some_and(|previous| previous != model)
        {
            return Err(InvalidStream::Malformed);
        }
        self.model = Some(model.into());
        Ok(())
    }

    fn frame(&mut self, data: &str, event: &str) -> Result<bool> {
        if event == "error" || event == "response.failed" {
            return Err(InvalidStream::Malformed);
        }
        if data == "[DONE]" {
            if self.completed
                || self.protocol != GenerationProtocol::Chat
                || !self.started
                || !self.stopped
            {
                return Err(InvalidStream::Malformed);
            }
            self.completed = true;
            return Ok(false);
        }
        let value: Value = serde_json::from_str(data).map_err(|_| InvalidStream::Malformed)?;
        if !value.is_object() || value.get("error").is_some_and(|value| !value.is_null()) {
            return Err(InvalidStream::Malformed);
        }
        let metadata = |kind: &str| matches!(kind, "ping" | "heartbeat" | "keepalive");
        if metadata(event) || value["type"].as_str().is_some_and(metadata) {
            return Ok(false);
        }
        if self.completed && self.protocol != GenerationProtocol::Gemini {
            return Err(InvalidStream::Malformed);
        }
        match self.protocol {
            GenerationProtocol::Chat => self.chat(&value),
            GenerationProtocol::Messages => self.messages(&value, event),
            GenerationProtocol::Responses => self.responses(&value, event),
            GenerationProtocol::Gemini => self.gemini(&value),
        }
    }

    fn chat(&mut self, value: &Value) -> Result<bool> {
        if value.get("object").is_some_and(|value| value != "chat.completion.chunk") {
            return Err(InvalidStream::Malformed);
        }
        self.model(&value["model"])?;
        let choices = value["choices"].as_array().ok_or(InvalidStream::Malformed)?;
        // Usage may follow a finish_reason, before the mandatory [DONE].
        let mut text = false;
        for choice in choices {
            let index = choice["index"]
                .as_u64()
                .filter(|index| *index < MAX_BLOCKS as u64)
                .ok_or(InvalidStream::Malformed)?;
            if self.finished_choices.contains(&index) {
                return Err(InvalidStream::Malformed);
            }
            self.blocks.insert(index);
            let delta = choice["delta"].as_object().ok_or(InvalidStream::Malformed)?;
            if delta.get("role").is_some_and(|role| role != "assistant") {
                return Err(InvalidStream::Malformed);
            }
            if let Some(content) = delta.get("content").filter(|value| !value.is_null()) {
                text |= !content.as_str().ok_or(InvalidStream::Malformed)?.trim().is_empty();
            }
            if let Some(reason) = choice.get("finish_reason").filter(|value| !value.is_null()) {
                if !matches!(
                    reason.as_str(),
                    Some("stop" | "length" | "tool_calls" | "function_call" | "content_filter")
                ) {
                    return Err(InvalidStream::Malformed);
                }
                self.blocks.remove(&index);
                self.finished_choices.insert(index);
            }
        }
        self.stopped = self.blocks.is_empty();
        self.started |= !choices.is_empty();
        Ok(text)
    }

    fn messages(&mut self, value: &Value, event: &str) -> Result<bool> {
        let kind = value["type"].as_str().ok_or(InvalidStream::Malformed)?;
        if !event.is_empty() && event != kind {
            return Err(InvalidStream::Malformed);
        }
        if kind == "ping" {
            return Ok(false);
        }
        if kind == "message_start" {
            if self.started || value["message"]["role"] != "assistant" {
                return Err(InvalidStream::Malformed);
            }
            self.model(&value["message"]["model"])?;
            self.started = true;
            return Ok(false);
        }
        if !self.started {
            return Err(InvalidStream::Malformed);
        }
        match kind {
            "content_block_start" => {
                let index = value["index"].as_u64().ok_or(InvalidStream::Malformed)?;
                let kind = match value["content_block"]["type"]
                    .as_str()
                    .ok_or(InvalidStream::Malformed)?
                {
                    "text" => BlockKind::Text,
                    "thinking" | "redacted_thinking" => BlockKind::Thinking,
                    "tool_use" | "server_tool_use" => BlockKind::Tool,
                    _ => BlockKind::Other,
                };
                if self.stopped
                    || self.message_blocks.len() >= MAX_BLOCKS
                    || self.message_blocks.insert(index, kind).is_some()
                {
                    return Err(InvalidStream::Malformed);
                }
            }
            "content_block_delta" => {
                let index = value["index"].as_u64().ok_or(InvalidStream::Malformed)?;
                let block = self.message_blocks.get(&index).ok_or(InvalidStream::Malformed)?;
                if self.stopped {
                    return Err(InvalidStream::Malformed);
                }
                return match (block, value["delta"]["type"].as_str()) {
                    (BlockKind::Text, Some("text_delta")) => Ok(!value["delta"]["text"]
                        .as_str()
                        .ok_or(InvalidStream::Malformed)?
                        .trim()
                        .is_empty()),
                    (BlockKind::Text, Some("citations_delta"))
                    | (BlockKind::Tool, Some("input_json_delta"))
                    | (BlockKind::Thinking, Some("thinking_delta" | "signature_delta")) => {
                        Ok(false)
                    }
                    _ => Err(InvalidStream::Malformed),
                };
            }
            "content_block_stop" => {
                if self
                    .message_blocks
                    .remove(&value["index"].as_u64().ok_or(InvalidStream::Malformed)?)
                    .is_none()
                {
                    return Err(InvalidStream::Malformed);
                }
            }
            "message_delta" => {
                if !self.message_blocks.is_empty() || self.stopped {
                    return Err(InvalidStream::Malformed);
                }
                if !matches!(
                    value["delta"]["stop_reason"].as_str(),
                    Some(
                        "end_turn"
                            | "max_tokens"
                            | "stop_sequence"
                            | "tool_use"
                            | "pause_turn"
                            | "refusal"
                    )
                ) {
                    return Err(InvalidStream::Malformed);
                }
                self.stopped = true;
            }
            "message_stop" => {
                if !self.stopped || !self.message_blocks.is_empty() {
                    return Err(InvalidStream::Malformed);
                }
                self.completed = true;
            }
            _ => return Err(InvalidStream::Malformed),
        }
        Ok(false)
    }

    fn responses(&mut self, value: &Value, event: &str) -> Result<bool> {
        let kind = value["type"].as_str().ok_or(InvalidStream::Malformed)?;
        if !event.is_empty() && event != kind {
            return Err(InvalidStream::Malformed);
        }
        if kind == "response.created" {
            if self.started || value["response"]["status"] != "in_progress" {
                return Err(InvalidStream::Malformed);
            }
            self.model(&value["response"]["model"])?;
            self.started = true;
            return Ok(false);
        }
        if !self.started || !kind.starts_with("response.") {
            return Err(InvalidStream::Malformed);
        }
        match kind {
            "response.output_text.delta" => {
                Ok(!value["delta"].as_str().ok_or(InvalidStream::Malformed)?.trim().is_empty())
            }
            "response.completed" | "response.incomplete" => {
                let status = if kind == "response.completed" { "completed" } else { "incomplete" };
                if value["response"]["status"] != status {
                    return Err(InvalidStream::Malformed);
                }
                self.model(&value["response"]["model"])?;
                self.completed = true;
                Ok(false)
            }
            "response.failed" | "response.cancelled" | "response.error" => {
                Err(InvalidStream::Malformed)
            }
            _ => Ok(false),
        }
    }

    fn gemini(&mut self, value: &Value) -> Result<bool> {
        self.model(&value["modelVersion"])?;
        if value["promptFeedback"].get("blockReason").is_some() {
            return Err(InvalidStream::Malformed);
        }
        let candidates = value.get("candidates").and_then(Value::as_array);
        if self.completed {
            return if candidates.is_none_or(Vec::is_empty) && value["usageMetadata"].is_object() {
                Ok(false)
            } else {
                Err(InvalidStream::Malformed)
            };
        }
        let mut text = false;
        if let Some(candidates) = candidates {
            for candidate in candidates {
                let index = candidate
                    .get("index")
                    .map_or(Ok(0), |index| index.as_u64().ok_or(InvalidStream::Malformed))?;
                if index >= MAX_BLOCKS as u64 || self.finished_choices.contains(&index) {
                    return Err(InvalidStream::Malformed);
                }
                self.blocks.insert(index);
                if let Some(parts) = candidate["content"]["parts"].as_array() {
                    for part in parts {
                        if let Some(value) = part.get("text") {
                            text |= part["thought"] != true
                                && !value
                                    .as_str()
                                    .ok_or(InvalidStream::Malformed)?
                                    .trim()
                                    .is_empty();
                        }
                    }
                }
                if let Some(reason) = candidate.get("finishReason") {
                    if !matches!(
                        reason.as_str(),
                        Some(
                            "STOP"
                                | "MAX_TOKENS"
                                | "SAFETY"
                                | "RECITATION"
                                | "OTHER"
                                | "BLOCKLIST"
                                | "PROHIBITED_CONTENT"
                                | "SPII"
                                | "MALFORMED_FUNCTION_CALL"
                                | "LANGUAGE"
                                | "UNEXPECTED_TOOL_CALL"
                                | "IMAGE_SAFETY"
                                | "IMAGE_PROHIBITED_CONTENT"
                                | "IMAGE_RECITATION"
                                | "NO_IMAGE"
                                | "IMAGE_OTHER"
                        )
                    ) {
                        return Err(InvalidStream::Malformed);
                    }
                    self.blocks.remove(&index);
                    self.finished_choices.insert(index);
                }
            }
            self.completed = !self.finished_choices.is_empty() && self.blocks.is_empty();
        } else if value.get("usageMetadata").is_none() {
            return Err(InvalidStream::Malformed);
        }
        Ok(text)
    }
}

fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let (key, value) = line.split_once(':').unwrap_or((line, ""));
    (key == name).then(|| value.strip_prefix(' ').unwrap_or(value))
}

/// Validate source bytes before passing them to the client or a converter.
/// Dropping this body or cancelling it drops the upstream transport immediately.
pub(crate) fn validated_body(
    body: Body,
    protocol: GenerationProtocol,
    cancel: CancellationToken,
) -> Body {
    Body::from_stream(futures_util::stream::unfold(
        (body.into_data_stream(), StreamObserver::new(protocol), cancel, false),
        |(mut stream, mut observer, cancel, done)| async move {
            if done {
                return None;
            }
            let next = tokio::select! { biased; () = cancel.cancelled() => return None, next = stream.next() => next };
            let (chunk, done) = match next {
                Some(Ok(bytes)) => match observer.push(&bytes) {
                    Ok(update) => (Ok(bytes), update.completed),
                    Err(_) => (Err(std::io::Error::other("Invalid upstream stream")), true),
                },
                Some(Err(_)) => (Err(std::io::Error::other("Upstream stream failed")), true),
                None if observer.finish().is_ok() => return None,
                None => {
                    (Err(std::io::Error::other("Upstream stream ended before completion")), true)
                }
            };
            Some((chunk, (stream, observer, cancel, done)))
        },
    ))
}

#[cfg(test)]
#[path = "stream_observer_tests.rs"]
pub(crate) mod tests;

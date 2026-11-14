//! Protocol usage projections. Counters are snapshots, never stream increments.
use serde_json::Value;
use vw_api_types::usage::{UsageInputSemantics, UsageReasoningSemantics, UsageTokens};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageProtocol {
    Claude,
    OpenAi,
    Responses,
    Gemini,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUsage {
    pub tokens: UsageTokens,
    pub response_model: Option<String>,
    pub response_id: Option<String>,
}

impl ParsedUsage {
    /// A synthetic all-zero usage must not increment billable request counts.
    pub fn has_billable_tokens(&self) -> bool {
        let t = &self.tokens;
        [t.input, t.output, t.reasoning, t.cache_read, t.cache_creation, t.cache_creation_1h]
            .into_iter()
            .flatten()
            .any(|n| n > 0)
    }
}

fn number(value: &Value, path: &str) -> Option<u64> {
    value.pointer(path).and_then(Value::as_u64)
}

fn counter(value: &Value, paths: &[&str]) -> Option<u64> {
    paths
        .iter()
        .filter_map(|path| number(value, path))
        .find(|n| *n > 0)
        .or_else(|| paths.iter().find_map(|path| number(value, path)))
}

fn label(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_owned)
}

/// Missing fields remain unknown. OpenAI/Responses input includes cached tokens;
/// Claude input is fresh. Gemini output includes thoughts when total is provided.
pub fn parse_response(protocol: UsageProtocol, body: &Value) -> Option<ParsedUsage> {
    let body = if body.get("usage").is_none() && body.get("data").is_some_and(Value::is_object) {
        &body["data"]
    } else {
        body
    };
    let gemini = protocol == UsageProtocol::Gemini;
    let usage = body.get(if gemini { "usageMetadata" } else { "usage" })?;
    if !usage.is_object() {
        return None;
    }
    let mut tokens = UsageTokens {
        input: None,
        output: None,
        reasoning: None,
        cache_read: None,
        cache_creation: None,
        cache_creation_1h: None,
        input_semantics: UsageInputSemantics::Total,
        reasoning_semantics: UsageReasoningSemantics::IncludedInOutput,
    };
    match protocol {
        UsageProtocol::Claude => {
            tokens.input_semantics = UsageInputSemantics::Fresh;
            tokens.reasoning_semantics = UsageReasoningSemantics::Unknown;
            tokens.input = number(usage, "/input_tokens");
            tokens.output = number(usage, "/output_tokens");
            tokens.cache_read = number(usage, "/cache_read_input_tokens");
            tokens.cache_creation = number(usage, "/cache_creation_input_tokens");
            tokens.cache_creation_1h = number(usage, "/cache_creation/ephemeral_1h_input_tokens");
        }
        UsageProtocol::OpenAi | UsageProtocol::Responses => {
            tokens.input = counter(usage, &["/prompt_tokens", "/input_tokens"]);
            tokens.output = counter(usage, &["/completion_tokens", "/output_tokens"]);
            tokens.reasoning = counter(
                usage,
                &[
                    "/output_tokens_details/reasoning_tokens",
                    "/completion_tokens_details/reasoning_tokens",
                ],
            );
            tokens.cache_read = counter(
                usage,
                &[
                    "/cache_read_input_tokens",
                    "/input_tokens_details/cached_tokens",
                    "/prompt_tokens_details/cached_tokens",
                    "/prompt_cache_hit_tokens",
                ],
            );
            tokens.cache_creation = counter(
                usage,
                &[
                    "/cache_creation_input_tokens",
                    "/input_tokens_details/cache_write_tokens",
                    "/prompt_tokens_details/cache_write_tokens",
                ],
            );
        }
        UsageProtocol::Gemini => {
            tokens.input = number(usage, "/promptTokenCount");
            tokens.reasoning = number(usage, "/thoughtsTokenCount");
            tokens.output = number(usage, "/totalTokenCount")
                .zip(tokens.input)
                .and_then(|(total, input)| total.checked_sub(input));
            if tokens.output.is_none() {
                tokens.output = number(usage, "/candidatesTokenCount");
                tokens.reasoning_semantics = UsageReasoningSemantics::SeparateFromOutput;
            }
            tokens.cache_read = number(usage, "/cachedContentTokenCount");
        }
    }
    if [
        tokens.input,
        tokens.output,
        tokens.reasoning,
        tokens.cache_read,
        tokens.cache_creation,
        tokens.cache_creation_1h,
    ]
    .iter()
    .all(Option::is_none)
    {
        return None;
    }
    Some(ParsedUsage {
        tokens,
        response_model: label(body, if gemini { "modelVersion" } else { "model" }),
        response_id: label(body, if gemini { "responseId" } else { "id" }),
    })
}

/// Parse decoded SSE JSON events (or Gemini chunks), retaining correlation fields.
/// Usage-only final chunks inherit IDs/models from earlier chunks.
pub fn parse_stream(protocol: UsageProtocol, events: &[Value]) -> Option<ParsedUsage> {
    let mut result: Option<ParsedUsage> = None;
    let mut model = None;
    let mut id = None;
    let mut claude_delta_input = false;
    for event in events {
        let body = match protocol {
            UsageProtocol::Claude if event["type"] == "message_start" => &event["message"],
            UsageProtocol::Responses if event.get("response").is_some() => &event["response"],
            _ => event,
        };
        model =
            label(body, if protocol == UsageProtocol::Gemini { "modelVersion" } else { "model" })
                .or(model);
        id =
            label(body, if protocol == UsageProtocol::Gemini { "responseId" } else { "id" }).or(id);
        let Some(mut next) = parse_response(protocol, body) else { continue };
        if let Some(previous) = &result {
            let old = &previous.tokens;
            let new = &mut next.tokens;
            if protocol == UsageProtocol::Claude {
                let adopt = new.input.is_some_and(|input| {
                    input > 0
                        && (old.input.unwrap_or(0) == 0
                            || input < old.input.unwrap_or(0)
                            || (claude_delta_input && Some(input) == old.input))
                });
                if !adopt {
                    new.input = old.input;
                    new.cache_read = old.cache_read;
                    new.cache_creation = old.cache_creation;
                    new.cache_creation_1h = old.cache_creation_1h;
                } else {
                    claude_delta_input = true;
                }
            }
            new.input = new.input.or(old.input);
            new.output = new.output.or(old.output);
            new.reasoning = new.reasoning.or(old.reasoning);
            new.cache_read = new.cache_read.or(old.cache_read);
            new.cache_creation = new.cache_creation.or(old.cache_creation);
            new.cache_creation_1h = new.cache_creation_1h.or(old.cache_creation_1h);
        }
        result = Some(next);
    }
    result.map(|mut parsed| {
        parsed.response_model = model;
        parsed.response_id = id;
        parsed
    })
}

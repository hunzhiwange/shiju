//! Per-rollout Codex usage projection. No filesystem discovery or cross-file replay.
use std::{collections::HashMap, io::{self, BufRead}};

use serde_json::Value;
use vw_api_types::usage::{UsageInputSemantics, UsageReasoningSemantics, UsageTokens};

use super::tokens::ParsedUsage;
use super::codex_timing::{CodexTiming, RequestTimer};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct Counters {
    input: Option<u64>,
    cached: Option<u64>,
    creation: Option<u64>,
    output: Option<u64>,
    reasoning: Option<u64>,
    total: Option<u64>,
}

impl Counters {
    pub(super) fn parse(value: &Value) -> Option<Self> {
        value.as_object()?;
        let number = |key| value.get(key).and_then(Value::as_u64);
        let result = Self {
            input: number("input_tokens"),
            cached: number("cached_input_tokens").or_else(|| number("cache_read_input_tokens")),
            creation: number("cache_write_input_tokens"),
            output: number("output_tokens"),
            reasoning: number("reasoning_output_tokens"),
            total: number("total_tokens"),
        };
        (result != Self::default()).then_some(result)
    }

    pub(super) fn matches_request(&self, other: &Self) -> bool {
        self.input.is_some() && self.output.is_some()
            && self.input == other.input && self.cached == other.cached
            && self.output == other.output
    }

    fn delta(&self, previous: &Self) -> Self {
        fn subtract(current: Option<u64>, previous: Option<u64>) -> Option<u64> {
            current.map(|n| n.saturating_sub(previous.unwrap_or(0)))
        }
        Self {
            input: subtract(self.input, previous.input),
            cached: subtract(self.cached, previous.cached),
            creation: subtract(self.creation, previous.creation),
            output: subtract(self.output, previous.output),
            reasoning: subtract(self.reasoning, previous.reasoning),
            total: subtract(self.total, previous.total),
        }
    }

    pub(super) fn advance(&mut self, current: &Self) {
        self.input = self.input.max(current.input);
        self.cached = self.cached.max(current.cached);
        self.creation = self.creation.max(current.creation);
        self.output = self.output.max(current.output);
        self.reasoning = self.reasoning.max(current.reasoning);
        self.total = self.total.max(current.total);
    }

    fn tokens(self) -> UsageTokens {
        let cached = self.cached.map(|n| self.input.map_or(n, |input| n.min(input)));
        let creation = self.creation.map(|n| self.input.map_or(n, |input| {
            n.min(input.saturating_sub(cached.unwrap_or(0)))
        }));
        UsageTokens {
            input: self.input,
            output: self.output,
            reasoning: self.reasoning,
            cache_read: cached,
            cache_creation: creation,
            cache_creation_1h: None,
            input_semantics: UsageInputSemantics::Total,
            reasoning_semantics: UsageReasoningSemantics::IncludedInOutput,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotSignature {
    total: Option<Counters>,
    last: Option<Counters>,
}

/// One billable token_count event; line identity remains stable on a full reparse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexUsageDelta {
    pub line_number: u64,
    pub session_id: Option<String>,
    pub timestamp: Option<String>,
    /// Exact source model is retained separately from its normalized pricing name.
    pub request_model: Option<String>,
    pub pricing_model: Option<String>,
    pub usage: ParsedUsage,
    pub timing: CodexTiming,
}

/// Keep one state per rollout, in file order. A new file requires a new state;
/// continuation/fork baseline recovery belongs to the scanner, not this parser.
#[derive(Debug, Default)]
pub struct CodexDeltaParser {
    high_water: Counters,
    timer: RequestTimer,
    previous: Option<SnapshotSignature>,
    source_signatures: HashMap<Option<String>, SnapshotSignature>,
    model: Option<String>,
    session_id: Option<String>,
    line_number: u64,
}

impl CodexDeltaParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Restore a preceding segment only when the new segment has not reset its
    /// cumulative input counter. Exact per-request usage still takes precedence.
    pub(super) fn inherit_baseline(&mut self, baseline: &Counters, first: &Value) {
        let current = first
            .pointer("/payload/info/total_token_usage")
            .and_then(Counters::parse);
        if current.as_ref().is_some_and(|c| c.input >= baseline.input) {
            self.high_water.advance(baseline);
        }
    }

    pub fn observe(&mut self, event: &Value) -> Option<CodexUsageDelta> {
        self.line_number = self.line_number.saturating_add(1);
        self.timer.observe(event);
        let payload = event.get("payload")?;
        match event.get("type")?.as_str()? {
            "session_meta" => {
                self.session_id = label(payload, "id").or(self.session_id.take());
                return None;
            }
            "turn_context" => {
                self.model = label(payload, "model")
                    .or_else(|| payload.get("info").and_then(|info| label(info, "model")))
                    .or(self.model.take());
                return None;
            }
            "event_msg" => {}
            _ => return None,
        }
        if payload.get("type").and_then(Value::as_str) == Some("thread_settings_applied") {
            self.model = payload.get("thread_settings").and_then(|v| label(v, "model"))
                .or(self.model.take());
            return None;
        }
        if payload.get("type").and_then(Value::as_str) != Some("token_count") {
            return None;
        }
        let info = payload.get("info")?;
        self.model = label(info, "model").or_else(|| label(info, "model_name"))
            .or_else(|| label(payload, "model")).or(self.model.take());
        let total = info.get("total_token_usage").and_then(Counters::parse);
        let last = info.get("last_token_usage").and_then(Counters::parse);
        if total.is_none() && last.is_none() {
            return None;
        }
        let signature = SnapshotSignature { total: total.clone(), last: last.clone() };
        let source = payload.get("rate_limits").and_then(|v| label(v, "limit_id"));
        // Last-only events can represent equal-sized consecutive requests. Only
        // a cumulative snapshot provides evidence for signature deduplication.
        let duplicate = total.is_some() && (self.previous.as_ref() == Some(&signature)
            || self.source_signatures.get(&source) == Some(&signature));
        if total.is_some() {
            self.source_signatures.insert(source, signature.clone());
        }
        self.previous = Some(signature);
        // Exact last usage wins: independent rate-limit lanes can regress total.
        let request_usage = last.clone();
        let delta = last.or_else(|| total.as_ref().map(|t| t.delta(&self.high_water)));
        if let Some(total) = &total {
            self.high_water.advance(total);
        }
        if duplicate {
            return None;
        }
        let usage = ParsedUsage {
            tokens: delta?.tokens(),
            response_model: self.model.clone(),
            response_id: None,
        };
        if !usage.has_billable_tokens() {
            return None;
        }
        Some(CodexUsageDelta {
            line_number: self.line_number,
            session_id: self.session_id.clone(),
            timestamp: label(event, "timestamp"),
            request_model: self.model.clone(),
            pricing_model: self.model.as_deref().map(normalize_codex_model),
            timing: self.timer.finish(event, request_usage.as_ref()),
            usage,
        })
    }
}

/// Decode a single rollout, ignoring non-JSON lines without losing line identity.
pub fn parse_reader(mut reader: impl BufRead) -> io::Result<Vec<CodexUsageDelta>> {
    let mut parser = CodexDeltaParser::new();
    let mut result = Vec::new();
    let mut line = Vec::new();
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        let event = serde_json::from_slice::<Value>(&line).unwrap_or(Value::Null);
        if let Some(delta) = parser.observe(&event) {
            result.push(delta);
        }
    }
    Ok(result)
}

fn label(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned)
}

/// Codex's provider prefix and dated revision do not change the pricing family.
pub fn normalize_codex_model(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();
    let mut name = lower.rsplit('/').next().unwrap_or(&lower).to_owned();
    let bytes = name.as_bytes();
    if bytes.len() > 11 {
        let suffix = &bytes[bytes.len() - 11..];
        if suffix[0] == b'-' && suffix[5] == b'-' && suffix[8] == b'-'
            && suffix[1..5].iter().chain(&suffix[6..8]).chain(&suffix[9..11])
                .all(u8::is_ascii_digit)
        {
            name.truncate(name.len() - 11);
        }
    }
    if let Some((base, date)) = name.rsplit_once('-') {
        if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) {
            return base.to_owned();
        }
    }
    name
}

//! Codex request timing inferred from rollout events, never wall-clock measurements.
use chrono::DateTime;
use serde_json::Value;
use vw_api_types::usage::{UsageMeasurement, UsageMeasurementKind, UsagePerformance, UsageTokens};

use super::codex_delta::Counters;

const FLUSH_SLACK_MS: i64 = 100;
const MAX_LATENCY_MS: i64 = 60 * 60 * 1000;

#[derive(Debug)]
enum TimingLine {
    Boundary,
    ToolOutput,
    ModelOutput,
    UsageRecord,
    Reset,
    ItemStarted { start: i64, message: bool },
}

fn classify_timing_line(event: &Value) -> Option<TimingLine> {
    let payload = &event["payload"];
    Some(match event["type"].as_str()? {
        "turn_context" => TimingLine::Boundary,
        "token_usage_record" => TimingLine::UsageRecord,
        "response_item" => match payload["type"].as_str()? {
            "message" if payload["role"] == "assistant" => TimingLine::ModelOutput,
            "message" => TimingLine::Boundary,
            "reasoning" => TimingLine::ModelOutput,
            kind if kind.ends_with("_output") => TimingLine::ToolOutput,
            kind if kind.ends_with("_call") => TimingLine::ModelOutput,
            _ => return None,
        },
        "event_msg" => match payload["type"].as_str()? {
            "task_started" | "turn_aborted" => TimingLine::Reset,
            "item_completed" => {
                let message = match payload["item"]["type"].as_str()? {
                    "Reasoning" => false,
                    "AgentMessage" => true,
                    _ => return None,
                };
                TimingLine::ItemStarted { start: payload["started_at_ms"].as_i64()?, message }
            }
            _ => return None,
        },
        _ => return None,
    })
}

fn timestamp(event: &Value) -> Option<i64> {
    DateTime::parse_from_rfc3339(event["timestamp"].as_str()?).ok().map(|t| t.timestamp_millis())
}

/// Milliseconds remain integers until conversion to the public performance DTO.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodexTiming {
    pub latency_ms: Option<i64>,
    /// Earliest model output, including reasoning, as in Codex item timing.
    pub first_token_ms: Option<i64>,
    /// Message output only; excludes the preceding reasoning phase.
    pub output_duration_ms: Option<i64>,
}

impl CodexTiming {
    pub fn performance(&self, tokens: &UsageTokens) -> UsagePerformance {
        let estimated = |value| UsageMeasurement { value, kind: UsageMeasurementKind::Estimated };
        let output = tokens.output.and_then(|output| {
            // Codex output includes reasoning. Without its count we cannot
            // attribute the numerator to the message generation interval.
            output.checked_sub(tokens.reasoning?)
        });
        UsagePerformance {
            first_token_ms: self.first_token_ms.map(|n| estimated(n as f64)),
            latency_ms: self.latency_ms.map(|n| estimated(n as f64)),
            output_tokens_per_second: output
                .zip(self.output_duration_ms)
                .filter(|(_, ms)| *ms > 0)
                .map(|(n, ms)| estimated(n as f64 * 1000.0 / ms as f64)),
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct RequestTimer {
    boundary: Option<i64>,
    previous_count: Option<i64>,
    last_seen: Option<i64>,
    start: Option<i64>,
    last_output: Option<i64>,
    tool_after_output: Option<i64>,
    record: Option<(i64, Counters)>,
    first: Option<i64>,
    message_start: Option<i64>,
    invalid: bool,
}

impl RequestTimer {
    pub(super) fn observe(&mut self, event: &Value) {
        let Some(kind) = classify_timing_line(event) else { return };
        let time = timestamp(event);
        if matches!(kind, TimingLine::Reset) {
            *self = Self {
                boundary: time,
                previous_count: self.previous_count,
                last_seen: time,
                ..Self::default()
            };
            return;
        }
        let Some(time) = time else {
            self.invalid = true;
            return;
        };
        if self.last_seen.is_some_and(|last| time < last) {
            self.invalid = true;
        }
        self.last_seen = Some(time);
        match kind {
            TimingLine::Boundary => self.boundary = Some(time),
            TimingLine::ToolOutput => {
                self.boundary = Some(time);
                if self.last_output.is_some() {
                    self.tool_after_output = Some(time);
                }
            }
            TimingLine::ModelOutput => {
                if self.start.is_none()
                    && self.previous_count.is_some_and(|last| {
                        time.checked_sub(last)
                            .is_some_and(|gap| (0..=FLUSH_SLACK_MS).contains(&gap))
                    })
                {
                    return;
                }
                self.start = self.start.or(self.boundary);
                self.last_output = Some(time);
                self.tool_after_output = None;
            }
            TimingLine::UsageRecord => {
                self.record =
                    Counters::parse(&event["payload"]["usage"]).map(|usage| (time, usage));
            }
            TimingLine::ItemStarted { start, message } => {
                self.start = self.start.or(self.boundary);
                if start > time || self.start.is_some_and(|boundary| start < boundary) {
                    self.invalid = true;
                }
                self.first = Some(self.first.map_or(start, |old| old.min(start)));
                if message {
                    self.message_start =
                        Some(self.message_start.map_or(start, |old| old.min(start)));
                }
            }
            TimingLine::Reset => unreachable!(),
        }
    }

    /// Called only for a nonduplicate billable snapshot. Consume all pending
    /// request state even if its timing is unknown, so it cannot leak forward.
    pub(super) fn finish(&mut self, event: &Value, last: Option<&Counters>) -> CodexTiming {
        let count = timestamp(event);
        let current = std::mem::replace(
            self,
            Self { boundary: count, previous_count: count, last_seen: count, ..Self::default() },
        );
        if current.invalid || count.zip(current.last_seen).is_some_and(|(end, seen)| end < seen) {
            return CodexTiming::default();
        }
        // A usage record is trustworthy only when its request counters match.
        let record = current
            .record
            .filter(|(_, usage)| last.is_some_and(|last| usage.matches_request(last)));
        let waited = current.tool_after_output.zip(count).is_some_and(|(tool, end)| {
            end.checked_sub(tool).is_some_and(|gap| (0..=FLUSH_SLACK_MS).contains(&gap))
        });
        let end = record.map(|(time, _)| time).or(if waited { current.last_output } else { count });
        let Some((start, end)) = current.start.or(current.boundary).zip(end) else {
            return CodexTiming::default();
        };
        let Some(latency) = end.checked_sub(start).filter(|ms| (100..=MAX_LATENCY_MS).contains(ms))
        else {
            return CodexTiming::default();
        };
        if current.last_output.is_some_and(|output| output > end)
            || current.first.is_some_and(|first| first < start || first > end)
        {
            return CodexTiming::default();
        }
        CodexTiming {
            latency_ms: Some(latency),
            first_token_ms: current
                .first
                .and_then(|first| first.checked_sub(start))
                .filter(|ms| *ms > 0),
            output_duration_ms: current
                .message_start
                .and_then(|first| end.checked_sub(first))
                .filter(|ms| *ms > 0),
        }
    }
}

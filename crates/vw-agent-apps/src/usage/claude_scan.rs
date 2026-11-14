//! Incremental Claude projections over sources discovered by
//! `sessions::providers::claude::scan_sessions` and resolved by SourceRegistry.
//! No home-directory fallback or client-supplied path is accepted.
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::fs::File;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use vw_api_types::external_session::{ExternalSessionIdentity, SessionProvider, SessionSource};
use vw_api_types::usage::{
    UsageCost, UsageCostSource, UsageMeasurement, UsageMeasurementKind, UsageModels,
    UsagePerformance, UsageRecord, UsageRequestStatus, UsageSourceKind, UsageSourceRef,
    UsageUnixSeconds,
};

use super::claude::{ClaudeUsageMessage, parse_reader};
use super::cursors::{
    UsageCursorError, UsageCursorKey, UsageScanOutcome, scan_usage_file_with_finish,
};
use super::store::{StoredUsageRecord, UsageSourceIdentity};
use crate::sessions::source::SourceRegistry;

pub const MIN_PLAUSIBLE_LATENCY_MS: i64 = 100;
const MAX_PLAUSIBLE_LATENCY_MS: i64 = 60 * 60 * 1000;
const CONTEXT_LIMIT: usize = 1024;

#[derive(Default, Serialize, Deserialize)]
struct Context {
    nodes: VecDeque<Node>,
    timings: VecDeque<Timing>,
}
#[derive(Serialize, Deserialize)]
struct Node {
    id: String,
    parent: Option<String>,
    time: Option<i64>,
    attachment: bool,
}
#[derive(Serialize, Deserialize)]
struct Timing {
    id: String,
    start: Option<i64>,
    first_output: Option<i64>,
    end: Option<i64>,
}
#[derive(Default)]
struct Parser {
    context: Option<Context>,
    messages: BTreeMap<String, ClaudeUsageMessage>,
}

/// `identity` comes from the existing Claude path enumerator. Registry resolution
/// rechecks scope, host, instance, containment and symlinks before opening the file.
/// Claude Desktop consuming the same native transcript must use this same binding.
/// A rewritten tail yields `history_changed`; callers must surface that warning.
/// Representative snapshots are selected within this scan. Existing ledger
/// identities retain their original payload, following the shared replay contract.
pub fn scan_session(
    connection: &mut Connection,
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
) -> Result<UsageScanOutcome, UsageCursorError> {
    if identity.provider != SessionProvider::Claude {
        return Err(UsageCursorError::InvalidRecord);
    }
    let source = registry.resolve(identity)?;
    let SessionSource::File { relative_path } = &identity.source else {
        return Err(UsageCursorError::InvalidRecord);
    };
    let key = UsageCursorKey {
        target_host: identity.target_host.clone(),
        app: "claude".into(),
        instance_id: identity.instance_id.clone(),
        file_identity: format!("claude-file:{:x}", Sha256::digest(relative_path.as_bytes())),
    };
    let mut file = File::open(source.path())?;
    let parser = RefCell::new(Parser::default());
    scan_usage_file_with_finish(
        connection,
        &identity.scope,
        &key,
        &mut file,
        |line, state| {
            parser.borrow_mut().line(line, state, &identity.session_id.0)?;
            Ok(Vec::new())
        },
        |state| std::mem::take(&mut *parser.borrow_mut()).finish(state, &key),
    )
}

impl Parser {
    fn line(
        &mut self,
        line: &[u8],
        state: &Option<String>,
        session: &str,
    ) -> Result<(), UsageCursorError> {
        if self.context.is_none() {
            self.context = Some(match state {
                Some(raw) => {
                    serde_json::from_str(raw).map_err(|_| UsageCursorError::InvalidCursor)?
                }
                None => Context::default(),
            });
        }
        let Ok(value) = serde_json::from_slice::<Value>(line) else { return Ok(()) };
        if value.get("sessionId").and_then(Value::as_str).is_some_and(|id| id != session) {
            return Err(UsageCursorError::InvalidRecord);
        }
        let context = self.context.as_mut().ok_or(UsageCursorError::InvalidCursor)?;
        context.observe(&value);
        for message in parse_reader(line, Some(session))? {
            // Without an original time, there is no truthful ledger timestamp.
            if message.timestamp.is_none() {
                continue;
            }
            let replace = match self.messages.get(&message.message_id) {
                None => true,
                Some(old) => match (old.stop_reason.is_some(), message.stop_reason.is_some()) {
                    (false, true) => true,
                    (true, false) => false,
                    _ => message.usage.tokens.output > old.usage.tokens.output,
                },
            };
            if replace {
                self.messages.insert(message.message_id.clone(), message);
            }
        }
        Ok(())
    }

    fn finish(
        self,
        state: &mut Option<String>,
        key: &UsageCursorKey,
    ) -> Result<Vec<(UsageSourceIdentity, StoredUsageRecord)>, UsageCursorError> {
        let Some(context) = self.context else { return Ok(Vec::new()) };
        let mut records = Vec::new();
        for message in self.messages.into_values() {
            let Some(timestamp) = message.timestamp else { continue };
            let identity = UsageSourceIdentity {
                target_host: key.target_host.clone(),
                app: key.app.clone(),
                instance_id: key.instance_id.clone(),
                file_identity: key.file_identity.clone(),
                kind: UsageSourceKind::Native,
                native_session_id: message.session_id.clone(),
                native_request_id: message.message_id.clone(),
            };
            let bytes =
                serde_json::to_vec(&identity).map_err(|_| UsageCursorError::InvalidRecord)?;
            let id = format!("claude:{:x}", Sha256::digest(bytes));
            let performance = context.performance(&message);
            let started = context
                .timings
                .iter()
                .find(|timing| timing.id == message.message_id)
                .and_then(|timing| timing.start)
                .filter(|_| performance.latency_ms.is_some())
                .map(|millis| UsageUnixSeconds(millis.div_euclid(1000)))
                .unwrap_or(timestamp);
            records.push((
                identity,
                StoredUsageRecord {
                    record: UsageRecord {
                        id: id.clone(),
                        started_at_unix_seconds: started,
                        provider: message.provider,
                        sources: vec![UsageSourceRef {
                            source_id: id,
                            kind: UsageSourceKind::Native,
                            app: key.app.clone(),
                            native_request_id: Some(message.message_id),
                            native_session_id: message.session_id,
                        }],
                        models: UsageModels {
                            request_model: None,
                            response_model: message.usage.response_model,
                            pricing_model: None,
                            pricing_multiplier: None,
                            multiplier_basis: None,
                        },
                        status: if message.stop_reason.is_some() {
                            UsageRequestStatus::Succeeded
                        } else {
                            UsageRequestStatus::Unknown
                        },
                        billable: true,
                        tokens: Some(message.usage.tokens),
                        cost: UsageCost { source: UsageCostSource::Unknown, amount_usd: None },
                        performance,
                        suspected_duplicate_of: None,
                    },
                    status_code: None,
                    failure: None,
                },
            ));
        }
        *state =
            Some(serde_json::to_string(&context).map_err(|_| UsageCursorError::InvalidCursor)?);
        Ok(records)
    }
}

fn timestamp(value: &Value) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value.get("timestamp")?.as_str()?)
        .ok()
        .map(|time| time.timestamp_millis())
}
fn uuid(value: &Value, key: &str) -> Option<String> {
    uuid::Uuid::parse_str(value.get(key)?.as_str()?).ok().map(|id| id.to_string())
}
fn estimated(value: i64) -> UsageMeasurement {
    UsageMeasurement { value: value as f64, kind: UsageMeasurementKind::Estimated }
}

impl Context {
    fn start(&self, mut parent: Option<String>) -> Option<i64> {
        for _ in 0..32 {
            let id = parent?;
            let node = self.nodes.iter().rev().find(|node| node.id == id)?;
            if !node.attachment {
                return node.time;
            }
            parent = node.parent.clone();
        }
        None
    }

    fn observe(&mut self, value: &Value) {
        let time = timestamp(value);
        let parent = uuid(value, "parentUuid");
        if let Some(id) = uuid(value, "uuid") {
            self.nodes.push_back(Node {
                id,
                parent: parent.clone(),
                time,
                attachment: value["type"] == "attachment",
            });
            if self.nodes.len() > CONTEXT_LIMIT {
                self.nodes.pop_front();
            }
        }
        if value["type"] != "assistant" {
            return;
        }
        let Some(id) = value.pointer("/message/id").and_then(Value::as_str) else { return };
        if let Some(timing) = self.timings.iter_mut().find(|timing| timing.id == id) {
            timing.end = timing.end.max(time);
            return;
        }
        let first = value.get("apiBlockIndex").is_none_or(|index| index.as_u64() == Some(0));
        let start = if first { self.start(parent) } else { None };
        let first_output = if first
            && value.pointer("/message/content/0/type").and_then(Value::as_str) == Some("thinking")
        {
            value
                .get("thinkingDurationMs")
                .and_then(Value::as_i64)
                .filter(|n| *n > 0)
                .and_then(|duration| time?.checked_sub(duration))
        } else {
            None
        };
        self.timings.push_back(Timing { id: id.into(), start, first_output, end: time });
        if self.timings.len() > CONTEXT_LIMIT {
            self.timings.pop_front();
        }
    }

    fn performance(&self, message: &ClaudeUsageMessage) -> UsagePerformance {
        let mut result = UsagePerformance {
            first_token_ms: None,
            latency_ms: None,
            output_tokens_per_second: None,
        };
        if message.stop_reason.is_none() {
            return result;
        }
        let Some(timing) = self.timings.iter().find(|timing| timing.id == message.message_id)
        else {
            return result;
        };
        let Some((start, end)) = timing.start.zip(timing.end) else { return result };
        let Some(latency) = end
            .checked_sub(start)
            .filter(|n| (MIN_PLAUSIBLE_LATENCY_MS..=MAX_PLAUSIBLE_LATENCY_MS).contains(n))
        else {
            return result;
        };
        result.latency_ms = Some(estimated(latency));
        let first = timing
            .first_output
            .and_then(|first| first.checked_sub(start))
            .filter(|n| *n > 0 && *n <= latency);
        result.first_token_ms = first.map(estimated);
        // Only an evidenced first output permits estimating generation speed.
        if let Some(generation) =
            first.and_then(|first| latency.checked_sub(first)).filter(|n| *n > 0)
        {
            result.output_tokens_per_second =
                message.usage.tokens.output.map(|output| UsageMeasurement {
                    value: output as f64 * 1000.0 / generation as f64,
                    kind: UsageMeasurementKind::Estimated,
                });
        }
        result
    }
}

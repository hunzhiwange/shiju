//! Antigravity usage projection. Paths and identities are resolved by the authorized host.
use super::{
    agy_native,
    cost::{ModelPricing, calculate},
    cursors::{UsageCursorError, UsageCursorKey},
    store::{
        StoredUsageRecord, UsageSaveOutcome, UsageSourceIdentity, UsageStoreError,
        save_usage_record_in_transaction,
    },
};
use rusqlite::{Connection, TransactionBehavior};
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use std::path::Path;
use vw_api_types::{id::ProjectScope, usage::*};

#[derive(Debug)]
pub enum AgyScanError {
    SourceUnavailable,
    Projection(UsageCursorError),
}
impl std::fmt::Display for AgyScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::SourceUnavailable => "Antigravity usage source unavailable",
            Self::Projection(_) => "Antigravity usage projection failed",
        })
    }
}
impl std::error::Error for AgyScanError {}
impl From<UsageCursorError> for AgyScanError {
    fn from(error: UsageCursorError) -> Self {
        Self::Projection(error)
    }
}
impl From<UsageStoreError> for AgyScanError {
    fn from(error: UsageStoreError) -> Self {
        Self::Projection(error.into())
    }
}
fn invalid() -> AgyScanError {
    UsageCursorError::InvalidRecord.into()
}
fn storage(_: rusqlite::Error) -> AgyScanError {
    UsageStoreError::StorageUnavailable.into()
}

#[derive(Debug, Default)]
pub struct AgyScanOutcome {
    pub imported: usize,
    pub replayed: usize,
    pub deferred: usize,
    pub estimated_models: usize,
}

fn known(model: &str) -> Option<String> {
    let model = model.trim();
    (!super::model_names::is_placeholder_pricing_model(model) && !model.contains('\0'))
        .then(|| model.to_owned())
}
fn selection(content: &str) -> Option<String> {
    let rest = &content[content.find("Model Selection")?..];
    let tail = &rest[rest.find(" to ")? + 4..];
    let end = tail
        .find(". ")
        .or_else(|| tail.find('\n'))
        .or_else(|| tail.find('`'))
        .unwrap_or(tail.len());
    known(&tail[..end])
}
fn pricing_id(model: &str) -> String {
    model.trim().to_ascii_lowercase().replace(" (", "-").replace(')', "").replace(' ', "-")
}
fn measured(value: i64) -> UsageMeasurement {
    UsageMeasurement { value: value as f64, kind: UsageMeasurementKind::Measured }
}

/// Rescan rewritten steps using the shared immutable source identities for replay safety.
/// No transcript-only fingerprint shortcut: native metadata can arrive after the export.
/// Missing native DB/individual malformed protobuf fields permit fallback; permission,
/// schema and SQLite failures propagate without writing partial ledger records.
/// Initialize the shared ledger schema before calling; no raw transcript is persisted.
pub fn scan_agy_transcript(
    ledger: &mut Connection,
    scope: &ProjectScope,
    source: &UsageCursorKey,
    transcript: &Path,
    mut lookup_price: impl FnMut(&str) -> Result<Option<ModelPricing>, UsageCursorError>,
) -> Result<AgyScanOutcome, AgyScanError> {
    if source.app != "agy"
        || [&source.target_host, &source.instance_id, &source.file_identity]
            .into_iter()
            .any(|s| s.trim().is_empty() || s.contains('\0'))
    {
        return Err(invalid());
    }
    if transcript.file_name().is_none_or(|name| name != "transcript_full.jsonl")
        || transcript.parent().and_then(Path::file_name).is_none_or(|name| name != "logs")
        || transcript
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .is_none_or(|name| name != ".system_generated")
    {
        return Err(invalid());
    }
    let session = transcript
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(|s| s.to_str())
        .ok_or_else(invalid)?;
    let tx = ledger.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    let settings =
        super::source_settings::load_source_settings(&tx, scope, &source.target_host, &source.app)
            .map_err(|_| UsageStoreError::StorageUnavailable)?;
    if !settings.settings.scan_enabled {
        return Err(UsageCursorError::ScanDisabled.into());
    }
    let rows = crate::sessions::providers::agy::records(transcript)
        .map_err(|_| AgyScanError::SourceUnavailable)?;
    let steps: Vec<_> = rows
        .iter()
        .filter(|r| r["type"] == "PLANNER_RESPONSE" && r["status"] == "DONE")
        .filter_map(|r| r["step_index"].as_u64())
        .collect();
    let native = agy_native::generations_for_transcript(transcript, &steps)
        .map_err(|_| AgyScanError::SourceUnavailable)?;
    let mut current = None;
    let mut outcome = AgyScanOutcome::default();
    for row in &rows {
        if row["type"] == "USER_INPUT" {
            if let Some(model) = row["content"].as_str().and_then(selection) {
                current = Some(model);
            }
            continue;
        }
        if row["type"] != "PLANNER_RESPONSE" || row["status"] != "DONE" {
            continue;
        }
        let step = row["step_index"].as_u64().ok_or_else(invalid)?;
        let generation = native.get(&step);
        let native_model = generation.and_then(|g| g.model.clone());
        let explicit = row["model"].as_str().and_then(known);
        let estimated = native_model.is_none() && explicit.is_none() && current.is_some();
        let model = native_model.or(explicit).or_else(|| current.clone());
        let tokens = UsageTokens {
            input: row["input_tokens"].as_u64(),
            output: row["output_tokens"].as_u64(),
            cache_read: row["cache_read_tokens"].as_u64(),
            cache_creation: row["cache_write_tokens"].as_u64(),
            reasoning: row["reasoning_tokens"].as_u64(),
            cache_creation_1h: None,
            input_semantics: UsageInputSemantics::Fresh,
            reasoning_semantics: UsageReasoningSemantics::IncludedInOutput,
        };
        if ![tokens.input, tokens.output, tokens.cache_read, tokens.cache_creation]
            .into_iter()
            .flatten()
            .any(|n| n > 0)
        {
            continue;
        }
        let Some(timestamp) = crate::sessions::providers::agy::timestamp(row) else {
            outcome.deferred += 1;
            continue;
        };
        let mut pricing = None;
        if let Some(model) = &model {
            for candidate in super::model_names::model_pricing_candidates(&pricing_id(model)) {
                if let Some(price) = lookup_price(&candidate)? {
                    pricing = Some((candidate, price));
                    break;
                }
            }
        }
        let cost = calculate(&tokens, pricing.as_ref().map(|(_, p)| p), Some(Decimal::ONE))
            .map_err(|_| invalid())?;
        let priced = cost.is_some();
        let mut cost = cost
            .map(|c| c.to_usage_cost())
            .unwrap_or(UsageCost { source: UsageCostSource::Unknown, amount_usd: None });
        if estimated && priced {
            cost.source = UsageCostSource::Estimated;
        }
        let timing = generation.and_then(|g| g.timing);
        let identity = UsageSourceIdentity {
            target_host: source.target_host.clone(),
            app: source.app.clone(),
            instance_id: source.instance_id.clone(),
            file_identity: source.file_identity.clone(),
            kind: UsageSourceKind::Native,
            native_session_id: Some(session.to_owned()),
            native_request_id: step.to_string(),
        };
        let id = format!(
            "agy:{:x}",
            Sha256::digest(serde_json::to_vec(&(scope, &identity)).map_err(|_| invalid())?)
        );
        let record = StoredUsageRecord {
            record: UsageRecord {
                id: id.clone(),
                started_at_unix_seconds: UsageUnixSeconds(timestamp / 1000),
                provider: UsageProvider::Unknown,
                sources: vec![UsageSourceRef {
                    source_id: id,
                    kind: UsageSourceKind::Native,
                    app: source.app.clone(),
                    native_session_id: Some(session.to_owned()),
                    native_request_id: Some(step.to_string()),
                }],
                models: UsageModels {
                    request_model: model.clone(),
                    response_model: model,
                    pricing_model: if priced { pricing.map(|(id, _)| id) } else { None },
                    pricing_multiplier: priced.then(|| "1".into()),
                    multiplier_basis: if estimated {
                        Some("estimated: Antigravity step Model Selection".into())
                    } else {
                        priced.then(|| "Antigravity generation pricing".into())
                    },
                },
                status: UsageRequestStatus::Succeeded,
                billable: true,
                tokens: Some(tokens),
                cost,
                performance: UsagePerformance {
                    first_token_ms: timing.map(|t| measured(t.first_token_ms)),
                    latency_ms: timing.map(|t| measured(t.latency_ms)),
                    output_tokens_per_second: None,
                },
                suspected_duplicate_of: None,
            },
            status_code: None,
            failure: None,
        };
        match save_usage_record_in_transaction(&tx, scope, &identity, &record)? {
            UsageSaveOutcome::Inserted { .. } => outcome.imported += 1,
            UsageSaveOutcome::Replayed { .. } => outcome.replayed += 1,
        }
        if estimated {
            outcome.estimated_models += 1;
        }
    }
    tx.commit().map_err(storage)?;
    Ok(outcome)
}

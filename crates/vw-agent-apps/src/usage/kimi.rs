//! Kimi committed usage projection. Full metadata rereads permit delayed step.end
//! enrichment without retaining message bodies in the ledger or parser cursor.
use std::{fs::File, io::{BufRead, BufReader, Read, Seek, SeekFrom}};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use rust_decimal::Decimal;
use serde_json::Value;
use sha2::{Digest, Sha256};
use vw_api_types::{id::ProjectScope, usage::*};
use crate::sessions::providers::kimi_requests::{RequestIndex, RequestInfo, timestamp_ms};
use super::{
    cost::{ModelPricing, calculate},
    cursors::{UsageCursor, UsageCursorError, UsageCursorKey},
    store::{StoredUsageRecord, UsageSourceIdentity, UsageStoreError, UsageSaveOutcome,
        save_usage_record_in_transaction},
};

#[derive(Debug, Default)]
pub struct KimiScanOutcome {
    pub imported: usize,
    pub refreshed: usize,
    pub replayed: usize,
}
fn invalid() -> UsageCursorError { UsageCursorError::InvalidRecord }
fn storage(_: rusqlite::Error) -> UsageCursorError {
    UsageStoreError::StorageUnavailable.into()
}
fn measurement(value: i64) -> UsageMeasurement {
    // The shared reader can fall back to completed-part timestamps, so these
    // projections conservatively identify timing as estimated.
    UsageMeasurement { value: value as f64, kind: UsageMeasurementKind::Estimated }
}
fn completed_ms(started: i64, latency: i64, committed: i64) -> i64 {
    started.saturating_add(latency).min(committed)
}

/// The host supplies an already-authorized wire and stable registry identity.
/// Main wires use committed assistant messages; child wires use step.end usage.
/// Native usage.record/context messages are intentionally excluded.
/// Initialize the shared ledger/cursor schema before calling this function.
pub fn scan_kimi_file(
    connection: &mut Connection,
    scope: &ProjectScope,
    source: &UsageCursorKey,
    file: &mut File,
    session: &str,
    agent: &str,
    mut lookup_price: impl FnMut(&str) -> Result<Option<ModelPricing>, UsageCursorError>,
) -> Result<KimiScanOutcome, UsageCursorError> {
    if source.app != "kimi" || [source.target_host.as_str(), source.instance_id.as_str(),
        source.file_identity.as_str(), session, agent].into_iter()
        .any(|s| s.trim().is_empty() || s.contains('\0')) { return Err(invalid()); }
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > 128 * 1024 * 1024 { return Err(invalid()); }
    file.seek(SeekFrom::Start(0))?;
    let mut requests = if agent == "main" { RequestIndex::default() } else { RequestIndex::for_agent(agent) };
    let mut pending = Vec::new();
    let mut bytes = 0u64;
    let mut lines = 0u64;
    {
        let mut reader = BufReader::new((&mut *file).take(metadata.len()));
        let mut line = Vec::new();
        loop {
            line.clear();
            if reader.read_until(b'\n', &mut line)? == 0 || line.last() != Some(&b'\n') { break; }
            bytes += line.len() as u64;
            lines += 1;
            let Ok(record) = serde_json::from_slice::<Value>(&line) else { continue; };
            requests.observe(&record);
            if record["agentId"].as_str().is_some_and(|id| id != agent) { continue; }
            let (meta, usage) = if agent == "main" {
                if !matches!(record["type"].as_str(), Some("agent.message.appended" | "human.agent.message.appended"))
                    || record.pointer("/message/message/role").and_then(Value::as_str) != Some("assistant") { continue; }
                let meta = &record["message"]["meta"];
                (meta.clone(), meta["usage"].clone())
            } else {
                let event = &record["event"];
                if record["type"] != "context.append_loop_event" || event["type"] != "step.end" { continue; }
                (serde_json::json!({"messageId": event["messageId"], "model":{"provider":"subagent"}}), event["usage"].clone())
            };
            if !usage.is_object() { continue; }
            let counter = |name: &str| -> Result<Option<u64>, UsageCursorError> {
                match usage.get(name) {
                    None | Some(Value::Null) => Ok(None),
                    Some(value) => value.as_u64().map(Some).ok_or_else(invalid),
                }
            };
            let tokens = UsageTokens {
                input: counter("inputOther")?, output: counter("output")?,
                cache_read: counter("inputCacheRead")?, cache_creation: counter("inputCacheCreation")?,
                reasoning: None, cache_creation_1h: None,
                input_semantics: UsageInputSemantics::Fresh,
                reasoning_semantics: UsageReasoningSemantics::IncludedInOutput,
            };
            if ![tokens.input, tokens.output, tokens.cache_read, tokens.cache_creation]
                .into_iter().flatten().any(|n| n > 0) { continue; }
            let Some(committed) = timestamp_ms(&record["time"]) else { continue; };
            let id = meta["messageId"].as_str().filter(|id| !id.is_empty()).map(str::to_owned)
                .unwrap_or_else(|| format!("{:x}", Sha256::digest(record.to_string().as_bytes())));
            // Only metadata and counters survive parsing, never prompts or tools.
            pending.push((id, meta, tokens, committed));
        }
    }
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    let settings = super::source_settings::load_source_settings(&tx, scope, &source.target_host, "kimi")
        .map_err(|_| UsageStoreError::StorageUnavailable)?;
    if !settings.settings.scan_enabled { return Err(UsageCursorError::ScanDisabled); }
    let mut outcome = KimiScanOutcome::default();
    for (id, meta, tokens, committed) in pending {
        let model = requests.model(&meta).map(str::to_owned);
        let info = requests.get(&meta);
        let latency = info.map_or(0, |info| info.latency_ms);
        let first = info.and_then(|info| info.first_token_ms).filter(|n| *n <= latency);
        let pricing = match model.as_deref() { Some(model) => lookup_price(model)?, None => None };
        let cost = calculate(&tokens, pricing.as_ref(), Some(Decimal::ONE)).map_err(|_| invalid())?;
        let priced = cost.is_some();
        let identity = UsageSourceIdentity {
            target_host: source.target_host.clone(), app: source.app.clone(),
            instance_id: source.instance_id.clone(), file_identity: source.file_identity.clone(),
            kind: UsageSourceKind::Native, native_session_id: Some(session.into()),
            native_request_id: format!("{agent}:{id}"),
        };
        let record_id = format!("kimi:{:x}", Sha256::digest(serde_json::to_vec(&(scope, &identity)).map_err(|_| invalid())?));
        let value = StoredUsageRecord {
            record: UsageRecord {
                id: record_id.clone(),
                started_at_unix_seconds: UsageUnixSeconds(info.and_then(|i| i.request_ms)
                    .map_or(committed, |start| completed_ms(start, latency, committed)) / 1000),
                provider: UsageProvider::Unknown,
                sources: vec![UsageSourceRef { source_id: record_id, kind: UsageSourceKind::Native,
                    app: "kimi".into(), native_session_id: Some(session.into()),
                    native_request_id: Some(identity.native_request_id.clone()) }],
                models: UsageModels {
                    request_model: model.clone(), response_model: model.clone(),
                    pricing_model: priced.then_some(model).flatten(),
                    pricing_multiplier: priced.then(|| "1".into()),
                    multiplier_basis: priced.then(|| "Kimi committed usage pricing".into()),
                },
                tokens: Some(tokens), billable: true, status: UsageRequestStatus::Succeeded,
                cost: cost.map(|c| c.to_usage_cost()).unwrap_or(UsageCost { source: UsageCostSource::Unknown, amount_usd: None }),
                performance: UsagePerformance {
                    first_token_ms: first.map(measurement),
                    latency_ms: (latency > 0).then(|| measurement(latency)),
                    output_tokens_per_second: None,
                }, suspected_duplicate_of: None,
            }, status_code: None, failure: None,
        };
        match save_usage_record_in_transaction(&tx, scope, &identity, &value)? {
            UsageSaveOutcome::Inserted { .. } => outcome.imported += 1,
            UsageSaveOutcome::Replayed { record_id } => {
                if refresh_existing(&tx, scope, &identity, &record_id, &value, info, committed)? {
                    outcome.refreshed += 1;
                } else { outcome.replayed += 1; }
            }
        }
    }
    if file.metadata()?.len() != metadata.len() || file.metadata()?.modified()? != metadata.modified()? {
        return Err(UsageCursorError::SourceChangedDuringScan);
    }
    let tail_len = bytes.min(4096);
    file.seek(SeekFrom::Start(bytes - tail_len))?;
    let mut tail = vec![0; tail_len as usize];
    file.read_exact(&mut tail)?;
    let mut fingerprint = Sha256::new();
    fingerprint.update(b"shiju-usage-cursor-v1");
    fingerprint.update(&tail);
    let cursor = UsageCursor { byte_offset: bytes, line_offset: lines,
        tail_fingerprint: fingerprint.finalize().to_vec(), parser_state: None };
    tx.execute("INSERT INTO usage_scan_cursors(cursor_key,payload_json) VALUES (?1,?2)
        ON CONFLICT(cursor_key) DO UPDATE SET payload_json=excluded.payload_json",
        params![serde_json::to_string(&(scope, source)).map_err(|_| invalid())?,
            serde_json::to_string(&cursor).map_err(|_| invalid())?]).map_err(storage)?;
    tx.commit().map_err(storage)?;
    Ok(outcome)
}

/// Receipts outlive retained detail: never resurrect pruned requests. Enrich only
/// metadata with new evidence, preserving richer timing and any native cost.
fn refresh_existing(
    tx: &Transaction<'_>, scope: &ProjectScope, identity: &UsageSourceIdentity,
    record_id: &str, incoming: &StoredUsageRecord, info: Option<&RequestInfo>, committed: i64,
) -> Result<bool, UsageCursorError> {
    let scope_key = serde_json::to_string(&(scope, &identity.target_host)).map_err(|_| invalid())?;
    let payload: Option<String> = tx.query_row(
        "SELECT payload_json FROM usage_records WHERE scope_key=?1 AND record_id=?2",
        params![scope_key, record_id], |row| row.get(0)).optional().map_err(storage)?;
    let Some(payload) = payload else { return Ok(false); };
    let mut old: StoredUsageRecord = serde_json::from_str(&payload).map_err(|_| invalid())?;
    let before = serde_json::to_value(&old).map_err(|_| invalid())?;
    if incoming.record.models.response_model.is_some() {
        old.record.models.request_model = incoming.record.models.request_model.clone();
        old.record.models.response_model = incoming.record.models.response_model.clone();
        if old.record.cost.source != UsageCostSource::Native && incoming.record.cost.source != UsageCostSource::Unknown {
            old.record.cost = incoming.record.cost.clone();
            old.record.models.pricing_model = incoming.record.models.pricing_model.clone();
            old.record.models.pricing_multiplier = incoming.record.models.pricing_multiplier.clone();
            old.record.models.multiplier_basis = incoming.record.models.multiplier_basis.clone();
        }
    }
    let performance = &mut old.record.performance;
    if let Some(latency) = &incoming.record.performance.latency_ms {
        if performance.first_token_ms.is_none() || incoming.record.performance.first_token_ms.is_some() {
            performance.latency_ms = Some(latency.clone());
        }
    }
    if incoming.record.performance.first_token_ms.is_some() {
        performance.first_token_ms = incoming.record.performance.first_token_ms.clone();
    }
    if let Some(start) = info.and_then(|i| i.request_ms) {
        let latency = performance.latency_ms.as_ref().map_or(0, |n| n.value as i64);
        old.record.started_at_unix_seconds = UsageUnixSeconds(completed_ms(start, latency, committed) / 1000);
    }
    if serde_json::to_value(&old).map_err(|_| invalid())? == before { return Ok(false); }
    tx.execute("UPDATE usage_records SET payload_json=?3 WHERE scope_key=?1 AND record_id=?2",
        params![scope_key, record_id, serde_json::to_string(&old).map_err(|_| invalid())?]).map_err(storage)?;
    super::source_settings::advance_statistics_revision(tx, scope, &identity.target_host, "kimi")?;
    Ok(true)
}

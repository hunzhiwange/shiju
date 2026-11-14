//! DSH Web and Desktop share one host-authorized sessions source (`dsh`).
//! Only complete JSONL lines / independent zstd frames advance the cursor.
use std::{collections::BTreeMap, fs::File, io::{Read, Seek, SeekFrom}};
use rusqlite::{Connection, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use vw_api_types::{id::ProjectScope, usage::*};
use super::{
    cursors::{UsageCursor, UsageCursorError, UsageCursorKey, UsageScanOutcome, load_usage_cursor},
    store::{StoredUsageRecord, UsageSourceIdentity, UsageStoreError, save_usage_record_in_transaction},
};

const MAX_BYTES: u64 = 128 * 1024 * 1024;
#[derive(Default, Serialize, Deserialize)]
struct State {
    // String keys allow portable JSON persistence of (turn, step).
    starts: BTreeMap<String, i64>,
}
fn invalid() -> UsageCursorError { UsageCursorError::InvalidRecord }
fn storage(_: rusqlite::Error) -> UsageCursorError { UsageStoreError::StorageUnavailable.into() }
fn fingerprint(bytes: &[u8]) -> Vec<u8> {
    let mut hash = Sha256::new();
    hash.update(b"shiju-usage-cursor-v1");
    hash.update(&bytes[bytes.len().saturating_sub(4096)..]);
    hash.finalize().to_vec()
}

fn file_fingerprint(file: &mut File, end: u64) -> Result<Vec<u8>, UsageCursorError> {
    let length = end.min(4096);
    file.seek(SeekFrom::Start(end - length))?;
    let mut bytes = vec![0; length as usize];
    file.read_exact(&mut bytes)?;
    Ok(fingerprint(&bytes))
}

/// The host supplies the same stable source/instance identity for both entry
/// points, an already-authorized file, and its native session ID. No discovery or
/// client paths are accepted here. Initialize shared ledger/cursor schemas first.
/// `compressed` describes the native file format, not an optional capability.
pub fn scan_dsh_file(
    connection: &mut Connection,
    scope: &ProjectScope,
    source: &UsageCursorKey,
    session_id: &str,
    file: &mut File,
    compressed: bool,
) -> Result<UsageScanOutcome, UsageCursorError> {
    if source.app != "dsh" || [&source.target_host, &source.instance_id, &source.file_identity, session_id]
        .into_iter().any(|v| v.trim().is_empty() || v.contains('\0')) {
        return Err(invalid());
    }
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    let settings = super::source_settings::load_source_settings(&tx, scope, &source.target_host, &source.app)
        .map_err(|_| UsageStoreError::StorageUnavailable)?;
    if !settings.settings.scan_enabled { return Err(UsageCursorError::ScanDisabled); }
    let old = load_usage_cursor(&tx, scope, source)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES { return Err(invalid()); }
    let history_changed = match &old {
        Some(c) if c.byte_offset > metadata.len() => true,
        Some(c) => file_fingerprint(file, c.byte_offset)? != c.tail_fingerprint,
        None => false,
    };
    let base = if history_changed { 0 } else { old.as_ref().map_or(0, |c| c.byte_offset) };
    file.seek(SeekFrom::Start(base))?;
    let mut bytes = Vec::new();
    (&mut *file).take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES { return Err(invalid()); }
    let mut cursor = if history_changed { None } else { old }.unwrap_or(UsageCursor {
        byte_offset: 0, line_offset: 0, tail_fingerprint: vec![], parser_state: None,
    });
    let mut state: State = cursor.parser_state.as_deref().map(serde_json::from_str)
        .transpose().map_err(|_| UsageCursorError::InvalidCursor)?.unwrap_or_default();
    let mut records = 0;
    let mut offset = 0;
    while offset < bytes.len() {
        let (text, consumed) = if compressed {
            let tail = &bytes[offset..];
            let size = match zstd::zstd_safe::find_frame_compressed_size(tail) {
                Ok(size) => size,
                Err(_) if tail.len() < 4 || tail.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) => break,
                Err(_) => return Err(invalid()),
            };
            let decoder = zstd::stream::read::Decoder::new(&tail[..size])?;
            let mut text = Vec::new();
            decoder.take(MAX_BYTES + 1).read_to_end(&mut text)?;
            if text.len() as u64 > MAX_BYTES { return Err(invalid()); }
            // A complete frame with a partial JSONL event must be retried whole.
            if !text.is_empty() && !text.ends_with(b"\n") { break; }
            (text, size)
        } else {
            let Some(end) = bytes[offset..].iter().position(|b| *b == b'\n') else { break; };
            (bytes[offset..offset + end + 1].to_vec(), end + 1)
        };
        for line in text.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
            if !history_changed {
                if let Some((identity, record)) = parse_line(line, &mut state, scope, source, session_id)? {
                    save_usage_record_in_transaction(&tx, scope, &identity, &record)?;
                    records += 1;
                }
            }
            cursor.line_offset += 1;
        }
        offset += consumed;
    }
    if file.metadata()?.len() != metadata.len() || file.metadata()?.modified()? != metadata.modified()? {
        return Err(UsageCursorError::SourceChangedDuringScan);
    }
    // Re-read the consumed prefix to reject rewrites during parsing.
    file.seek(SeekFrom::Start(base))?;
    let mut current = Vec::new();
    (&mut *file).take(offset as u64).read_to_end(&mut current)?;
    if current != bytes[..offset] { return Err(UsageCursorError::SourceChangedDuringScan); }
    if !history_changed && file_fingerprint(file, base)? != cursor.tail_fingerprint && base > 0 {
        return Err(UsageCursorError::SourceChangedDuringScan);
    }
    cursor.byte_offset = base + offset as u64;
    cursor.tail_fingerprint = file_fingerprint(file, cursor.byte_offset)?;
    cursor.parser_state = Some(serde_json::to_string(&state).map_err(|_| invalid())?);
    tx.execute("INSERT INTO usage_scan_cursors(cursor_key,payload_json) VALUES (?1,?2)
        ON CONFLICT(cursor_key) DO UPDATE SET payload_json=excluded.payload_json",
        params![serde_json::to_string(&(scope, source)).map_err(|_| invalid())?,
            serde_json::to_string(&cursor).map_err(|_| invalid())?]).map_err(storage)?;
    tx.commit().map_err(storage)?;
    Ok(UsageScanOutcome { cursor, records, history_changed })
}

fn parse_line(
    line: &[u8], state: &mut State, scope: &ProjectScope, source: &UsageCursorKey, session: &str,
) -> Result<Option<(UsageSourceIdentity, StoredUsageRecord)>, UsageCursorError> {
    if line.iter().all(u8::is_ascii_whitespace) { return Ok(None); }
    let event: Value = serde_json::from_slice(line).map_err(|_| invalid())?;
    let data = &event["data"];
    let step = match (data["turn"].as_u64(), data["step"].as_u64()) {
        (Some(turn), Some(step)) => Some(format!("{turn}:{step}")),
        _ => None,
    };
    if event["type"] == "step/start" {
        let time = event["time"].as_i64().filter(|t| *t >= 0).ok_or_else(invalid)?;
        state.starts.insert(step.ok_or_else(invalid)?, time);
        return Ok(None);
    }
    if event["type"] != "assistant/message" || data.get("usage").is_none() { return Ok(None); }
    let usage = &data["usage"];
    let counter = |name: &str| -> Result<u64, UsageCursorError> {
        match usage.get(name) { None => Ok(0), Some(v) => v.as_u64().ok_or_else(invalid) }
    };
    let input = counter("inputTokens")?;
    let output = counter("outputTokens")?;
    let read = counter("cacheReadTokens")?;
    let write = counter("cacheWriteTokens")?;
    if [input, output, read, write].iter().all(|v| *v == 0) { return Ok(None); }
    let end = event["time"].as_i64().filter(|t| *t >= 0).ok_or_else(invalid)?;
    let start = step.and_then(|key| state.starts.remove(&key));
    let latency = start.and_then(|s| end.checked_sub(s)).filter(|v| *v > 0);
    let first = data["stream"].as_array().into_iter().flatten()
        .flat_map(|v| [v["time"].as_i64(), v["time0"].as_i64()]).flatten().min()
        .zip(start).and_then(|(t, s)| t.checked_sub(s))
        .filter(|v| *v >= 0 && latency.is_some_and(|l| *v <= l));
    let measurement = |v: i64| UsageMeasurement { value: v as f64, kind: UsageMeasurementKind::Estimated };
    let request = data["message"]["id"].as_str().filter(|id| !id.trim().is_empty())
        .map(str::to_owned).unwrap_or_else(|| format!("{:x}", Sha256::digest(event.to_string().as_bytes())));
    let model = data["message"]["source"]["model"].as_str().filter(|m| !m.trim().is_empty()).map(str::to_owned);
    let identity = UsageSourceIdentity {
        target_host: source.target_host.clone(), app: source.app.clone(), instance_id: source.instance_id.clone(),
        file_identity: source.file_identity.clone(), kind: UsageSourceKind::Native,
        native_session_id: Some(session.to_owned()), native_request_id: request.clone(),
    };
    let id = format!("dsh:{:x}", Sha256::digest(serde_json::to_vec(&(scope, &identity)).map_err(|_| invalid())?));
    Ok(Some((identity, StoredUsageRecord {
        record: UsageRecord {
            id: id.clone(), started_at_unix_seconds: UsageUnixSeconds(start.unwrap_or(end) / 1000),
            provider: UsageProvider::Unknown,
            sources: vec![UsageSourceRef { source_id: id, kind: UsageSourceKind::Native, app: "dsh".into(),
                native_session_id: Some(session.into()), native_request_id: Some(request) }],
            models: UsageModels { request_model: model.clone(), response_model: model.clone(),
                pricing_model: model.map(|m| m.split_once(':').map_or(m.as_str(), |(_, bare)| bare).to_owned()),
                pricing_multiplier: None, multiplier_basis: None },
            status: UsageRequestStatus::Succeeded, billable: true,
            tokens: Some(UsageTokens { input: Some(input), output: Some(output), reasoning: None,
                cache_read: Some(read), cache_creation: Some(write), cache_creation_1h: None,
                input_semantics: UsageInputSemantics::Fresh, reasoning_semantics: UsageReasoningSemantics::Unknown }),
            cost: UsageCost { source: UsageCostSource::Unknown, amount_usd: None },
            performance: UsagePerformance { first_token_ms: first.map(measurement), latency_ms: latency.map(measurement),
                output_tokens_per_second: None }, suspected_duplicate_of: None,
        }, status_code: None, failure: None,
    })))
}

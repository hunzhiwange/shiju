//! Gemini usage from host-authorized session snapshots. No scheduling or path grants.
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, TransactionBehavior, params};
use rust_decimal::Decimal;
use serde_json::Value;
use sha2::{Digest, Sha256};
use vw_api_types::{id::ProjectScope, usage::*};

use super::cost::{ModelPricing, calculate};
use super::cursors::{UsageCursor, UsageCursorError, UsageCursorKey, load_usage_cursor};
use super::model_names::model_pricing_candidates;
use super::store::{
    StoredUsageRecord, UsageSaveOutcome, UsageSourceIdentity, UsageStoreError,
    save_usage_record_in_transaction,
};
use crate::sessions::providers::gemini::{is_session_file, parse_session_document};

const MAX_BYTES: u64 = 128 * 1024 * 1024;
const SNAPSHOT_VERSION: &str = "gemini-snapshot-v1";

fn invalid() -> UsageCursorError { UsageCursorError::InvalidRecord }
fn storage(_: rusqlite::Error) -> UsageCursorError {
    UsageStoreError::StorageUnavailable.into()
}

/// Only a host-selected instance root is accepted, never a path from a usage query.
/// Traverse the fixed tmp/<project>/chats shape without following symbolic links.
/// Missing tmp is an empty source; permission and malformed-directory errors propagate.
pub fn collect_gemini_session_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    fn directory(path: &Path) -> std::io::Result<()> {
        let meta = fs::symlink_metadata(path)?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid Gemini directory"));
        }
        Ok(())
    }
    if !root.is_absolute() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid Gemini root"));
    }
    directory(root)?;
    let tmp = root.join("tmp");
    match directory(&tmp) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        result => result?,
    }
    let mut files = Vec::new();
    for project in fs::read_dir(tmp)? {
        let project = project?;
        if !project.file_type()?.is_dir() { continue; }
        let chats = project.path().join("chats");
        match directory(&chats) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            result => result?,
        }
        for entry in fs::read_dir(chats)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() { continue; }
            let path = entry.path();
            if path.file_name().and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("session-")) && is_session_file(&path) {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// Native Gemini input includes cached tokens; thoughts are outside output.
/// Preserve absent counters and u64 values instead of manufacturing zero or narrowing.
pub fn parse_gemini_tokens(value: &Value) -> Option<UsageTokens> {
    value.as_object()?;
    let tokens = UsageTokens {
        input: value["input"].as_u64(), output: value["output"].as_u64(),
        cache_read: value["cached"].as_u64(), reasoning: value["thoughts"].as_u64(),
        cache_creation: None, cache_creation_1h: None,
        input_semantics: UsageInputSemantics::Total,
        reasoning_semantics: UsageReasoningSemantics::SeparateFromOutput,
    };
    [tokens.input, tokens.output, tokens.cache_read, tokens.reasoning]
        .into_iter().flatten().any(|n| n > 0).then_some(tokens)
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct GeminiScanOutcome {
    pub imported: usize,
    pub replayed: usize,
    /// Usage without a stable ID or valid native timestamp is not silently dated now.
    pub deferred: usize,
    pub unchanged: bool,
}

fn label<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field)?.as_str().filter(|s| !s.trim().is_empty() && !s.contains('\0'))
}

fn snapshot(file: &mut File) -> Result<Vec<u8>, UsageCursorError> {
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > MAX_BYTES { return Err(invalid()); }
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES { return Err(invalid()); }
    Ok(bytes)
}

/// Scan an already-open authorized file. `source.file_identity` must be stable for
/// the logical session across .json -> .jsonl migration. Initialize the shared
/// ledger/cursor schemas before calling. Price lookup is exact for each ordered
/// alias and must return errors rather than treating an unavailable store as free.
///
/// Snapshot fingerprints and ledger writes share one transaction. JSON rewrites
/// are reread; existing native identities remain immutable under the shared ledger
/// contract. Rewind never deletes accounted usage. Malformed/partial documents
/// fail without advancing the cursor. No raw message content is persisted.
pub fn scan_gemini_file(
    connection: &mut Connection,
    scope: &ProjectScope,
    source: &UsageCursorKey,
    file: &mut File,
    mut lookup_price: impl FnMut(&str) -> Result<Option<ModelPricing>, UsageCursorError>,
) -> Result<GeminiScanOutcome, UsageCursorError> {
    if source.app != "gemini" || [&source.target_host, &source.instance_id, &source.file_identity]
        .into_iter().any(|s| s.trim().is_empty() || s.contains('\0')) {
        return Err(invalid());
    }
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage)?;
    let settings = super::source_settings::load_source_settings(
        &transaction, scope, &source.target_host, &source.app,
    ).map_err(|_| UsageStoreError::StorageUnavailable)?;
    if !settings.settings.scan_enabled { return Err(UsageCursorError::ScanDisabled); }
    let bytes = snapshot(file)?;
    let fingerprint = Sha256::digest(&bytes).to_vec();
    let old = load_usage_cursor(&transaction, scope, source)?;
    if old.as_ref().is_some_and(|cursor| cursor.parser_state.as_deref() == Some(SNAPSHOT_VERSION)
        && cursor.byte_offset == bytes.len() as u64 && cursor.tail_fingerprint == fingerprint) {
        return Ok(GeminiScanOutcome { unchanged: true, ..Default::default() });
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| invalid())?;
    let document = parse_session_document(text).ok_or_else(invalid)?;
    let session = label(&document, "sessionId").ok_or_else(invalid)?;
    let messages = document["messages"].as_array().ok_or_else(invalid)?;
    let mut outcome = GeminiScanOutcome::default();
    for message in messages {
        if message["type"] != "gemini" { continue; }
        let Some(tokens) = parse_gemini_tokens(&message["tokens"]) else { continue };
        let timestamp = label(message, "timestamp")
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
        let (Some(request), Some(timestamp)) = (label(message, "id"), timestamp) else {
            outcome.deferred += 1;
            continue;
        };
        let model = label(message, "model").map(str::to_owned);
        let mut pricing = None;
        if let Some(model) = &model {
            for candidate in model_pricing_candidates(model) {
                if let Some(price) = lookup_price(&candidate)? {
                    pricing = Some((candidate, price));
                    break;
                }
            }
        }
        let cost = calculate(&tokens, pricing.as_ref().map(|(_, price)| price), Some(Decimal::ONE))
            .map_err(|_| invalid())?;
        let priced = cost.is_some();
        let identity = UsageSourceIdentity {
            target_host: source.target_host.clone(), app: source.app.clone(),
            instance_id: source.instance_id.clone(), file_identity: source.file_identity.clone(),
            kind: UsageSourceKind::Native, native_session_id: Some(session.to_owned()),
            native_request_id: request.to_owned(),
        };
        let encoded = serde_json::to_vec(&(scope, &identity)).map_err(|_| invalid())?;
        let id = format!("gemini_session:{:x}", Sha256::digest(encoded));
        let record = StoredUsageRecord {
            record: UsageRecord {
                id: id.clone(), started_at_unix_seconds: UsageUnixSeconds(timestamp.timestamp()),
                provider: UsageProvider::Unknown,
                sources: vec![UsageSourceRef {
                    source_id: id, kind: UsageSourceKind::Native, app: source.app.clone(),
                    native_session_id: Some(session.to_owned()), native_request_id: Some(request.to_owned()),
                }],
                models: UsageModels {
                    request_model: model.clone(), response_model: model,
                    pricing_model: if priced { pricing.map(|(candidate, _)| candidate) } else { None },
                    pricing_multiplier: priced.then(|| "1".to_owned()),
                    multiplier_basis: priced.then(|| "native Gemini session pricing".to_owned()),
                },
                status: UsageRequestStatus::Unknown, billable: true, tokens: Some(tokens),
                cost: cost.map(|cost| cost.to_usage_cost()).unwrap_or(UsageCost {
                    source: UsageCostSource::Unknown, amount_usd: None,
                }),
                performance: UsagePerformance {
                    first_token_ms: None, latency_ms: None, output_tokens_per_second: None,
                },
                suspected_duplicate_of: None,
            },
            status_code: None, failure: None,
        };
        match save_usage_record_in_transaction(&transaction, scope, &identity, &record)? {
            UsageSaveOutcome::Inserted { .. } => outcome.imported += 1,
            UsageSaveOutcome::Replayed { .. } => outcome.replayed += 1,
        }
    }
    if Sha256::digest(snapshot(file)?).to_vec() != fingerprint {
        return Err(UsageCursorError::SourceChangedDuringScan);
    }
    let cursor = UsageCursor {
        byte_offset: bytes.len() as u64, line_offset: 0, tail_fingerprint: fingerprint,
        parser_state: Some(SNAPSHOT_VERSION.to_owned()),
    };
    let key = serde_json::to_string(&(scope, source)).map_err(|_| invalid())?;
    let payload = serde_json::to_string(&cursor).map_err(|_| invalid())?;
    transaction.execute(
        "INSERT INTO usage_scan_cursors(cursor_key,payload_json) VALUES (?1,?2)
         ON CONFLICT(cursor_key) DO UPDATE SET payload_json=excluded.payload_json",
        params![key, payload],
    ).map_err(storage)?;
    transaction.commit().map_err(storage)?;
    Ok(outcome)
}

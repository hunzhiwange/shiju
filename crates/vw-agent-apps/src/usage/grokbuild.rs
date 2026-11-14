//! GrokBuild turn-completed usage from host-authorized files.
//! Counters accumulate within a prompt, never across distinct prompts.
use std::{collections::BTreeMap, fs::{self, File}, io::{BufRead, BufReader, Read, Seek, SeekFrom}, path::{Path, PathBuf}};

use rusqlite::{Connection, TransactionBehavior, params};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use vw_api_types::{id::ProjectScope, usage::*};
use super::{cost::{ModelPricing, calculate}, cursors::{UsageCursor, UsageCursorError, UsageCursorKey, load_usage_cursor}, store::{StoredUsageRecord, UsageSaveOutcome, UsageSourceIdentity, UsageStoreError, save_usage_record_in_transaction}};

const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;
const SETTLE_SECONDS: i64 = 60;

fn invalid() -> UsageCursorError { UsageCursorError::InvalidRecord }
fn storage(_: rusqlite::Error) -> UsageCursorError { UsageStoreError::StorageUnavailable.into() }

/// `root` must be selected by the host application registry, never a query path.
/// Only sessions and archived_sessions are traversed; symlinks are not followed.
pub fn collect_grokbuild_session_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    fn directory(path: &Path) -> std::io::Result<()> {
        let meta = fs::symlink_metadata(path)?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid GrokBuild root"));
        }
        Ok(())
    }
    fn walk(path: &Path, depth: usize, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() && depth < 16 { walk(&entry.path(), depth + 1, files)?; }
            else if kind.is_file() && entry.file_name() == "updates.jsonl" { files.push(entry.path()); }
        }
        Ok(())
    }
    if !root.is_absolute() { return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid GrokBuild root")); }
    directory(root)?;
    let mut files = Vec::new();
    for name in ["sessions", "archived_sessions"] {
        let path = root.join(name);
        match directory(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            result => result?,
        }
        walk(&path, 0, &mut files)?;
    }
    files.sort();
    Ok(files)
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Counters { input: u64, output: u64, cached: u64, reasoning: u64, ticks: u64 }
impl Counters {
    fn parse(value: &Value) -> Self {
        let n = |key| value.get(key).and_then(Value::as_u64).unwrap_or(0);
        Self { input: n("inputTokens"), output: n("outputTokens"), cached: n("cachedReadTokens"), reasoning: n("reasoningTokens"), ticks: n("costUsdTicks") }
    }
    fn delta(&self, old: &Self) -> Self {
        Self { input: self.input.saturating_sub(old.input), output: self.output.saturating_sub(old.output), cached: self.cached.saturating_sub(old.cached), reasoning: self.reasoning.saturating_sub(old.reasoning), ticks: self.ticks.saturating_sub(old.ticks) }
    }
    fn advance(&mut self, current: &Self) {
        self.input = self.input.max(current.input); self.output = self.output.max(current.output);
        self.cached = self.cached.max(current.cached); self.reasoning = self.reasoning.max(current.reasoning);
        self.ticks = self.ticks.max(current.ticks);
    }
}

#[derive(Default, Serialize, Deserialize)]
struct State { prompts: BTreeMap<String, Counters> }
#[derive(Debug, Default)]
pub struct GrokBuildScanOutcome {
    pub imported: usize,
    pub replayed: usize,
    pub deferred: bool,
    pub skipped_lines: usize,
    pub history_changed: bool,
}

// Stream oversized payloads without retaining them. An incomplete tail, including
// an oversized one, is retried from its beginning rather than losing its boundary.
fn line(reader: &mut impl BufRead) -> std::io::Result<(Vec<u8>, u64, bool)> {
    let mut bytes = Vec::new();
    let mut consumed = 0u64;
    let mut oversized = false;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() { return Ok((bytes, consumed, false)); }
        let end = available.iter().position(|b| *b == b'\n').map(|n| n + 1);
        let count = end.unwrap_or(available.len());
        consumed += count as u64;
        if consumed > MAX_LINE_BYTES as u64 { oversized = true; bytes.clear(); }
        if !oversized { bytes.extend_from_slice(&available[..count]); }
        reader.consume(count);
        if end.is_some() { return Ok((bytes, consumed, true)); }
    }
}
fn fingerprint(file: &mut File, end: u64) -> Result<Vec<u8>, UsageCursorError> {
    let count = end.min(4096);
    file.seek(SeekFrom::Start(end - count))?;
    let mut bytes = vec![0; count as usize]; file.read_exact(&mut bytes)?;
    Ok(Sha256::digest(bytes).to_vec())
}

/// The host supplies scope, stable logical session identity (also across archive
/// moves), and an already-open authorized file. No arbitrary path is opened here.
/// `now` is UTC Unix seconds. Initialize ledger and cursor schemas beforehand.
/// A young event leaves the cursor at its line start, even if mtime never changes.
/// Rewrites pin history without replay; counters and ledger commit atomically.
/// Pricing errors abort, and partial native costs are never presented as exact.
pub fn scan_grokbuild_file(
    connection: &mut Connection, scope: &ProjectScope, source: &UsageCursorKey,
    file: &mut File, now: i64,
    mut lookup_price: impl FnMut(&str) -> Result<Option<ModelPricing>, UsageCursorError>,
) -> Result<GrokBuildScanOutcome, UsageCursorError> {
    if source.app != "grokbuild" { return Err(invalid()); }
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    let settings = super::source_settings::load_source_settings(&transaction, scope, &source.target_host, &source.app)
        .map_err(|_| UsageStoreError::StorageUnavailable)?;
    if !settings.settings.scan_enabled { return Err(UsageCursorError::ScanDisabled); }
    let old = load_usage_cursor(&transaction, scope, source)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() { return Err(invalid()); }
    let length = metadata.len();
    let changed = match &old {
        Some(c) => c.byte_offset > length || fingerprint(file, c.byte_offset)? != c.tail_fingerprint,
        None => false,
    };
    let mut cursor = old.clone().unwrap_or(UsageCursor { byte_offset: 0, line_offset: 0, tail_fingerprint: Vec::new(), parser_state: None });
    let mut state: State = cursor.parser_state.as_deref().map(serde_json::from_str).transpose()
        .map_err(|_| UsageCursorError::InvalidCursor)?.unwrap_or_default();
    if changed { cursor.byte_offset = 0; cursor.line_offset = 0; }
    let mut result = GrokBuildScanOutcome { history_changed: changed, ..Default::default() };
    file.seek(SeekFrom::Start(cursor.byte_offset))?;
    {
        let mut reader = BufReader::new((&mut *file).take(length - cursor.byte_offset));
        loop {
            let (bytes, consumed, complete) = line(&mut reader)?;
            if !complete { break; }
            if !changed {
                let value = serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null);
                let update = &value["params"]["update"];
                if value["method"] == "_x.ai/session/update" && update["sessionUpdate"] == "turn_completed" && update["usage"].is_object() {
                    let timestamp = value["timestamp"].as_i64().or_else(|| value["timestamp"].as_str()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map(|t| t.timestamp())).ok_or_else(invalid)?;
                    if now.saturating_sub(timestamp) < SETTLE_SECONDS { result.deferred = true; break; }
                    let prompt = update["prompt_id"].as_str().filter(|s| !s.is_empty() && !s.contains('\0')).ok_or_else(invalid)?;
                    let usage = &update["usage"];
                    let models: Vec<(&str, &Value)> = usage["modelUsage"].as_object().filter(|m| !m.is_empty())
                        .map(|m| m.iter().map(|(k,v)| (k.as_str(),v)).collect()).unwrap_or_else(|| vec![("unknown", usage)]);
                    for (model, value) in models {
                        let key = serde_json::to_string(&(prompt, model)).map_err(|_| invalid())?;
                        let current = Counters::parse(value);
                        let previous = state.prompts.entry(key).or_default();
                        let delta = current.delta(previous);
                        previous.advance(&current);
                        if delta.input == 0 && delta.output == 0 && delta.cached == 0 { continue; }
                        let tokens = UsageTokens { input: Some(delta.input), output: Some(delta.output), cache_read: Some(delta.cached), reasoning: Some(delta.reasoning), cache_creation: None, cache_creation_1h: None, input_semantics: UsageInputSemantics::Total, reasoning_semantics: UsageReasoningSemantics::IncludedInOutput };
                        let price = lookup_price(model)?;
                        let calculated = calculate(&tokens, price.as_ref(), Some(Decimal::ONE)).map_err(|_| invalid())?;
                        let partial = usage["costIsPartial"].as_bool() == Some(true) || value["costIsPartial"].as_bool() == Some(true);
                        let cost = if !partial && delta.ticks > 0 {
                            UsageCost { source: UsageCostSource::Native, amount_usd: Some((Decimal::from(delta.ticks) / Decimal::from(10_000_000_000u64)).normalize().to_string()) }
                        } else { calculated.map(|c| c.to_usage_cost()).unwrap_or(UsageCost { source: UsageCostSource::Unknown, amount_usd: None }) };
                        let native_id = serde_json::to_string(&(prompt, model, &*previous)).map_err(|_| invalid())?;
                        let identity = UsageSourceIdentity { target_host: source.target_host.clone(), app: source.app.clone(), instance_id: source.instance_id.clone(), file_identity: source.file_identity.clone(), kind: UsageSourceKind::Native, native_session_id: Some(source.file_identity.clone()), native_request_id: native_id.clone() };
                        let id = format!("grok_session:{:x}", Sha256::digest(serde_json::to_vec(&(scope, &identity)).map_err(|_| invalid())?));
                        let priced = price.is_some() && cost.source == UsageCostSource::Calculated;
                        let record = StoredUsageRecord { record: UsageRecord {
                            id: id.clone(), started_at_unix_seconds: UsageUnixSeconds(timestamp), provider: UsageProvider::Unknown,
                            sources: vec![UsageSourceRef { source_id: id, kind: UsageSourceKind::Native, app: source.app.clone(), native_session_id: Some(source.file_identity.clone()), native_request_id: Some(native_id) }],
                            models: UsageModels { request_model: Some(model.into()), response_model: Some(model.into()), pricing_model: priced.then(|| model.into()), pricing_multiplier: priced.then(|| "1".into()), multiplier_basis: priced.then(|| "native GrokBuild turn pricing".into()) },
                            status: UsageRequestStatus::Unknown, billable: true, tokens: Some(tokens), cost,
                            performance: UsagePerformance { first_token_ms: None, latency_ms: None, output_tokens_per_second: None }, suspected_duplicate_of: None,
                        }, status_code: None, failure: None };
                        match save_usage_record_in_transaction(&transaction, scope, &identity, &record)? {
                            UsageSaveOutcome::Inserted { .. } => result.imported += 1,
                            UsageSaveOutcome::Replayed { .. } => result.replayed += 1,
                        }
                    }
                } else if bytes.is_empty() || value.is_null() { result.skipped_lines += 1; }
            }
            cursor.byte_offset += consumed; cursor.line_offset += 1;
        }
    }
    if file.metadata()?.len() != length || file.metadata()?.modified()? != metadata.modified()? { return Err(UsageCursorError::SourceChangedDuringScan); }
    if !changed {
        if let Some(old) = old {
            if fingerprint(file, old.byte_offset)? != old.tail_fingerprint { return Err(UsageCursorError::SourceChangedDuringScan); }
        }
    }
    cursor.tail_fingerprint = fingerprint(file, cursor.byte_offset)?;
    cursor.parser_state = Some(serde_json::to_string(&state).map_err(|_| invalid())?);
    transaction.execute("INSERT INTO usage_scan_cursors(cursor_key,payload_json) VALUES (?1,?2) ON CONFLICT(cursor_key) DO UPDATE SET payload_json=excluded.payload_json",
        params![serde_json::to_string(&(scope, source)).map_err(|_| invalid())?, serde_json::to_string(&cursor).map_err(|_| invalid())?]).map_err(storage)?;
    transaction.commit().map_err(storage)?;
    Ok(result)
}

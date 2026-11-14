//! Pi/OMP main-session usage projection. The host resolves authorized files and binds
//! scope/file identity before saving; no discovery or child-session traversal here.
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::str::FromStr;

use rust_decimal::Decimal;
use serde_json::Value;
use sha2::{Digest, Sha256};
use vw_api_types::usage::{
    UsageCost, UsageCostSource, UsageInputSemantics, UsageProvider,
    UsageReasoningSemantics, UsageTokens, UsageUnixSeconds,
};

const MAX_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ENTRIES: usize = 500_000;

/// Shared JSONL engine with application-specific carriers and request identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor { Pi, Omp }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiUsageMessage {
    pub flavor: Flavor,
    pub request_id: String,
    pub session_id: String,
    pub timestamp: Option<UsageUnixSeconds>,
    pub provider: UsageProvider,
    pub request_model: String,
    pub response_model: String,
    pub tokens: UsageTokens,
    pub cost: UsageCost,
    /// Fixed status only: never copy upstream error payloads into the ledger.
    pub failed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiFileRevision {
    pub modified_nanos: Option<u128>,
    pub file_size: u64,
    pub tail_fingerprint: Vec<u8>,
}

#[derive(Debug)]
pub struct PiUsageSnapshot {
    pub revision: PiFileRevision,
    pub messages: Vec<PiUsageMessage>,
    pub incomplete_tail: bool,
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid or oversized Pi session")
}

pub(crate) fn revision(file: &mut File) -> io::Result<PiFileRevision> {
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > MAX_BYTES { return Err(invalid()); }
    let size = meta.len();
    file.seek(SeekFrom::Start(size.saturating_sub(4096)))?;
    let mut tail = Vec::new();
    Read::by_ref(file).take(4096).read_to_end(&mut tail)?;
    Ok(PiFileRevision {
        modified_nanos: meta.modified().ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|time| time.as_nanos()),
        file_size: size,
        tail_fingerprint: Sha256::digest(&tail).to_vec(),
    })
}

/// Apply the complementary ownership rule before parsing a shared store.
pub fn own_files(files: Vec<std::path::PathBuf>, shared_with_omp: bool) -> io::Result<Vec<std::path::PathBuf>> {
    super::omp::select_files(files, shared_with_omp, false)
}

/// Revisions detect same-size repairs and deferred tails. A changed revision may
/// be parsed again; request IDs never depend on revision, offsets or usage totals.
/// Persist them through the shared ledger's durable source identity deduplication.
pub fn parse_file(path: &Path) -> io::Result<PiUsageSnapshot> {
    parse_file_for(path, Flavor::Pi)
}

pub(crate) fn parse_file_for(path: &Path, flavor: Flavor) -> io::Result<PiUsageSnapshot> {
    let mut file = File::open(path)?;
    let before = revision(&mut file)?;
    file.seek(SeekFrom::Start(0))?;
    let (messages, incomplete_tail) = parse_reader_for(BufReader::new((&mut file).take(before.file_size)), flavor)?;
    if revision(&mut file)? != before {
        return Err(io::Error::new(io::ErrorKind::Interrupted, "Pi session changed during scan"));
    }
    Ok(PiUsageSnapshot { revision: before, messages, incomplete_tail })
}

/// Only direct carriers are visited; retained snapshots and child tool details
/// must never be recursively counted. Partial final lines wait for the next scan.
pub fn parse_reader(reader: impl BufRead) -> io::Result<(Vec<PiUsageMessage>, bool)> {
    parse_reader_for(reader, Flavor::Pi)
}

pub(crate) fn parse_reader_for(mut reader: impl BufRead, flavor: Flavor) -> io::Result<(Vec<PiUsageMessage>, bool)> {
    let mut session: Option<String> = None;
    let mut session_time = None;
    let mut messages = BTreeMap::new();
    let mut line = Vec::new();
    let mut bytes = 0u64;
    let mut entries = 0usize;
    let mut incomplete = false;
    let mut first_json = true;
    loop {
        line.clear();
        // Bound allocations even for a malformed file with no newlines.
        let read = Read::by_ref(&mut reader).take(MAX_BYTES.saturating_sub(bytes) + 1)
            .read_until(b'\n', &mut line)?;
        if read == 0 { break; }
        bytes += read as u64;
        entries += 1;
        if bytes > MAX_BYTES || entries > MAX_ENTRIES + 1 { return Err(invalid()); }
        if line.last() != Some(&b'\n') { incomplete = true; break; }
        let Ok(entry) = serde_json::from_slice::<Value>(&line) else { continue };
        if first_json {
            first_json = false;
            if flavor == Flavor::Omp && entry["type"] == "title" { continue; }
        }
        if session.is_none() {
            if entry["type"] != "session" { return Err(invalid()); }
            session = entry.get("id").and_then(Value::as_str)
                .filter(|id| !id.trim().is_empty() && !id.contains('\0')).map(str::to_owned);
            if session.is_none() { return Err(invalid()); }
            session_time = entry.get("timestamp").and_then(timestamp);
            continue;
        }
        let (kind, carrier) = match entry["type"].as_str() {
            Some("message") => match entry["message"]["role"].as_str() {
                Some("assistant") => ("assistant", &entry["message"]),
                Some("toolResult") => ("tool_result", &entry["message"]),
                _ => continue,
            },
            Some("model_usage") if flavor == Flavor::Omp => ("model_usage", &entry),
            Some("compaction") => ("compaction", &entry),
            Some("branch_summary") => ("branch_summary", &entry),
            _ => continue,
        };
        let Some(usage) = carrier.get("usage").filter(|u| u.is_object()) else { continue };
        let tokens = UsageTokens {
            input: usage["input"].as_u64(), output: usage["output"].as_u64(),
            cache_read: usage["cacheRead"].as_u64(), cache_creation: usage["cacheWrite"].as_u64(),
            reasoning: usage["reasoning"].as_u64().or_else(|| {
                if flavor == Flavor::Omp { usage["reasoningTokens"].as_u64() } else { None }
            }), cache_creation_1h: None,
            input_semantics: UsageInputSemantics::Fresh,
            reasoning_semantics: UsageReasoningSemantics::IncludedInOutput,
        };
        let attributed = matches!(kind, "assistant" | "model_usage");
        let failed = attributed && matches!(carrier["stopReason"].as_str(), Some("error" | "aborted"));
        let amount = native_cost(&usage["cost"]);
        if !failed && ![tokens.input, tokens.output, tokens.cache_read, tokens.cache_creation]
            .into_iter().flatten().any(|n| n > 0) && !amount.is_some_and(|n| n > Decimal::ZERO) { continue; }
        let session_id = session.as_ref().ok_or_else(invalid)?.clone();
        // Canonical JSON fallback supports old records without entry IDs. Native
        // IDs deliberately exclude mutable token/cost snapshots.
        let (request_tag, legacy_tag, prefix) = match flavor {
            Flavor::Pi => ("pi-request-v1", "pi-legacy-v1", "pi_session"),
            Flavor::Omp => ("omp-request-v1", "omp-legacy-v1", "omp_session"),
        };
        let identity = match entry.get("id").and_then(Value::as_str).filter(|id| !id.is_empty()) {
            Some(id) => serde_json::to_vec(&(request_tag, &session_id, kind, id)),
            None => serde_json::to_vec(&(legacy_tag, &session_id, kind, canonical_json(&entry))),
        }.map_err(|_| invalid())?;
        let request_id = format!("{prefix}:{:x}", Sha256::digest(identity));
        let request_model = if attributed { label(&carrier["model"]).unwrap_or_else(|| "unknown".into()) } else { "unknown".into() };
        let message = PiUsageMessage {
            flavor, request_id: request_id.clone(), session_id,
            timestamp: entry.get("timestamp").and_then(timestamp)
                .or_else(|| carrier.get("timestamp").and_then(timestamp)).or(session_time),
            provider: if attributed { label(&carrier["provider"]).map(|provider_id| UsageProvider::Known { provider_id }).unwrap_or(UsageProvider::Unknown) } else { UsageProvider::Unknown },
            response_model: if attributed { label(&carrier["responseModel"]).unwrap_or_else(|| request_model.clone()) } else { "unknown".into() },
            request_model, tokens, failed,
            cost: UsageCost { source: if amount.is_some() { UsageCostSource::Native } else { UsageCostSource::Unknown }, amount_usd: amount.map(|n| n.normalize().to_string()) },
        };
        messages.insert(request_id, message);
    }
    if session.is_none() && !incomplete { return Err(invalid()); }
    Ok((messages.into_values().collect(), incomplete))
}

fn label(value: &Value) -> Option<String> {
    let text = value.as_str()?.trim();
    if text.is_empty() { return None; }
    let mut end = text.len().min(512);
    while !text.is_char_boundary(end) { end -= 1; }
    Some(text[..end].to_owned())
}

fn decimal(value: &Value) -> Option<Decimal> {
    let raw = match value { Value::String(s) => s.clone(), Value::Number(n) => n.to_string(), _ => return None };
    Decimal::from_str(&raw).or_else(|_| Decimal::from_scientific(&raw)).ok().filter(|n| *n >= Decimal::ZERO)
}

fn native_cost(value: &Value) -> Option<Decimal> {
    if let Some(total) = decimal(&value["total"]) { return Some(total); }
    let mut total = Decimal::ZERO;
    let mut known = false;
    for key in ["input", "output", "cacheRead", "cacheWrite"] {
        if let Some(part) = decimal(&value[key]) { total = total.checked_add(part)?; known = true; }
    }
    known.then_some(total)
}

fn timestamp(value: &Value) -> Option<UsageUnixSeconds> {
    let millis = if let Some(n) = value.as_i64() {
        if (-100_000_000_000..=100_000_000_000).contains(&n) { n.checked_mul(1000)? } else { n }
    } else {
        chrono::DateTime::parse_from_rfc3339(value.as_str()?).ok()?.timestamp_millis()
    };
    (-62_167_219_200_000..=253_402_300_799_999).contains(&millis)
        .then_some(UsageUnixSeconds(millis.div_euclid(1000)))
}

// Object order is not semantic, including when another workspace dependency
// enables serde_json's preserve_order feature.
pub(crate) fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(fields) => {
            let sorted: BTreeMap<_, _> = fields.iter()
                .map(|(key, value)| (key.clone(), canonical_json(value))).collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical_json).collect()),
        other => other.clone(),
    }
}

//! Incremental projection of MCode's committed usage, through its shared read-only reader.
use super::{
    cost::{ModelPricing, calculate},
    cursors::{UsageCursor, UsageCursorError, UsageCursorKey, load_usage_cursor},
    store::{StoredUsageRecord, UsageSaveOutcome, UsageSourceIdentity, UsageStoreError,
        save_usage_record_in_transaction},
};
use rusqlite::{Connection, TransactionBehavior, params};
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use std::path::Path;
use vw_api_types::{id::ProjectScope, usage::*};

#[derive(Debug)]
pub enum MCodeScanError {
    SourceUnavailable,
    Cursor(UsageCursorError),
}
impl std::fmt::Display for MCodeScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::SourceUnavailable => "MCode usage source unavailable",
            Self::Cursor(_) => "MCode usage projection failed",
        })
    }
}
impl std::error::Error for MCodeScanError {}
impl From<UsageCursorError> for MCodeScanError {
    fn from(error: UsageCursorError) -> Self { Self::Cursor(error) }
}
impl From<UsageStoreError> for MCodeScanError {
    fn from(error: UsageStoreError) -> Self { Self::Cursor(error.into()) }
}
fn invalid() -> MCodeScanError { UsageCursorError::InvalidRecord.into() }
fn storage(_: rusqlite::Error) -> MCodeScanError { UsageStoreError::StorageUnavailable.into() }
fn source_error(_: rusqlite::Error) -> MCodeScanError { MCodeScanError::SourceUnavailable }

#[derive(Debug, Default)]
pub struct MCodeScanOutcome {
    pub imported: usize,
    pub replayed: usize,
}

/// `data_root` and stable identity must come from the authorized host registry.
/// Source snapshot excludes uncommitted rows; ledger and ID watermark commit together.
/// The caller initializes the shared ledger and cursor schemas before scanning.
pub fn scan_mcode_database(
    ledger: &mut Connection,
    scope: &ProjectScope,
    source: &UsageCursorKey,
    data_root: &Path,
    mut lookup_price: impl FnMut(&str) -> Result<Option<ModelPricing>, UsageCursorError>,
) -> Result<MCodeScanOutcome, MCodeScanError> {
    if source.app != "mcode" || [&source.target_host, &source.instance_id, &source.file_identity]
        .into_iter().any(|s| s.trim().is_empty() || s.contains('\0')) {
        return Err(invalid());
    }
    let native = crate::sessions::providers::mcode::open_database(data_root)
        .map_err(|_| MCodeScanError::SourceUnavailable)?;
    let Some(mut native) = native else { return Ok(MCodeScanOutcome::default()); };
    let snapshot = native.transaction().map_err(source_error)?;
    let tx = ledger.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    let settings = super::source_settings::load_source_settings(&tx, scope, &source.target_host, &source.app)
        .map_err(|_| UsageStoreError::StorageUnavailable)?;
    if !settings.settings.scan_enabled { return Err(UsageCursorError::ScanDisabled.into()); }
    let old = load_usage_cursor(&tx, scope, source)?;
    let mut watermark: i64 = match old {
        Some(cursor) => cursor.parser_state.as_deref().ok_or_else(invalid)?.parse()
            .map_err(|_| UsageCursorError::InvalidCursor)?,
        None => 0,
    };
    if watermark < 0 { return Err(UsageCursorError::InvalidCursor.into()); }
    let mut query = snapshot.prepare(
        "SELECT id,session_id,model,ts,input_tokens,output_tokens,reasoning_tokens,
         cache_read_tokens,cache_write_tokens,cost_usd FROM local_runtime_token_usage
         WHERE id > ?1 ORDER BY id"
    ).map_err(source_error)?;
    let mut rows = query.query([watermark]).map_err(source_error)?;
    let mut outcome = MCodeScanOutcome::default();
    while let Some(row) = rows.next().map_err(source_error)? {
        let id: i64 = row.get(0).map_err(source_error)?;
        let session: String = row.get(1).map_err(source_error)?;
        let ts: i64 = row.get(3).map_err(source_error)?;
        if session.trim().is_empty() || session.contains('\0') || ts < 0 { return Err(invalid()); }
        let native_model: Option<String> = row.get(2).map_err(source_error)?;
        let model = match native_model.filter(|v| !v.trim().is_empty()) {
            Some(model) => Some(model.split_once('/').map_or(model.as_str(), |(_, m)| m).to_owned()),
            None => history_model(&snapshot, &session, ts)?,
        };
        let counter = |index| row.get::<_, u64>(index).map_err(source_error);
        let reasoning = counter(6)?;
        let tokens = UsageTokens {
            input: Some(counter(4)?),
            output: Some(counter(5)?.checked_add(reasoning).ok_or_else(invalid)?),
            reasoning: Some(reasoning),
            cache_read: Some(counter(7)?), cache_creation: Some(counter(8)?),
            cache_creation_1h: None,
            input_semantics: UsageInputSemantics::Fresh,
            reasoning_semantics: UsageReasoningSemantics::IncludedInOutput,
        };
        let native_cost: Option<f64> = row.get(9).map_err(source_error)?;
        let native_cost = native_cost.filter(|v| v.is_finite() && *v >= 0.0)
            .and_then(|v| v.to_string().parse::<Decimal>().ok());
        let pricing = if native_cost.is_none() {
            match model.as_deref() { Some(m) => lookup_price(m)?, None => None }
        } else { None };
        let calculated = calculate(&tokens, pricing.as_ref(), Some(Decimal::ONE)).map_err(|_| invalid())?;
        let priced = calculated.is_some();
        let cost = match native_cost {
            Some(amount) => UsageCost { source: UsageCostSource::Native, amount_usd: Some(amount.normalize().to_string()) },
            None => calculated.map(|c| c.to_usage_cost()).unwrap_or(UsageCost {
                source: UsageCostSource::Unknown, amount_usd: None,
            }),
        };
        let identity = UsageSourceIdentity {
            target_host: source.target_host.clone(), app: source.app.clone(),
            instance_id: source.instance_id.clone(), file_identity: source.file_identity.clone(),
            kind: UsageSourceKind::Native, native_session_id: Some(session.clone()),
            native_request_id: id.to_string(),
        };
        let record_id = format!("mcode:{:x}", Sha256::digest(serde_json::to_vec(&(scope, &identity)).map_err(|_| invalid())?));
        let record = StoredUsageRecord {
            record: UsageRecord {
                id: record_id.clone(), started_at_unix_seconds: UsageUnixSeconds(ts / 1000),
                provider: UsageProvider::Unknown,
                sources: vec![UsageSourceRef {
                    source_id: record_id, kind: UsageSourceKind::Native, app: source.app.clone(),
                    native_session_id: Some(session), native_request_id: Some(id.to_string()),
                }],
                models: UsageModels {
                    request_model: model.clone(), response_model: model.clone(),
                    pricing_model: if priced { model } else { None },
                    pricing_multiplier: priced.then(|| "1".into()),
                    multiplier_basis: priced.then(|| "MCode pricing".into()),
                },
                status: UsageRequestStatus::Succeeded, billable: true, tokens: Some(tokens), cost,
                performance: UsagePerformance { first_token_ms: None, latency_ms: None, output_tokens_per_second: None },
                suspected_duplicate_of: None,
            }, status_code: None, failure: None,
        };
        match save_usage_record_in_transaction(&tx, scope, &identity, &record)? {
            UsageSaveOutcome::Inserted { .. } => outcome.imported += 1,
            UsageSaveOutcome::Replayed { .. } => outcome.replayed += 1,
        }
        watermark = id;
    }
    let cursor = UsageCursor { byte_offset: 0, line_offset: 0, tail_fingerprint: vec![0; 32], parser_state: Some(watermark.to_string()) };
    tx.execute("INSERT INTO usage_scan_cursors(cursor_key,payload_json) VALUES (?1,?2)
        ON CONFLICT(cursor_key) DO UPDATE SET payload_json=excluded.payload_json",
        params![serde_json::to_string(&(scope, source)).map_err(|_| invalid())?, serde_json::to_string(&cursor).map_err(|_| invalid())?]).map_err(storage)?;
    tx.commit().map_err(storage)?;
    Ok(outcome)
}

/// Recover the model from the exact assistant timestamp, preferring migrated rows.
/// Absent optional history tables are supported; malformed existing history is an error.
fn history_model(db: &Connection, session: &str, ts: i64) -> Result<Option<String>, MCodeScanError> {
    for (table, sql) in [
        ("local_runtime_pi_history_rows", "SELECT json_extract(data_json,'$.model') FROM local_runtime_pi_history_rows WHERE session_id=?1 AND created_at_ms=?2 AND role='assistant' ORDER BY id"),
        ("local_runtime_messages", "SELECT json_extract(message.value,'$.model') FROM local_runtime_messages,json_each(pi_history_json) AS message WHERE session_id=?1 AND json_extract(message.value,'$.role')='assistant' AND json_extract(message.value,'$.timestamp')=?2"),
    ] {
        let exists: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)", [table], |r| r.get(0)).map_err(source_error)?;
        if !exists { continue; }
        let mut query = db.prepare(sql).map_err(source_error)?;
        let mut rows = query.query(params![session, ts]).map_err(source_error)?;
        while let Some(row) = rows.next().map_err(source_error)? {
            let model: Option<String> = row.get(0).map_err(source_error)?;
            if let Some(model) = model.filter(|v| !v.trim().is_empty()) { return Ok(Some(model)); }
        }
    }
    Ok(None)
}

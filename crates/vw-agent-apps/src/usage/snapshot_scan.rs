//! Transactional persistence for the existing Codex and Pi/OMP projections.
use std::path::{Path, PathBuf};
use rusqlite::{Connection, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use vw_api_types::{ProjectScope, usage::*};
use super::{cursors::{UsageCursorError, UsageCursorKey}, store::{self, StoredUsageRecord, UsageSourceIdentity}};

pub fn scan_codex(connection: &mut Connection, scope: &ProjectScope, key: &UsageCursorKey,
    path: &Path, files: Vec<PathBuf>) -> Result<usize, UsageCursorError> {
    if key.app != "codex" { return Err(UsageCursorError::InvalidRecord); }
    let mut lineage = super::codex_lineage::CodexLineage::new(files)?;
    let records = lineage.project(path)?.into_iter().map(|event| {
        let time = event.timestamp.as_deref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|t| UsageUnixSeconds(t.timestamp())).ok_or(UsageCursorError::InvalidRecord)?;
        let performance = event.timing.performance(&event.usage.tokens);
        Ok((event.session_id, format!("line:{}", event.line_number), UsageRecord {
            id: String::new(), started_at_unix_seconds: time, provider: UsageProvider::Unknown,
            sources: Vec::new(), models: UsageModels { request_model: event.request_model,
                response_model: event.usage.response_model.clone(), pricing_model: event.pricing_model,
                pricing_multiplier: None, multiplier_basis: None },
            status: UsageRequestStatus::Succeeded, billable: event.usage.has_billable_tokens(),
            tokens: Some(event.usage.tokens), cost: UsageCost { source: UsageCostSource::Unknown, amount_usd: None },
            performance, suspected_duplicate_of: None,
        }))
    }).collect::<Result<Vec<_>, UsageCursorError>>()?;
    commit(connection, scope, key, records)
}

pub fn scan_pi(connection: &mut Connection, scope: &ProjectScope, key: &UsageCursorKey,
    path: &Path) -> Result<usize, UsageCursorError> {
    let snapshot = match key.app.as_str() {
        "pi" => super::pi::parse_file(path)?, "omp" => super::omp::parse_file(path)?,
        _ => return Err(UsageCursorError::InvalidRecord),
    };
    let records = snapshot.messages.into_iter().map(|message| {
        let time = message.timestamp.ok_or(UsageCursorError::InvalidRecord)?;
        let billable = [&message.tokens.input, &message.tokens.output, &message.tokens.reasoning,
            &message.tokens.cache_read, &message.tokens.cache_creation].into_iter().any(|n| n.is_some_and(|n| n > 0));
        Ok((Some(message.session_id), message.request_id, UsageRecord {
            id: String::new(), started_at_unix_seconds: time, provider: message.provider,
            sources: Vec::new(), models: UsageModels { request_model: Some(message.request_model),
                response_model: Some(message.response_model), pricing_model: None,
                pricing_multiplier: None, multiplier_basis: None },
            status: if message.failed { UsageRequestStatus::Failed } else { UsageRequestStatus::Succeeded },
            billable, tokens: Some(message.tokens), cost: message.cost,
            performance: UsagePerformance { first_token_ms: None, latency_ms: None, output_tokens_per_second: None },
            suspected_duplicate_of: None,
        }))
    }).collect::<Result<Vec<_>, UsageCursorError>>()?;
    commit(connection, scope, key, records)
}

fn commit(connection: &mut Connection, scope: &ProjectScope, key: &UsageCursorKey,
    records: Vec<(Option<String>, String, UsageRecord)>) -> Result<usize, UsageCursorError> {
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| UsageCursorError::Storage(store::UsageStoreError::StorageUnavailable))?;
    let settings = super::source_settings::load_source_settings(&tx, scope, &key.target_host, &key.app)
        .map_err(|_| UsageCursorError::Storage(store::UsageStoreError::StorageUnavailable))?;
    if !settings.settings.scan_enabled { return Err(UsageCursorError::ScanDisabled); }
    let mut imported = 0;
    for (session, request, mut record) in records {
        let identity = UsageSourceIdentity { target_host: key.target_host.clone(), app: key.app.clone(),
            instance_id: key.instance_id.clone(), file_identity: key.file_identity.clone(),
            kind: UsageSourceKind::Native, native_session_id: session.clone(), native_request_id: request.clone() };
        let encoded = serde_json::to_vec(&identity).map_err(|_| UsageCursorError::InvalidRecord)?;
        record.id = format!("{}:{:x}", key.app, Sha256::digest(encoded));
        record.sources.push(UsageSourceRef { source_id: record.id.clone(), kind: UsageSourceKind::Native,
            app: key.app.clone(), native_session_id: session, native_request_id: Some(request) });
        let stored = StoredUsageRecord { record, status_code: None, failure: None };
        if matches!(store::save_usage_record_in_transaction(&tx, scope, &identity, &stored)?,
            store::UsageSaveOutcome::Inserted { .. }) { imported += 1; }
    }
    // Full snapshot replay uses durable request receipts as its incremental waterline.
    // Never delete receipts on rewrites or when no new requests were observed.
    let cursor_key = serde_json::to_string(&(scope, key)).map_err(|_| UsageCursorError::InvalidRecord)?;
    let cursor = super::cursors::UsageCursor { byte_offset: 0, line_offset: 0,
        tail_fingerprint: Vec::new(), parser_state: Some("snapshot-receipts-v1".into()) };
    let payload = serde_json::to_string(&cursor).map_err(|_| UsageCursorError::InvalidRecord)?;
    tx.execute("INSERT INTO usage_scan_cursors(cursor_key,payload_json) VALUES (?1,?2)
        ON CONFLICT(cursor_key) DO UPDATE SET payload_json=excluded.payload_json", params![cursor_key,payload])
        .map_err(|_| UsageCursorError::Storage(store::UsageStoreError::StorageUnavailable))?;
    tx.commit().map_err(|_| UsageCursorError::Storage(store::UsageStoreError::StorageUnavailable))?;
    Ok(imported)
}

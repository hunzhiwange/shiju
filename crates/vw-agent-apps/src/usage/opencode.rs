//! Read-only OpenCode usage projection. Roots and source identities are host-authorized.
use super::{
    cost::{ModelPricing, calculate},
    cursors::{UsageCursor, UsageCursorError, UsageCursorKey, load_usage_cursor},
    store::{
        StoredUsageRecord, UsageSaveOutcome, UsageSourceIdentity, UsageStoreError,
        save_usage_record_in_transaction,
    },
};
use rusqlite::{Connection, TransactionBehavior, params};
use rust_decimal::Decimal;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};
use vw_api_types::{id::ProjectScope, usage::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenCodeSchema {
    V1,
    V2,
}

#[derive(Debug)]
pub enum OpenCodeScanError {
    UnsupportedSchema,
    SourceUnavailable,
    Cursor(UsageCursorError),
}
impl std::fmt::Display for OpenCodeScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnsupportedSchema => "Unsupported OpenCode database schema",
            Self::SourceUnavailable => "OpenCode source unavailable",
            Self::Cursor(_) => "OpenCode usage scan failed",
        })
    }
}
impl std::error::Error for OpenCodeScanError {}
impl From<UsageCursorError> for OpenCodeScanError {
    fn from(error: UsageCursorError) -> Self {
        Self::Cursor(error)
    }
}
impl From<UsageStoreError> for OpenCodeScanError {
    fn from(error: UsageStoreError) -> Self {
        Self::Cursor(error.into())
    }
}
fn storage(_: rusqlite::Error) -> OpenCodeScanError {
    UsageStoreError::StorageUnavailable.into()
}
fn invalid() -> OpenCodeScanError {
    UsageCursorError::InvalidRecord.into()
}

/// Prefer V2 after migration, when frozen V1 tables may still be present.
pub fn detect_schema(db: &Connection) -> Result<OpenCodeSchema, OpenCodeScanError> {
    let exists = |table| {
        db.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            [table],
            |r| r.get::<_, bool>(0),
        )
        .map_err(|_| OpenCodeScanError::SourceUnavailable)
    };
    let schema = if exists("session_v2")? && exists("session_message")? {
        OpenCodeSchema::V2
    } else if exists("session")? && exists("message")? {
        OpenCodeSchema::V1
    } else {
        return Err(OpenCodeScanError::UnsupportedSchema);
    };
    let (sessions, messages) = tables(schema);
    db.prepare(&format!("SELECT s.id,s.time_updated,m.id,m.session_id,m.time_updated,m.data{} FROM {sessions} s LEFT JOIN {messages} m ON m.session_id=s.id LIMIT 0",
        if schema == OpenCodeSchema::V2 { ",m.type" } else { "" }))
        .map_err(|_| OpenCodeScanError::UnsupportedSchema)?;
    Ok(schema)
}
fn tables(schema: OpenCodeSchema) -> (&'static str, &'static str) {
    match schema {
        OpenCodeSchema::V1 => ("session", "message"),
        OpenCodeSchema::V2 => ("session_v2", "session_message"),
    }
}
#[derive(Debug, Default)]
pub struct OpenCodeScanOutcome {
    pub imported: usize,
    pub replayed: usize,
    pub deferred: usize,
}
fn label<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key)?.as_str().filter(|s| !s.trim().is_empty() && !s.contains('\0'))
}

/// Ledger and per-session message update watermarks commit atomically. Pending
/// messages pin their session watermark so completion can be retried even if the
/// session timestamp does not change. Never use main-file mtime (WAL may be newer).
/// The caller initializes the shared ledger/cursor schemas before scanning.
pub fn scan_opencode_database(
    ledger: &mut Connection,
    scope: &ProjectScope,
    source: &UsageCursorKey,
    data_root: &Path,
    mut lookup_price: impl FnMut(&str) -> Result<Option<ModelPricing>, UsageCursorError>,
) -> Result<OpenCodeScanOutcome, OpenCodeScanError> {
    if source.app != "opencode"
        || [&source.target_host, &source.instance_id, &source.file_identity]
            .into_iter()
            .any(|s| s.trim().is_empty() || s.contains('\0'))
    {
        return Err(invalid());
    }
    let (mut native, _) = crate::sessions::providers::opencode_sqlite::open(data_root)
        .map_err(|_| OpenCodeScanError::SourceUnavailable)?;
    let snapshot = native.transaction().map_err(|_| OpenCodeScanError::SourceUnavailable)?;
    let schema = detect_schema(&snapshot)?;
    let tx = ledger.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    let settings =
        super::source_settings::load_source_settings(&tx, scope, &source.target_host, &source.app)
            .map_err(|_| UsageStoreError::StorageUnavailable)?;
    if !settings.settings.scan_enabled {
        return Err(UsageCursorError::ScanDisabled.into());
    }
    let old = load_usage_cursor(&tx, scope, source)?;
    let mut watermarks: BTreeMap<String, i64> = match old {
        Some(cursor) => serde_json::from_str(cursor.parser_state.as_deref().ok_or_else(invalid)?)
            .map_err(|_| invalid())?,
        None => BTreeMap::new(),
    };
    let (sessions, messages) = tables(schema);
    let mut stmt = snapshot.prepare(&format!(
        "SELECT s.id, MAX(s.time_updated,COALESCE(MAX(m.time_updated),s.time_updated)) FROM {sessions} s LEFT JOIN {messages} m ON m.session_id=s.id GROUP BY s.id"
    )).map_err(|_| OpenCodeScanError::SourceUnavailable)?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .map_err(|_| OpenCodeScanError::SourceUnavailable)?;
    let mut outcome = OpenCodeScanOutcome::default();
    for row in rows {
        let (session, watermark) = row.map_err(|_| invalid())?;
        let cursor_id = format!("{schema:?}:{session}");
        if watermarks.get(&cursor_id).is_some_and(|old| *old == watermark) {
            continue;
        }
        let mut pending = false;
        let mut stmt = snapshot
            .prepare(&format!(
                "SELECT id,data,{} FROM {messages} WHERE session_id=?1{} ORDER BY id",
                if schema == OpenCodeSchema::V2 { "type" } else { "NULL" },
                if schema == OpenCodeSchema::V2 {
                    " AND type IN ('assistant','compaction')"
                } else {
                    ""
                }
            ))
            .map_err(|_| OpenCodeScanError::SourceUnavailable)?;
        let rows = stmt
            .query_map([&session], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?))
            })
            .map_err(|_| OpenCodeScanError::SourceUnavailable)?;
        for row in rows {
            let (request, json, kind) = row.map_err(|_| invalid())?;
            let value: Value = serde_json::from_str(&json).map_err(|_| invalid())?;
            if schema == OpenCodeSchema::V1 && label(&value, "role") != Some("assistant") {
                continue;
            }
            let compaction = kind.as_deref() == Some("compaction");
            let done = if compaction {
                matches!(label(&value, "status"), Some("completed" | "failed"))
            } else {
                value["time"]["completed"].as_i64().is_some()
            };
            if !done {
                pending = true;
                outcome.deferred += 1;
                continue;
            }
            let t = &value["tokens"];
            let tokens = UsageTokens {
                input: t["input"].as_u64(),
                output: t["output"].as_u64(),
                reasoning: t["reasoning"].as_u64(),
                cache_read: t["cache"]["read"].as_u64(),
                cache_creation: t["cache"]["write"].as_u64(),
                cache_creation_1h: None,
                input_semantics: UsageInputSemantics::Fresh,
                reasoning_semantics: UsageReasoningSemantics::SeparateFromOutput,
            };
            if ![
                tokens.input,
                tokens.output,
                tokens.reasoning,
                tokens.cache_read,
                tokens.cache_creation,
            ]
            .into_iter()
            .flatten()
            .any(|v| v > 0)
            {
                continue;
            }
            let created =
                value["time"]["created"].as_i64().filter(|v| *v > 0).ok_or_else(invalid)?;
            let model = label(&value, "modelID")
                .or_else(|| label(&value["model"], "id"))
                .or_else(|| value["model"].as_str())
                .map(str::to_owned);
            let native_cost = value
                .get("cost")
                .filter(|v| !v.is_null())
                .map(|v| {
                    let text = v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string());
                    text.parse::<Decimal>().ok().filter(|v| *v >= Decimal::ZERO).ok_or_else(invalid)
                })
                .transpose()?;
            let price = if native_cost.is_none() {
                match &model {
                    Some(m) => lookup_price(m)?,
                    None => None,
                }
            } else {
                None
            };
            let calculated =
                calculate(&tokens, price.as_ref(), Some(Decimal::ONE)).map_err(|_| invalid())?;
            let priced = calculated.is_some();
            let cost = match native_cost {
                Some(cost) => UsageCost {
                    source: UsageCostSource::Native,
                    amount_usd: Some(cost.to_string()),
                },
                None => calculated
                    .map(|c| c.to_usage_cost())
                    .unwrap_or(UsageCost { source: UsageCostSource::Unknown, amount_usd: None }),
            };
            let identity = UsageSourceIdentity {
                target_host: source.target_host.clone(),
                app: source.app.clone(),
                instance_id: source.instance_id.clone(),
                file_identity: source.file_identity.clone(),
                kind: UsageSourceKind::Native,
                native_session_id: Some(session.clone()),
                native_request_id: request.clone(),
            };
            let id = format!(
                "opencode_session:{:x}",
                Sha256::digest(serde_json::to_vec(&(scope, &identity)).map_err(|_| invalid())?)
            );
            let end = value["time"]["streamed"]
                .as_i64()
                .filter(|v| *v >= created)
                .or_else(|| value["time"]["completed"].as_i64().filter(|v| *v >= created));
            let record = StoredUsageRecord {
                record: UsageRecord {
                    id: id.clone(),
                    started_at_unix_seconds: UsageUnixSeconds(created / 1000),
                    provider: label(&value, "providerID")
                        .or_else(|| label(&value["model"], "providerID"))
                        .map(|p| UsageProvider::Known { provider_id: p.to_owned() })
                        .unwrap_or(UsageProvider::Unknown),
                    sources: vec![UsageSourceRef {
                        source_id: id,
                        kind: UsageSourceKind::Native,
                        app: source.app.clone(),
                        native_session_id: Some(session.clone()),
                        native_request_id: Some(request),
                    }],
                    models: UsageModels {
                        request_model: model.clone(),
                        response_model: model.clone(),
                        pricing_model: if priced { model } else { None },
                        pricing_multiplier: priced.then(|| "1".into()),
                        multiplier_basis: priced.then(|| "native OpenCode pricing".into()),
                    },
                    status: if label(&value, "status") == Some("failed")
                        || !value["error"].is_null()
                    {
                        UsageRequestStatus::Failed
                    } else {
                        UsageRequestStatus::Succeeded
                    },
                    billable: true,
                    tokens: Some(tokens),
                    cost,
                    performance: UsagePerformance {
                        first_token_ms: None,
                        latency_ms: end.and_then(|v| v.checked_sub(created)).map(|v| {
                            UsageMeasurement {
                                value: v as f64,
                                kind: UsageMeasurementKind::Measured,
                            }
                        }),
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
        }
        if !pending {
            watermarks.insert(cursor_id, watermark);
        }
    }
    let cursor = UsageCursor {
        byte_offset: 0,
        line_offset: 0,
        tail_fingerprint: vec![0; 32],
        parser_state: Some(serde_json::to_string(&watermarks).map_err(|_| invalid())?),
    };
    tx.execute("INSERT INTO usage_scan_cursors(cursor_key,payload_json) VALUES (?1,?2) ON CONFLICT(cursor_key) DO UPDATE SET payload_json=excluded.payload_json",
        params![serde_json::to_string(&(scope,source)).map_err(|_| invalid())?, serde_json::to_string(&cursor).map_err(|_| invalid())?]).map_err(storage)?;
    tx.commit().map_err(storage)?;
    Ok(outcome)
}

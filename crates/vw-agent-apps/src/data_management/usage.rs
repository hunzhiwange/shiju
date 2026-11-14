//! CC request migration into the unified ledger. Never imports scanner cursors or
//! synthesizes requests from aggregates. Callers authorize and capture rollback
//! preimages before execute, retaining their operation lock through commit.
use std::{collections::BTreeSet, str::FromStr};
use rusqlite::{Connection, TransactionBehavior};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use vw_api_types::{agent_apps::AppId, usage::*};
use crate::usage::store::{self, StoredUsageRecord, UsageSourceIdentity};
use super::{preview::{self, Decision, ImportPreview, PreviewCandidate, PreviewError}, source::*};

pub struct UsagePreview {
    preview: ImportPreview,
    rows: Vec<(UsageSourceIdentity, StoredUsageRecord)>,
}
impl UsagePreview {
    pub fn entries(&self) -> &[preview::PreviewEntry] { self.preview.entries() }

    /// Caller supplies an authenticated provider binding checker, scoped to the
    /// same principal/project/host as preparation. Any error rolls back the batch.
    pub fn execute(&self, connection: &mut Connection, scope: &SourceScope,
        source: &SourceSnapshot, decisions: &[(Uuid, Decision)],
        mut binding: impl FnMut(&str, &str) -> bool,
    ) -> Result<usize, PreviewError> {
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| PreviewError::InvalidCandidate)?;
        let current = current_values(&tx, scope, &self.rows)?;
        let resolved = self.preview.resolve(scope, source, &revision(&current)?, decisions)?;
        let mut written = 0;
        for (entry, (identity, value)) in self.preview.entries().iter().zip(&self.rows) {
            if resolved[&entry.id] == Decision::Skip { continue; }
            let UsageProvider::Known { provider_id } = &value.record.provider else {
                return Err(PreviewError::InvalidCandidate);
            };
            if !binding(&identity.app, provider_id) { return Err(PreviewError::Forbidden); }
            crate::usage::import_store::replace_imported_usage_in_transaction(&tx, scope.project(), identity, value)
                .map_err(|_| PreviewError::InvalidCandidate)?;
            written += 1;
        }
        tx.commit().map_err(|_| PreviewError::InvalidCandidate)?;
        Ok(written)
    }
}

/// namespace is a stable, host-authorized source registry identity, not the
/// changing file digest or a user path. Provider remapping must resolve only
/// accounts already owned by this authenticated principal.
pub fn prepare(connection: &Connection, scope: &SourceScope, source: &SourceSnapshot,
    namespace: &str, mut provider: impl FnMut(&str, &str) -> Option<String>,
) -> Result<UsagePreview, PreviewError> {
    if namespace.trim().is_empty() || namespace.contains('\0') { return Err(PreviewError::InvalidCandidate); }
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    for row in source.records(scope).map_err(|_| PreviewError::Forbidden)?.iter()
        .filter(|r| r.category == SourceCategory::Usage) {
        // Aggregates and prices need their own owner transaction contract; do not
        // silently drop them or count them alongside their original requests.
        if row.origin != "proxy_request_logs" { return Err(PreviewError::UnsupportedCategory); }
        let f = &row.fields;
        let request = text(f, "request_id")?;
        let app = text(f, "app_type")?;
        serde_json::from_value::<AppId>(json!(app)).map_err(|_| PreviewError::InvalidCandidate)?;
        let target = provider(app, text(f, "provider_id")?).filter(|s| !s.trim().is_empty())
            .ok_or(PreviewError::Forbidden)?;
        if !seen.insert((app.to_owned(), request.to_owned())) { return Err(PreviewError::InvalidCandidate); }
        let source_kind = match f.get("data_source").and_then(Value::as_str).unwrap_or("proxy") {
            "proxy" => UsageSourceKind::Relay,
            _ => return Err(PreviewError::UnsupportedCategory),
        };
        let identity = UsageSourceIdentity { target_host: scope.host_id().into(), app: app.into(),
            instance_id: scope.principal_id().into(), kind: source_kind,
            file_identity: format!("cc-import:{namespace}"), native_session_id: optional(f,"session_id")?,
            native_request_id: request.into() };
        let id = format!("cc-{:x}", Sha256::digest(serde_json::to_vec(&(scope.project(), &identity))
            .map_err(|_| PreviewError::InvalidCandidate)?));
        let amount = text(f,"total_cost_usd")?;
        decimal(amount)?;
        let multiplier = text(f,"cost_multiplier")?;
        decimal(multiplier)?;
        let status = f.get("status_code").and_then(Value::as_u64).filter(|n| (100..=599).contains(n))
            .ok_or(PreviewError::InvalidCandidate)? as u16;
        let semantics = match f.get("input_token_semantics").and_then(Value::as_i64) {
            Some(0) if ["codex", "gemini", "grokbuild"].contains(&app) => UsageInputSemantics::Total,
            Some(0) | Some(2) => UsageInputSemantics::Fresh,
            Some(1) => UsageInputSemantics::Total,
            _ => return Err(PreviewError::InvalidCandidate),
        };
        let tokens = UsageTokens { input: Some(number(f,"input_tokens")?), output: Some(number(f,"output_tokens")?),
            reasoning: None, cache_read: Some(number(f,"cache_read_tokens")?),
            cache_creation: Some(number(f,"cache_creation_tokens")?), cache_creation_1h: None,
            input_semantics: semantics, reasoning_semantics: UsageReasoningSemantics::Unknown };
        let measured = |key| -> Result<Option<UsageMeasurement>, PreviewError> {
            if f.get(key).is_none_or(Value::is_null) { return Ok(None); }
            Ok(Some(UsageMeasurement { value: number(f,key)? as f64, kind: UsageMeasurementKind::Measured }))
        };
        let value = StoredUsageRecord { status_code: Some(status), failure: if status >= 400 { Some(store::UsageFailure::Upstream) } else { None },
            record: UsageRecord { id: id.clone(), started_at_unix_seconds: UsageUnixSeconds(f.get("created_at").and_then(Value::as_i64)
                .filter(|n| *n >= 0).ok_or(PreviewError::InvalidCandidate)?),
                provider: UsageProvider::Known { provider_id: target },
                sources: vec![UsageSourceRef { source_id: id, kind: source_kind, app: app.into(),
                    native_request_id: Some(request.into()), native_session_id: identity.native_session_id.clone() }],
                models: UsageModels { request_model: optional(f,"request_model")?, response_model: Some(text(f,"model")?.into()),
                    pricing_model: optional(f,"pricing_model")?, pricing_multiplier: Some(multiplier.into()), multiplier_basis: Some("cc-source".into()) },
                status: if status < 400 { UsageRequestStatus::Succeeded } else { UsageRequestStatus::Failed },
                billable: Decimal::from_str(amount).map_err(|_| PreviewError::InvalidCandidate)? != Decimal::ZERO || tokens.input != Some(0) || tokens.output != Some(0)
                    || tokens.cache_read != Some(0) || tokens.cache_creation != Some(0),
                tokens: Some(tokens), cost: UsageCost { source: UsageCostSource::Native, amount_usd: Some(amount.into()) },
                performance: UsagePerformance { first_token_ms: measured("first_token_ms")?, latency_ms: measured("latency_ms")?, output_tokens_per_second: None },
                suspected_duplicate_of: None } };
        rows.push((identity,value));
    }
    let current = current_values(connection, scope, &rows)?;
    let preview = preview::build_preview(scope, source, &[SourceCategory::Usage], revision(&current)?, |_,_| {
        rows.iter().zip(&current).map(|((_,v), before)| Ok(PreviewCandidate {
            incoming: serde_json::to_value(v).map_err(|_| PreviewError::InvalidCandidate)?, existing: before.clone(), references_valid: true,
        })).collect()
    })?;
    Ok(UsagePreview { preview, rows })
}
fn current_values(connection: &Connection, scope: &SourceScope, rows: &[(UsageSourceIdentity,StoredUsageRecord)])
    -> Result<Vec<Option<Value>>, PreviewError> {
    rows.iter().map(|(identity,_)| crate::usage::import_store::load_imported_usage(connection, scope.project(), identity)
        .map_err(|_| PreviewError::InvalidCandidate)?.map(|v| serde_json::to_value(v)
            .map_err(|_| PreviewError::InvalidCandidate)).transpose()).collect()
}
fn revision(values: &[Option<Value>]) -> Result<String, PreviewError> {
    Ok(format!("{:x}",Sha256::digest(serde_json::to_vec(values).map_err(|_| PreviewError::InvalidCandidate)?)))
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, PreviewError> {
    v.get(key).and_then(Value::as_str).filter(|s| !s.trim().is_empty() && !s.contains('\0')).ok_or(PreviewError::InvalidCandidate)
}
fn optional(v: &Value, key: &str) -> Result<Option<String>, PreviewError> {
    if v.get(key).is_none_or(Value::is_null) { Ok(None) } else { Ok(Some(text(v,key)?.into())) }
}
fn number(v: &Value, key: &str) -> Result<u64, PreviewError> {
    v.get(key).and_then(Value::as_u64).ok_or(PreviewError::InvalidCandidate)
}
fn decimal(s: &str) -> Result<(), PreviewError> {
    Decimal::from_str(s).ok().filter(|v| *v >= Decimal::ZERO).map(|_| ()).ok_or(PreviewError::InvalidCandidate)
}

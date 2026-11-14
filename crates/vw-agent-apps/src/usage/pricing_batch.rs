//! Atomic explicit price imports and bounded, cancellable unknown-cost maintenance.
use std::{collections::{BTreeMap, BTreeSet}, str::FromStr};
use rusqlite::{Connection, TransactionBehavior, params};
use rust_decimal::Decimal;
use vw_api_types::{id::{ModelId, ProjectScope}, usage::UsageCostSource};
use super::{cost::calculate, pricing::{ModelPricingInfo, StoredModelPricing, get_model_pricing},
    source_settings::advance_statistics_revision, store::StoredUsageRecord};

fn storage(_: rusqlite::Error) -> String { "Usage pricing storage unavailable".into() }
fn scope_key(scope: &ProjectScope, host: &str) -> Result<String, String> {
    if host.trim().is_empty() || host.contains('\0') { return Err("Missing target host".into()); }
    serde_json::to_string(&(scope, host)).map_err(|_| "Invalid pricing scope".into())
}

/// Manual imports are custom overrides. Source must explicitly identify the import.
/// Every entry is validated (including overwritten duplicates); last duplicate wins.
/// Returns the number of distinct saved prices whose complete value changed.
pub fn import_model_pricing(
    connection: &mut Connection, scope: &ProjectScope, host: &str,
    source: &str, entries: Vec<ModelPricingInfo>,
) -> Result<usize, String> {
    let key = scope_key(scope, host)?;
    let source = source.trim();
    if source.is_empty() || source.contains('\0') { return Err("Missing price import source".into()); }
    let mut normalized = BTreeMap::new();
    for entry in entries {
        let price = entry.normalize()?;
        normalized.insert(price.model_id.as_ref().to_owned(), StoredModelPricing {
            price, source: source.into(), is_custom: true,
        });
    }
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    let mut changed = 0;
    for saved in normalized.into_values() {
        if get_model_pricing(&tx, scope, host, &saved.price.model_id)?.as_ref() == Some(&saved) {
            continue;
        }
        let payload = serde_json::to_string(&saved).map_err(|_| "Invalid model price".to_owned())?;
        tx.execute("INSERT INTO usage_model_prices(scope_key,model_id,payload_json) VALUES (?1,?2,?3)
            ON CONFLICT(scope_key,model_id) DO UPDATE SET payload_json=excluded.payload_json",
            params![key, saved.price.model_id.as_ref(), payload]).map_err(storage)?;
        changed += 1;
    }
    tx.commit().map_err(storage)?;
    Ok(changed)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BackfillProgress {
    pub scanned: usize,
    pub changed: usize,
    pub cancelled: bool,
}

/// Each page commits independently. Cancellation rolls back the current page,
/// preserving earlier committed pages; rerunning safely skips known costs.
/// Archived rollups are deliberately excluded: their request-level evidence is gone.
/// Call this after importing prices, including when a replay changed zero prices.
pub fn backfill_unknown_costs(
    connection: &mut Connection, scope: &ProjectScope, host: &str,
    batch_size: usize, mut is_cancelled: impl FnMut() -> bool,
) -> Result<BackfillProgress, String> {
    let key = scope_key(scope, host)?;
    if !(1..=256).contains(&batch_size) { return Err("Backfill batch size must be 1..=256".into()); }
    let mut progress = BackfillProgress::default();
    let mut cursor: Option<String> = None;
    loop {
        if is_cancelled() { progress.cancelled = true; return Ok(progress); }
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
        let rows = {
            let mut stmt = tx.prepare("SELECT record_id,payload_json FROM usage_records
                WHERE scope_key=?1 AND (?2 IS NULL OR record_id>?2) ORDER BY record_id LIMIT ?3").map_err(storage)?;
            stmt.query_map(params![key, cursor, batch_size as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }).map_err(storage)?.collect::<Result<Vec<_>, _>>().map_err(storage)?
        };
        if rows.is_empty() { return Ok(progress); }
        let mut apps = BTreeSet::new();
        let mut changed = 0;
        for (id, payload) in &rows {
            if is_cancelled() { progress.cancelled = true; return Ok(progress); }
            let mut value: StoredUsageRecord = serde_json::from_str(payload)
                .map_err(|_| "Invalid saved usage record".to_owned())?;
            let record = &mut value.record;
            if record.id != *id { return Err("Invalid saved usage identity".into()); }
            if record.cost.source != UsageCostSource::Unknown || record.cost.amount_usd.is_some() { continue; }
            let (Some(model), Some(multiplier), Some(tokens)) = (
                &record.models.pricing_model, &record.models.pricing_multiplier, &record.tokens,
            ) else { continue; };
            let Some(price) = get_model_pricing(&tx, scope, host, &ModelId::from(model.clone()))? else { continue; };
            let multiplier = Decimal::from_str(multiplier).map_err(|_| "Invalid saved pricing multiplier".to_owned())?;
            let Some(cost) = calculate(tokens, Some(&price.price.rates()?), Some(multiplier))? else { continue; };
            record.cost = cost.to_usage_cost();
            apps.extend(record.sources.iter().map(|source| source.app.clone()));
            let payload = serde_json::to_string(&value).map_err(|_| "Invalid usage cost".to_owned())?;
            tx.execute("UPDATE usage_records SET payload_json=?3 WHERE scope_key=?1 AND record_id=?2",
                params![key, id, payload]).map_err(storage)?;
            changed += 1;
        }
        for app in apps {
            advance_statistics_revision(&tx, scope, host, &app).map_err(|_| "Usage revision unavailable".to_owned())?;
        }
        if is_cancelled() { progress.cancelled = true; return Ok(progress); }
        tx.commit().map_err(storage)?;
        progress.scanned += rows.len();
        progress.changed += changed;
        cursor = rows.last().map(|(id, _)| id.clone());
    }
}

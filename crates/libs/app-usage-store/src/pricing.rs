//! Scoped USD prices on the host-owned SQLite connection. No CC configuration writes.
use app_models::edits::validate_model_id;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use {
    vw_api_transport::id::ModelId,
    vw_api_transport::id::ProjectScope,
};

pub use app_usage_core::pricing::{ModelPricingInfo, StoredModelPricing};

fn key(scope: &ProjectScope, host: &str) -> Result<String, String> {
    if host.trim().is_empty() {
        return Err("Missing target host".into());
    }
    serde_json::to_string(&(scope, host)).map_err(|_| "Invalid pricing scope".into())
}
fn storage(_: rusqlite::Error) -> String {
    "Model pricing storage unavailable".into()
}

pub fn initialize_pricing_store(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS usage_model_prices (
            scope_key TEXT NOT NULL,
            model_id TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            PRIMARY KEY(scope_key, model_id)
        );
        CREATE TABLE IF NOT EXISTS usage_billing_defaults (scope_key TEXT PRIMARY KEY NOT NULL, payload_json TEXT NOT NULL);",
        )
        .map_err(storage)
}

pub fn get_model_pricing(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
    model_id: &ModelId,
) -> Result<Option<StoredModelPricing>, String> {
    validate_model_id(model_id)?;
    let payload: Option<String> = connection
        .query_row(
            "SELECT payload_json FROM usage_model_prices WHERE scope_key=?1 AND model_id=?2",
            params![key(scope, host)?, model_id.as_ref()],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage)?;
    payload
        .map(|payload| {
            serde_json::from_str(&payload).map_err(|_| "Invalid saved model price".to_owned())
        })
        .transpose()
}

pub fn list_model_pricing(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
) -> Result<Vec<StoredModelPricing>, String> {
    let mut statement = connection
        .prepare("SELECT payload_json FROM usage_model_prices WHERE scope_key=?1 ORDER BY model_id")
        .map_err(storage)?;
    let rows =
        statement.query_map([key(scope, host)?], |row| row.get::<_, String>(0)).map_err(storage)?;
    rows.map(|row| {
        serde_json::from_str(&row.map_err(storage)?)
            .map_err(|_| "Invalid saved model price".to_owned())
    })
    .collect()
}

/// Explicit user edit: insert or replace one price, marking it as a custom override.
/// Validate before starting the transaction; a failed write/commit retains the saved price.
pub fn save_model_pricing(
    connection: &mut Connection,
    scope: &ProjectScope,
    host: &str,
    price: ModelPricingInfo,
) -> Result<StoredModelPricing, String> {
    let saved =
        StoredModelPricing { price: price.normalize()?, source: "custom".into(), is_custom: true };
    let scope_key = key(scope, host)?;
    let payload = serde_json::to_string(&saved).map_err(|_| "Invalid model price".to_owned())?;
    let transaction =
        connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    transaction
        .execute(
            "INSERT INTO usage_model_prices(scope_key,model_id,payload_json) VALUES (?1,?2,?3)
         ON CONFLICT(scope_key,model_id) DO UPDATE SET payload_json=excluded.payload_json",
            params![scope_key, saved.price.model_id.as_ref(), payload],
        )
        .map_err(storage)?;
    transaction.commit().map_err(storage)?;
    Ok(saved)
}

/// Missing rows are distinguishable from successful deletion; failures roll back.
pub fn delete_model_pricing(
    connection: &mut Connection,
    scope: &ProjectScope,
    host: &str,
    model_id: &ModelId,
) -> Result<bool, String> {
    validate_model_id(model_id)?;
    let scope_key = key(scope, host)?;
    let transaction =
        connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    let deleted = transaction
        .execute(
            "DELETE FROM usage_model_prices WHERE scope_key=?1 AND model_id=?2",
            params![scope_key, model_id.as_ref()],
        )
        .map_err(storage)?;
    transaction.commit().map_err(storage)?;
    Ok(deleted > 0)
}

/// Defaults affect newly ingested relay requests; native reported costs stay authoritative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingModelSource { Request, Response }

pub const BILLING_APPS: [&str; 4] = ["claude", "codex", "gemini", "grokbuild"];
pub type BillingDefaults = std::collections::BTreeMap<String, BillingModelSource>;

pub fn load_billing_defaults(connection: &Connection, scope: &ProjectScope, host: &str) -> Result<BillingDefaults, String> {
    let payload: Option<String> = connection.query_row(
        "SELECT payload_json FROM usage_billing_defaults WHERE scope_key=?1", [key(scope, host)?], |row| row.get(0)
    ).optional().map_err(storage)?;
    match payload {
        Some(payload) => serde_json::from_str(&payload).map_err(|_| "Invalid saved billing defaults".into()),
        None => Ok(BILLING_APPS.into_iter().map(|app| (app.into(), BillingModelSource::Response)).collect()),
    }
}

pub fn save_billing_defaults(connection: &mut Connection, scope: &ProjectScope, host: &str, defaults: BillingDefaults) -> Result<BillingDefaults, String> {
    if defaults.len() != BILLING_APPS.len() || BILLING_APPS.iter().any(|app| !defaults.contains_key(*app)) {
        return Err("Invalid billing applications".into());
    }
    let payload = serde_json::to_string(&defaults).map_err(|_| "Invalid billing defaults".to_owned())?;
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    tx.execute("INSERT INTO usage_billing_defaults(scope_key,payload_json) VALUES (?1,?2)
        ON CONFLICT(scope_key) DO UPDATE SET payload_json=excluded.payload_json", params![key(scope, host)?, payload]).map_err(storage)?;
    tx.commit().map_err(storage)?;
    Ok(defaults)
}

pub(crate) fn apply_billing_default(connection: &Connection, scope: &ProjectScope, identity: &super::store::UsageSourceIdentity, record: &mut vw_api_usage::UsageRecord) -> Result<(), String> {
    use vw_api_usage::{UsageCost, UsageCostSource, UsageSourceKind};
    use std::str::FromStr;
    if identity.kind != UsageSourceKind::Relay || !BILLING_APPS.contains(&identity.app.as_str())
        || !matches!(record.cost.source, UsageCostSource::Unknown | UsageCostSource::Calculated) { return Ok(()); }
    let defaults = load_billing_defaults(connection, scope, &identity.target_host)?;
    let source = defaults.get(&identity.app).ok_or("Invalid billing application")?;
    let model = match source {
        BillingModelSource::Request => record.models.request_model.clone(),
        BillingModelSource::Response => record.models.response_model.clone().or_else(|| record.models.request_model.clone()),
    };
    record.models.pricing_model = model.clone();
    record.cost = UsageCost { source: UsageCostSource::Unknown, amount_usd: None };
    let (Some(model), Some(tokens), Some(multiplier)) = (model, &record.tokens, &record.models.pricing_multiplier) else { return Ok(()); };
    let Some(price) = get_model_pricing(connection, scope, &identity.target_host, &ModelId::from(model))? else { return Ok(()); };
    let multiplier = rust_decimal::Decimal::from_str(multiplier).map_err(|_| "Invalid pricing multiplier")?;
    if let Some(cost) = super::cost::calculate(tokens, Some(&price.price.rates()?), Some(multiplier))? { record.cost = cost.to_usage_cost(); }
    Ok(())
}

#[cfg(test)]
#[path = "usage_replica_08_billing_tests.rs"]
mod billing_tests;

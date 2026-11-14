//! Scoped USD prices on the host-owned SQLite connection. No CC configuration writes.
use super::cost::ModelPricing;
use crate::models::edits::validate_model_id;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use vw_api_types::id::{ModelId, ProjectScope};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPricingInfo {
    pub model_id: ModelId,
    pub display_name: String,
    pub input_cost_per_million: String,
    pub output_cost_per_million: String,
    pub cache_read_cost_per_million: String,
    pub cache_creation_cost_per_million: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredModelPricing {
    pub price: ModelPricingInfo,
    pub source: String,
    pub is_custom: bool,
}

impl ModelPricingInfo {
    pub fn rates(&self) -> Result<ModelPricing, String> {
        ModelPricing::from_strings(
            self.input_cost_per_million.trim(),
            self.output_cost_per_million.trim(),
            self.cache_read_cost_per_million.trim(),
            self.cache_creation_cost_per_million.trim(),
        )
    }

    pub(super) fn normalize(mut self) -> Result<Self, String> {
        self.model_id = ModelId::from(self.model_id.as_ref().trim().to_owned());
        validate_model_id(&self.model_id)?;
        self.display_name = self.display_name.trim().to_owned();
        if self.display_name.is_empty() {
            return Err("Model display name is required".into());
        }
        let rates = self.rates()?;
        self.input_cost_per_million = rates.input_cost_per_million.normalize().to_string();
        self.output_cost_per_million = rates.output_cost_per_million.normalize().to_string();
        self.cache_read_cost_per_million =
            rates.cache_read_cost_per_million.normalize().to_string();
        self.cache_creation_cost_per_million =
            rates.cache_creation_cost_per_million.normalize().to_string();
        Ok(self)
    }
}

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
        );",
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

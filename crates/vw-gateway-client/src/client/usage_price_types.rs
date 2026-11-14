//! Price and display wire types. Amounts remain Decimal strings; display rates
//! follow the host API's f64 contract and never change ledger amounts.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use vw_api_types::id::ModelId;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelsDevEntry {
    pub key: String,
    pub provider_id: String,
    pub provider_name: String,
    pub model_id: String,
    pub normalized_id: String,
    pub model_name: String,
    pub release_date: String,
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelsDevSelectionConfig {
    pub include_common_models: bool,
    pub selected_model_keys: Vec<String>,
    pub excluded_common_model_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelsDevSyncConfig {
    pub auto_sync_enabled: bool,
    pub selection: ModelsDevSelectionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsDevSyncState {
    pub config: ModelsDevSyncConfig,
    pub last_sync_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Currency {
    #[default]
    USD,
    CNY,
    EUR,
    JPY,
    TWD,
    TRY,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateMode {
    #[default]
    Manual,
    Auto,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExchangeSettings {
    pub currency: Currency,
    pub mode: RateMode,
    pub manual_rates: BTreeMap<String, f64>,
    pub cache: Option<CachedRates>,
    /// Fixed diagnostic; never contains upstream response bodies or URLs.
    pub refresh_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedRates {
    pub rates: BTreeMap<String, f64>,
    pub fetched_at: i64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoneyDisplay {
    pub currency: Currency,
    pub rate: f64,
    pub source: String,
    pub stale: bool,
    pub degraded: bool,
    pub error: Option<String>,
}

/// Only editable fields are serializable; sync results cannot be submitted.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UsageDisplayPatch {
    pub currency: Option<Currency>,
    pub mode: Option<RateMode>,
    pub manual_rates: BTreeMap<String, f64>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct UsageRatesState {
    pub display: MoneyDisplay,
    pub settings: ExchangeSettings,
}
#[derive(Debug, Clone, Deserialize)]
pub struct UsagePriceImportResult { pub changed: usize }
#[derive(Debug, Clone, Deserialize)]
pub struct UsagePriceDeleted { pub deleted: bool }
/// Accepted work is queried through usage_job; this is not a success result.
#[derive(Debug, Clone, Deserialize)]
pub struct UsagePriceSyncAccepted { pub task_id: String, pub status: String }

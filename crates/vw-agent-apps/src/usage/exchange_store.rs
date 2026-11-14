//! Scoped display settings on the host-owned database; separate from USD accounting.
use super::exchange::{CachedRates, Currency, ExchangeSettings, RateMode, TARGET_CURRENCIES};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::collections::BTreeMap;
use vw_api_types::id::ProjectScope;

#[derive(Default)]
pub struct DisplayPatch {
    pub currency: Option<Currency>,
    pub mode: Option<RateMode>,
    /// Merge only supplied currencies; do not replace the latest manual map.
    pub manual_rates: BTreeMap<String, f64>,
}
fn key(scope: &ProjectScope, host: &str) -> Result<String, String> {
    if host.trim().is_empty() {
        return Err("Missing target host".into());
    }
    serde_json::to_string(&(scope, host)).map_err(|_| "Invalid display scope".into())
}
fn storage(_: rusqlite::Error) -> String {
    "Exchange settings storage unavailable".into()
}

/// Invoked by the host migration alongside initialize_usage_store.
pub fn initialize_exchange_store(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS usage_exchange_settings (
        scope_key TEXT PRIMARY KEY NOT NULL,
        payload_json TEXT NOT NULL
    );",
        )
        .map_err(storage)
}
pub fn load_exchange_settings(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
) -> Result<ExchangeSettings, String> {
    let payload: Option<String> = connection
        .query_row(
            "SELECT payload_json FROM usage_exchange_settings WHERE scope_key=?1",
            [key(scope, host)?],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage)?;
    match payload {
        Some(payload) => {
            serde_json::from_str(&payload).map_err(|_| "Invalid exchange settings".into())
        }
        None => Ok(ExchangeSettings::default()),
    }
}
fn update(
    connection: &mut Connection,
    scope: &ProjectScope,
    host: &str,
    change: impl FnOnce(&mut ExchangeSettings),
) -> Result<ExchangeSettings, String> {
    let transaction =
        connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    let mut settings = load_exchange_settings(&transaction, scope, host)?;
    change(&mut settings);
    let payload =
        serde_json::to_string(&settings).map_err(|_| "Invalid exchange settings".to_string())?;
    transaction
        .execute(
            "INSERT INTO usage_exchange_settings(scope_key,payload_json) VALUES (?1,?2)
         ON CONFLICT(scope_key) DO UPDATE SET payload_json=excluded.payload_json",
            params![key(scope, host)?, payload],
        )
        .map_err(storage)?;
    transaction.commit().map_err(storage)?;
    Ok(settings)
}
pub fn save_display_patch(
    connection: &mut Connection,
    scope: &ProjectScope,
    host: &str,
    patch: DisplayPatch,
) -> Result<ExchangeSettings, String> {
    if patch.manual_rates.iter().any(|(code, rate)| {
        !TARGET_CURRENCIES.contains(&code.as_str()) || !rate.is_finite() || *rate <= 0.0
    }) {
        return Err("Invalid manual exchange rate".into());
    }
    update(connection, scope, host, |settings| {
        if let Some(currency) = patch.currency {
            settings.currency = currency;
        }
        if let Some(mode) = patch.mode {
            settings.mode = mode;
        }
        settings.manual_rates.extend(patch.manual_rates);
    })
}
/// Persist after awaiting the network, reloading current settings inside a transaction.
/// Failure retains the last successful cache. Clients cannot supply successful fetch results.
pub fn save_refresh_result(
    connection: &mut Connection,
    scope: &ProjectScope,
    host: &str,
    result: Result<CachedRates, String>,
) -> Result<ExchangeSettings, String> {
    if let Ok(cache) = &result {
        if cache.fetched_at <= 0
            || cache.rates.is_empty()
            || !["open.er-api.com", "floatrates.com"].contains(&cache.source.as_str())
            || cache.rates.iter().any(|(code, rate)| {
                !TARGET_CURRENCIES.contains(&code.as_str()) || !rate.is_finite() || *rate <= 0.0
            })
        {
            return Err("Invalid fetched exchange rates".into());
        }
    }
    update(connection, scope, host, |settings| match result {
        Ok(cache) => {
            // A slower, older refresh cannot replace a newer successful snapshot.
            if settings.cache.as_ref().is_none_or(|current| current.fetched_at <= cache.fetched_at)
            {
                settings.cache = Some(cache);
                settings.refresh_error = None;
            }
        }
        Err(_) => settings.refresh_error = Some("Exchange-rate sources unavailable".into()),
    })
}

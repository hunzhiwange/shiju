//! Host-owned price synchronization. Config writes never accept result timestamps.
use std::{sync::{Arc, Mutex}, time::{Duration, SystemTime, UNIX_EPOCH}};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use vw_api_types::id::ProjectScope;
use vw_provider_resolver::models::{self, ModelsRefreshStatus};
use super::{models_selection::{ModelsDevSelectionConfig, flatten_models, resolve_models_dev_selection, to_model_pricing}, pricing::{StoredModelPricing, get_model_pricing, initialize_pricing_store}};

pub const SYNC_INTERVAL_MS: i64 = 6 * 60 * 60 * 1000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelsDevSyncConfig {
    pub auto_sync_enabled: bool,
    pub selection: ModelsDevSelectionConfig,
}
impl Default for ModelsDevSyncConfig {
    fn default() -> Self {
        Self { auto_sync_enabled: false, selection: ModelsDevSelectionConfig {
            include_common_models: true, selected_model_keys: Vec::new(),
            excluded_common_model_keys: Vec::new(),
        } }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelsDevSyncState {
    pub config: ModelsDevSyncConfig,
    pub last_sync_at: Option<i64>,
    pub last_error: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelsDevSyncResult {
    pub skipped: bool,
    pub selected: usize,
    pub imported: usize,
    pub changed: usize,
    pub synced_at: Option<i64>,
}
fn storage(_: rusqlite::Error) -> String { "Model sync storage unavailable".into() }
fn key(scope: &ProjectScope, host: &str) -> Result<String, String> {
    if host.trim().is_empty() || host.contains('\0') { return Err("Missing target host".into()); }
    serde_json::to_string(&(scope, host)).map_err(|_| "Invalid model sync scope".into())
}
fn now_ms() -> Result<i64, String> {
    SystemTime::now().duration_since(UNIX_EPOCH).ok()
        .and_then(|time| i64::try_from(time.as_millis()).ok())
        .ok_or_else(|| "Invalid model sync clock".into())
}
pub fn initialize_models_sync(connection: &Connection) -> Result<(), String> {
    initialize_pricing_store(connection)?;
    connection.execute_batch("CREATE TABLE IF NOT EXISTS usage_models_sync (
        scope_key TEXT PRIMARY KEY, payload_json TEXT NOT NULL
    );").map_err(storage)
}
pub fn get_models_sync_config(connection: &Connection, scope: &ProjectScope, host: &str) -> Result<ModelsDevSyncState, String> {
    let raw: Option<String> = connection.query_row(
        "SELECT payload_json FROM usage_models_sync WHERE scope_key=?1", [key(scope, host)?],
        |row| row.get(0),
    ).optional().map_err(storage)?;
    raw.map(|raw| serde_json::from_str(&raw).map_err(|_| "Invalid saved model sync state".into()))
        .unwrap_or_else(|| Ok(ModelsDevSyncState::default()))
}
fn write_state(connection: &Connection, scope: &ProjectScope, host: &str, state: &ModelsDevSyncState) -> Result<(), String> {
    let payload = serde_json::to_string(state).map_err(|_| "Invalid model sync state".to_owned())?;
    connection.execute("INSERT INTO usage_models_sync(scope_key,payload_json) VALUES (?1,?2)
        ON CONFLICT(scope_key) DO UPDATE SET payload_json=excluded.payload_json",
        params![key(scope, host)?, payload]).map_err(storage)?;
    Ok(())
}
/// Separate input type prevents clients from forging last success or errors.
pub fn save_models_sync_config(connection: &mut Connection, scope: &ProjectScope, host: &str, config: ModelsDevSyncConfig) -> Result<ModelsDevSyncState, String> {
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    let mut state = get_models_sync_config(&tx, scope, host)?;
    state.config = config;
    write_state(&tx, scope, host, &state)?;
    tx.commit().map_err(storage)?;
    Ok(state)
}
fn due(state: &ModelsDevSyncState, now: i64) -> bool {
    state.config.auto_sync_enabled && state.last_sync_at.is_none_or(|last| {
        now < last || now.saturating_sub(last) >= SYNC_INTERVAL_MS
    })
}

/// One instance per host/scope, shared by the scheduler and explicit sync endpoint.
/// Drop the periodic future on gateway shutdown. No desktop process is required.
pub struct ModelsSync {
    connection: Arc<Mutex<Connection>>,
    scope: ProjectScope,
    host: String,
    flight: tokio::sync::Mutex<()>,
}
impl ModelsSync {
    pub fn new(connection: Arc<Mutex<Connection>>, scope: ProjectScope, host: String) -> Result<Self, String> {
        key(&scope, &host)?;
        {
            let guard = connection.lock().map_err(|_| "Model sync storage unavailable")?;
            initialize_models_sync(&guard)?;
        }
        Ok(Self { connection, scope, host, flight: tokio::sync::Mutex::new(()) })
    }
    pub async fn sync(&self, force: bool) -> Result<ModelsDevSyncResult, String> {
        // Concurrent automatic/manual calls report skipped while the active work finishes.
        let Ok(_flight) = self.flight.try_lock() else {
            return Ok(ModelsDevSyncResult { skipped: true, ..Default::default() });
        };
        {
            let connection = self.connection.lock().map_err(|_| "Model sync storage unavailable")?;
            let state = get_models_sync_config(&connection, &self.scope, &self.host)?;
            if !force && !due(&state, now_ms()?) {
                return Ok(ModelsDevSyncResult { skipped: true, synced_at: state.last_sync_at, ..Default::default() });
            }
        }
        let fetched = models::refresh().await;
        let entries = if matches!(fetched.status, ModelsRefreshStatus::Updated | ModelsRefreshStatus::Fresh) {
            Some(flatten_models(&models::snapshot().await.providers))
        } else { None };
        let mut connection = self.connection.lock().map_err(|_| "Model sync storage unavailable")?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
        // Selection and enabled state are reread only after fetching, under the write lock.
        let mut state = get_models_sync_config(&tx, &self.scope, &self.host)?;
        if !force && !due(&state, now_ms()?) {
            return Ok(ModelsDevSyncResult { skipped: true, synced_at: state.last_sync_at, ..Default::default() });
        }
        let Some(entries) = entries else {
            // Never persist remote payloads, URLs or credentials in diagnostics.
            state.last_error = Some("Model catalog refresh unavailable".into());
            write_state(&tx, &self.scope, &self.host, &state)?;
            tx.commit().map_err(storage)?;
            return Err("Model catalog refresh unavailable".into());
        };
        let selected = resolve_models_dev_selection(&entries, &state.config.selection);
        let prices = to_model_pricing(&selected);
        let mut result = ModelsDevSyncResult { selected: selected.len(), imported: prices.len(), ..Default::default() };
        for price in prices {
            let price = price.normalize()?;
            let existing = get_model_pricing(&tx, &self.scope, &self.host, &price.model_id)?;
            if existing.as_ref().is_some_and(|saved| saved.is_custom) { continue; }
            let saved = StoredModelPricing { price, source: "models.dev".into(), is_custom: false };
            if existing.as_ref() == Some(&saved) { continue; }
            let payload = serde_json::to_string(&saved).map_err(|_| "Invalid model price".to_owned())?;
            tx.execute("INSERT INTO usage_model_prices(scope_key,model_id,payload_json) VALUES (?1,?2,?3)
                ON CONFLICT(scope_key,model_id) DO UPDATE SET payload_json=excluded.payload_json",
                params![key(&self.scope, &self.host)?, saved.price.model_id.as_ref(), payload]).map_err(storage)?;
            result.changed += 1;
        }
        state.last_sync_at = Some(now_ms()?);
        state.last_error = None;
        write_state(&tx, &self.scope, &self.host, &state)?;
        tx.commit().map_err(storage)?;
        result.synced_at = state.last_sync_at;
        Ok(result)
    }
    /// Check on startup and every minute; persisted success enforces the six-hour window.
    /// Failures retain last success and retry next tick. Caller owns cancellation/lifecycle.
    pub async fn run(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let _ = self.sync(false).await;
        }
    }
}

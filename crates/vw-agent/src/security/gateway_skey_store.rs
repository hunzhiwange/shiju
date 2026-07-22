//! Tenant-scoped gateway skey metadata in the shared database.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use rusqlite::{TransactionBehavior, params};
use sha2::{Digest, Sha256};
use vw_api_types::id::TenantScope;
use vw_config_types::gateway::GatewaySkey;

use crate::app::agent::security::pairing::masked_skey_name;
use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;

mod postgres;

use postgres::PostgresGatewaySkeyStore;

const GATEWAY_SKEYS_SCHEMA: &str = r#"
BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS gateway_skeys (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    skey_hash TEXT NOT NULL CHECK (trim(skey_hash) <> ''),
    masked_skey TEXT NOT NULL DEFAULT '',
    name TEXT NOT NULL,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    expires_at TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (platform_id, company_id, skey_hash)
);

COMMIT;
"#;

#[derive(Debug, Clone)]
pub(crate) struct GatewaySkeyStore {
    provider: GatewaySkeyStoreProvider,
}

#[derive(Debug, Clone)]
enum GatewaySkeyStoreProvider {
    Sqlite(GlobalDatabase),
    Postgres(PostgresGatewaySkeyStore),
}

impl GatewaySkeyStore {
    pub(crate) fn new(database: GlobalDatabase) -> Result<Self> {
        let provider = match database.provider() {
            DatabaseProvider::Sqlite => {
                database.initialize_sqlite_schema(|connection| {
                    connection
                        .execute_batch(GATEWAY_SKEYS_SCHEMA)
                        .context("initialize SQLite gateway skey schema")?;
                    Ok(())
                })?;
                GatewaySkeyStoreProvider::Sqlite(database)
            }
            DatabaseProvider::Postgres => {
                GatewaySkeyStoreProvider::Postgres(PostgresGatewaySkeyStore::new(database)?)
            }
        };
        Ok(Self { provider })
    }

    pub(crate) fn load(&self, tenant: &TenantScope) -> Result<Vec<GatewaySkey>> {
        match &self.provider {
            GatewaySkeyStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                let mut statement = connection.prepare(
                    "SELECT enabled, skey_hash, masked_skey, name, expires_at
                     FROM gateway_skeys
                     WHERE platform_id = ?1 AND company_id = ?2
                     ORDER BY created_at_ms ASC, rowid ASC",
                )?;
                statement
                    .query_map(
                        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()],
                        |row| {
                            Ok(GatewaySkey {
                                enabled: row.get::<_, i64>(0)? != 0,
                                skey: None,
                                skey_hash: row.get(1)?,
                                masked_skey: row.get(2)?,
                                name: row.get(3)?,
                                expires_at: row.get(4)?,
                            })
                        },
                    )?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .context("load tenant SQLite gateway skeys")
            }
            GatewaySkeyStoreProvider::Postgres(store) => store.load(tenant),
        }
    }

    pub(crate) fn replace_all(&self, tenant: &TenantScope, skeys: &[GatewaySkey]) -> Result<()> {
        let normalized = skeys
            .iter()
            .map(normalize_skey)
            .collect::<Result<Vec<_>>>()
            .context("replace tenant gateway skeys")?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let timestamp_ms = current_timestamp_ms()?;
        let result = match &self.provider {
            GatewaySkeyStoreProvider::Sqlite(database) => {
                database.with_sqlite_transaction(TransactionBehavior::Immediate, |transaction| {
                    transaction.execute(
                        "DELETE FROM gateway_skeys WHERE platform_id = ?1 AND company_id = ?2",
                        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()],
                    )?;
                    for skey in &normalized {
                        transaction.execute(
                            "INSERT INTO gateway_skeys (
                                 platform_id, company_id, skey_hash, masked_skey, name, enabled,
                                 expires_at, created_at_ms, updated_at_ms
                             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                            params![
                                tenant.platform_id().as_ref(),
                                tenant.company_id().as_ref(),
                                skey.skey_hash,
                                skey.masked_skey,
                                skey.name,
                                if skey.enabled { 1_i64 } else { 0_i64 },
                                skey.expires_at,
                                timestamp_ms,
                            ],
                        )?;
                    }
                    Ok(())
                })
            }
            GatewaySkeyStoreProvider::Postgres(store) => {
                store.replace_all(tenant, &normalized, timestamp_ms)
            }
        };
        result.context("replace tenant gateway skeys")
    }
}

fn normalize_skey(skey: &GatewaySkey) -> Result<Option<GatewaySkey>> {
    let raw_skey = skey.skey.as_deref().map(str::trim).filter(|raw| !raw.is_empty());
    let skey_hash =
        raw_skey.map(hash_skey).unwrap_or_else(|| skey.skey_hash.trim().to_ascii_lowercase());
    if skey_hash.is_empty() {
        return Ok(None);
    }
    if skey_hash.len() != 64 || !skey_hash.chars().all(|value| value.is_ascii_hexdigit()) {
        bail!("gateway skey hash must be a SHA-256 hex hash");
    }

    let masked_skey =
        raw_skey.map(mask_skey_for_display).unwrap_or_else(|| skey.masked_skey.trim().to_string());
    let configured_name = skey.name.trim();
    let name_contains_raw_skey =
        raw_skey.is_some_and(|raw_skey| configured_name.contains(raw_skey));
    let name = if configured_name.is_empty() || name_contains_raw_skey {
        raw_skey.map(masked_skey_name).unwrap_or_default()
    } else {
        configured_name.to_string()
    };
    let expires_at = skey
        .expires_at
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    if let Some(expires_at) = &expires_at {
        chrono::DateTime::parse_from_rfc3339(expires_at)
            .context("gateway skey expiry must be RFC3339")?;
    }

    Ok(Some(GatewaySkey {
        enabled: skey.enabled,
        skey: None,
        skey_hash,
        masked_skey,
        name,
        expires_at,
    }))
}

fn hash_skey(raw_skey: &str) -> String {
    format!("{:x}", Sha256::digest(raw_skey.trim().as_bytes()))
}

fn mask_skey_for_display(raw_skey: &str) -> String {
    let chars = raw_skey.trim().chars().collect::<Vec<_>>();
    if chars.len() <= 25 {
        return "*".repeat(chars.len().max(1));
    }
    let prefix = chars.iter().take(16).collect::<String>();
    let suffix =
        chars.iter().rev().take(9).collect::<Vec<_>>().into_iter().rev().collect::<String>();
    format!("{prefix}{}{suffix}", "*".repeat(15))
}

fn current_timestamp_ms() -> Result<i64> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?;
    i64::try_from(elapsed.as_millis()).context("system timestamp exceeds database integer range")
}

#[cfg(test)]
#[path = "gateway_skey_store_tests.rs"]
mod gateway_skey_store_tests;

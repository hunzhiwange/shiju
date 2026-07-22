use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, params};
use vw_api_types::id::TenantScope;

use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;

mod postgres;

use postgres::PostgresSystemConfigStore;

const SYSTEM_CONFIGS_SCHEMA: &str = r#"
BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS system_configs (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    payload_json TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (platform_id, company_id)
);

COMMIT;
"#;

#[derive(Debug, Clone)]
pub struct SystemConfigStore {
    provider: SystemConfigStoreProvider,
}

#[derive(Debug, Clone)]
enum SystemConfigStoreProvider {
    Sqlite(GlobalDatabase),
    Postgres(PostgresSystemConfigStore),
}

impl SystemConfigStore {
    pub fn new(database: GlobalDatabase) -> Result<Self> {
        let provider = match database.provider() {
            DatabaseProvider::Sqlite => {
                database.initialize_sqlite_schema(|connection| {
                    connection
                        .execute_batch(SYSTEM_CONFIGS_SCHEMA)
                        .context("initialize SQLite system configs schema")?;
                    Ok(())
                })?;
                SystemConfigStoreProvider::Sqlite(database)
            }
            DatabaseProvider::Postgres => {
                SystemConfigStoreProvider::Postgres(PostgresSystemConfigStore::new(database)?)
            }
        };
        Ok(Self { provider })
    }

    pub fn get(&self, tenant: &TenantScope) -> Result<Option<String>> {
        match &self.provider {
            SystemConfigStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                connection
                    .query_row(
                        "SELECT payload_json
                         FROM system_configs
                         WHERE platform_id = ?1 AND company_id = ?2",
                        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()],
                        |row| row.get(0),
                    )
                    .optional()
                    .context("read tenant SQLite system config")
            }
            SystemConfigStoreProvider::Postgres(store) => store.get(tenant),
        }
    }

    pub fn upsert(&self, tenant: &TenantScope, payload_json: &str) -> Result<()> {
        let updated_at_ms = current_timestamp_ms()?;
        match &self.provider {
            SystemConfigStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                connection
                    .execute(
                        "INSERT INTO system_configs (
                             platform_id, company_id, payload_json, updated_at_ms
                         ) VALUES (?1, ?2, ?3, ?4)
                         ON CONFLICT (platform_id, company_id) DO UPDATE SET
                             payload_json = excluded.payload_json,
                             updated_at_ms = excluded.updated_at_ms",
                        params![
                            tenant.platform_id().as_ref(),
                            tenant.company_id().as_ref(),
                            payload_json,
                            updated_at_ms,
                        ],
                    )
                    .context("upsert tenant SQLite system config")?;
                Ok(())
            }
            SystemConfigStoreProvider::Postgres(store) => {
                store.upsert(tenant, payload_json, updated_at_ms)
            }
        }
    }
}

fn current_timestamp_ms() -> Result<i64> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?;
    i64::try_from(elapsed.as_millis()).context("system timestamp exceeds database integer range")
}

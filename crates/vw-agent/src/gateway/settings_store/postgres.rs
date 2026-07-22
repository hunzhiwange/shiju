use anyhow::{Context, Result, bail};
use vw_api_types::id::TenantScope;

use super::GatewaySetting;
use crate::global_database::GlobalDatabase;

const GATEWAY_SETTINGS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS gateway_settings (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    namespace TEXT NOT NULL CHECK (trim(namespace) <> ''),
    config_key TEXT NOT NULL CHECK (trim(config_key) <> ''),
    payload_json TEXT NOT NULL CHECK (trim(payload_json) <> ''),
    updated_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id, namespace, config_key)
);
"#;

#[derive(Debug, Clone)]
pub(super) struct PostgresGatewaySettingsStore {
    database: GlobalDatabase,
}

impl PostgresGatewaySettingsStore {
    pub(super) fn new(database: GlobalDatabase) -> Result<Self> {
        database.initialize_postgres_schema(|transaction| {
            transaction
                .batch_execute(GATEWAY_SETTINGS_SCHEMA)
                .context("initialize PostgreSQL gateway settings schema")?;
            Ok(())
        })?;
        Ok(Self { database })
    }

    pub(super) fn get(
        &self,
        tenant: &TenantScope,
        namespace: &str,
        config_key: &str,
    ) -> Result<Option<String>> {
        let mut connection = self.database.connect_postgres()?;
        connection
            .query_opt(
                "SELECT payload_json
                 FROM gateway_settings
                 WHERE platform_id = $1 AND company_id = $2
                   AND namespace = $3 AND config_key = $4",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &namespace,
                    &config_key,
                ],
            )
            .context("read tenant PostgreSQL gateway setting")?
            .map(|row| row.try_get(0))
            .transpose()
            .context("decode tenant PostgreSQL gateway setting")
    }

    pub(super) fn upsert(
        &self,
        tenant: &TenantScope,
        namespace: &str,
        config_key: &str,
        payload_json: &str,
        updated_at_ms: i64,
    ) -> Result<()> {
        let mut connection = self.database.connect_postgres()?;
        let changed = connection
            .execute(
                "INSERT INTO gateway_settings (
                     platform_id, company_id, namespace, config_key, payload_json, updated_at_ms
                 ) VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (platform_id, company_id, namespace, config_key) DO UPDATE SET
                     payload_json = excluded.payload_json,
                     updated_at_ms = excluded.updated_at_ms",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &namespace,
                    &config_key,
                    &payload_json,
                    &updated_at_ms,
                ],
            )
            .context("upsert tenant PostgreSQL gateway setting")?;
        if changed != 1 {
            bail!("PostgreSQL gateway setting upsert affected an unexpected row count");
        }
        Ok(())
    }

    pub(super) fn delete(
        &self,
        tenant: &TenantScope,
        namespace: &str,
        config_key: &str,
    ) -> Result<bool> {
        let mut connection = self.database.connect_postgres()?;
        let changed = connection
            .execute(
                "DELETE FROM gateway_settings
                 WHERE platform_id = $1 AND company_id = $2
                   AND namespace = $3 AND config_key = $4",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &namespace,
                    &config_key,
                ],
            )
            .context("delete tenant PostgreSQL gateway setting")?;
        Ok(changed == 1)
    }

    pub(super) fn list(
        &self,
        tenant: &TenantScope,
        namespace: &str,
    ) -> Result<Vec<GatewaySetting>> {
        let mut connection = self.database.connect_postgres()?;
        connection
            .query(
                "SELECT config_key, payload_json
                 FROM gateway_settings
                 WHERE platform_id = $1 AND company_id = $2 AND namespace = $3
                 ORDER BY config_key",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &namespace],
            )
            .context("list tenant PostgreSQL gateway settings")?
            .into_iter()
            .map(|row| {
                Ok(GatewaySetting { config_key: row.try_get(0)?, payload_json: row.try_get(1)? })
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "postgres_tests.rs"]
mod postgres_tests;

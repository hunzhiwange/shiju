//! PostgreSQL 系统配置存储，按租户隔离全局配置与项目覆盖。

use anyhow::{Context, Result, bail};
use vw_api_types::id::TenantScope;

use crate::global_database::GlobalDatabase;

const SYSTEM_CONFIGS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS system_configs (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    payload_json TEXT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id)
);

CREATE TABLE IF NOT EXISTS project_configs (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_path TEXT NOT NULL CHECK (trim(project_path) <> ''),
    payload_json TEXT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_path)
);
"#;

#[derive(Debug, Clone)]
pub(super) struct PostgresSystemConfigStore {
    database: GlobalDatabase,
}

impl PostgresSystemConfigStore {
    pub(super) fn new(database: GlobalDatabase) -> Result<Self> {
        database.initialize_postgres_schema(|transaction| {
            transaction
                .batch_execute(SYSTEM_CONFIGS_SCHEMA)
                .context("initialize PostgreSQL system configs schema")?;
            Ok(())
        })?;
        Ok(Self { database })
    }

    pub(super) fn get(&self, tenant: &TenantScope) -> Result<Option<String>> {
        let mut connection = self.database.connect_postgres()?;
        connection
            .query_opt(
                "SELECT payload_json
                 FROM system_configs
                 WHERE platform_id = $1 AND company_id = $2",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref()],
            )
            .context("read tenant PostgreSQL system config")?
            .map(|row| row.try_get(0))
            .transpose()
            .context("decode tenant PostgreSQL system config")
    }

    pub(super) fn upsert(
        &self,
        tenant: &TenantScope,
        payload_json: &str,
        updated_at_ms: i64,
    ) -> Result<()> {
        let mut connection = self.database.connect_postgres()?;
        let changed = connection
            .execute(
                "INSERT INTO system_configs (
                     platform_id, company_id, payload_json, updated_at_ms
                 ) VALUES ($1, $2, $3, $4)
                 ON CONFLICT (platform_id, company_id) DO UPDATE SET
                     payload_json = excluded.payload_json,
                     updated_at_ms = excluded.updated_at_ms",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &payload_json,
                    &updated_at_ms,
                ],
            )
            .context("upsert tenant PostgreSQL system config")?;
        if changed != 1 {
            bail!("PostgreSQL system config upsert affected an unexpected row count");
        }
        Ok(())
    }

    pub(super) fn get_project(
        &self,
        tenant: &TenantScope,
        project_path: &str,
    ) -> Result<Option<String>> {
        let mut connection = self.database.connect_postgres()?;
        connection
            .query_opt(
                "SELECT payload_json
                 FROM project_configs
                 WHERE platform_id = $1 AND company_id = $2 AND project_path = $3",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &project_path],
            )
            .context("read tenant PostgreSQL project config")?
            .map(|row| row.try_get(0))
            .transpose()
            .context("decode tenant PostgreSQL project config")
    }

    pub(super) fn upsert_project(
        &self,
        tenant: &TenantScope,
        project_path: &str,
        payload_json: &str,
        updated_at_ms: i64,
    ) -> Result<()> {
        let mut connection = self.database.connect_postgres()?;
        let changed = connection
            .execute(
                "INSERT INTO project_configs (
                     platform_id, company_id, project_path, payload_json, updated_at_ms
                 ) VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (platform_id, company_id, project_path) DO UPDATE SET
                     payload_json = excluded.payload_json,
                     updated_at_ms = excluded.updated_at_ms",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &project_path,
                    &payload_json,
                    &updated_at_ms,
                ],
            )
            .context("upsert tenant PostgreSQL project config")?;
        if changed != 1 {
            bail!("PostgreSQL project config upsert affected an unexpected row count");
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "postgres_tests.rs"]
mod postgres_tests;

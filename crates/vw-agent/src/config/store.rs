//! 租户系统配置存储门面，在 SQLite 与 PostgreSQL 后端间保持统一契约。
//!
//! 全局配置按租户键隔离，项目覆盖额外以规范化项目路径区分；空路径会被明确拒绝。

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
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

CREATE TABLE IF NOT EXISTS project_configs (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_path TEXT NOT NULL CHECK (trim(project_path) <> ''),
    payload_json TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_path)
);

COMMIT;
"#;

#[derive(Debug, Clone)]
/// 系统配置的后端无关存储入口。
pub struct SystemConfigStore {
    provider: SystemConfigStoreProvider,
}

#[derive(Debug, Clone)]
enum SystemConfigStoreProvider {
    Sqlite(GlobalDatabase),
    Postgres(PostgresSystemConfigStore),
}

impl SystemConfigStore {
    /// 初始化与数据库提供者匹配的表结构和存储实现。
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

    /// 读取租户全局配置的原始 JSON；记录不存在时返回 `None`。
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

    /// 新增或覆盖租户全局配置，并刷新更新时间。
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

    /// 读取租户在指定项目路径下的覆盖 JSON。
    pub fn get_project(&self, tenant: &TenantScope, project_path: &str) -> Result<Option<String>> {
        validate_project_path(project_path)?;
        match &self.provider {
            SystemConfigStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                connection
                    .query_row(
                        "SELECT payload_json
                         FROM project_configs
                         WHERE platform_id = ?1 AND company_id = ?2 AND project_path = ?3",
                        params![
                            tenant.platform_id().as_ref(),
                            tenant.company_id().as_ref(),
                            project_path,
                        ],
                        |row| row.get(0),
                    )
                    .optional()
                    .context("read tenant SQLite project config")
            }
            SystemConfigStoreProvider::Postgres(store) => store.get_project(tenant, project_path),
        }
    }

    /// 新增或覆盖租户项目配置；空白项目路径会被拒绝。
    pub fn upsert_project(
        &self,
        tenant: &TenantScope,
        project_path: &str,
        payload_json: &str,
    ) -> Result<()> {
        validate_project_path(project_path)?;
        let updated_at_ms = current_timestamp_ms()?;
        match &self.provider {
            SystemConfigStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                connection
                    .execute(
                        "INSERT INTO project_configs (
                             platform_id, company_id, project_path, payload_json, updated_at_ms
                         ) VALUES (?1, ?2, ?3, ?4, ?5)
                         ON CONFLICT (platform_id, company_id, project_path) DO UPDATE SET
                             payload_json = excluded.payload_json,
                             updated_at_ms = excluded.updated_at_ms",
                        params![
                            tenant.platform_id().as_ref(),
                            tenant.company_id().as_ref(),
                            project_path,
                            payload_json,
                            updated_at_ms,
                        ],
                    )
                    .context("upsert tenant SQLite project config")?;
                Ok(())
            }
            SystemConfigStoreProvider::Postgres(store) => {
                store.upsert_project(tenant, project_path, payload_json, updated_at_ms)
            }
        }
    }
}

fn validate_project_path(project_path: &str) -> Result<()> {
    if project_path.trim().is_empty() {
        bail!("project config path must not be empty");
    }
    Ok(())
}

fn current_timestamp_ms() -> Result<i64> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?;
    i64::try_from(elapsed.as_millis()).context("system timestamp exceeds database integer range")
}

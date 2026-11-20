//! PostgreSQL 系统配置存储，按租户隔离全局配置与项目覆盖。
//!
//! 表结构与 SQLite 门面保持相同主键语义，时间戳由上层统一生成后传入。

use anyhow::{Context, Result, bail};
use vw_api_transport::id::TenantScope;

use agent_database::GlobalDatabase;
use secret_store::gateway_skey_store::GatewaySkeyReplacement;

use super::sections::{ConfigSnapshot, DELETE_SECTION, INSERT_TENANT, READ_CONFIG, UPSERT_SECTION};

const SYSTEM_CONFIGS_SCHEMA: &str = include_str!("schema.sql");

#[derive(Debug, Clone)]
/// `SystemConfigStore` 的 PostgreSQL 后端。
pub(super) struct PostgresSystemConfigStore {
    database: GlobalDatabase,
}

impl PostgresSystemConfigStore {
    /// 初始化租户配置表并绑定全局数据库。
    pub(super) fn new(database: GlobalDatabase) -> Result<Self> {
        database.initialize_postgres_schema(|transaction| {
            transaction
                .batch_execute(SYSTEM_CONFIGS_SCHEMA)
                .context("initialize PostgreSQL system configs schema")?;
            Ok(())
        })?;
        Ok(Self { database })
    }

    pub(super) fn read_snapshot(&self, tenant: &TenantScope) -> Result<ConfigSnapshot> {
        let mut connection = self.database.connect_postgres()?;
        let rows = connection
            .query(READ_CONFIG, &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref()])?;
        ConfigSnapshot::from_rows(rows.iter().map(|row| {
            let key = row.try_get(0)?;
            row.try_get(1).map(|payload| (key, payload))
        }))
        .context("read tenant PostgreSQL system config")
    }

    pub(super) fn edit(
        &self,
        tenant: &TenantScope,
        payload_json: &str,
        updated_at_ms: i64,
        can_write: impl FnOnce(&ConfigSnapshot) -> Result<bool>,
        skeys: Option<&GatewaySkeyReplacement>,
    ) -> Result<bool> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction = connection.transaction().context("begin PostgreSQL config write")?;
        let platform = tenant.platform_id().as_ref();
        let company = tenant.company_id().as_ref();
        transaction.batch_execute("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")?;
        // Serialize tenant writes even before its first section exists. At READ COMMITTED
        // the next statement sees any writes committed by the previous lock holder.
        transaction.query_one(
            "SELECT pg_advisory_xact_lock(hashtextextended(
                json_build_array('system_config_sections', current_schema(), $1::text, $2::text)::text, 0
            ))",
            &[&platform, &company],
        ).context("lock tenant PostgreSQL system config")?;
        let rows = transaction.query(READ_CONFIG, &[&platform, &company])?;
        let snapshot = ConfigSnapshot::from_rows(rows.iter().map(|row| {
            let key = row.try_get(0)?;
            row.try_get(1).map(|payload| (key, payload))
        }))?;
        if !can_write(&snapshot)? {
            return Ok(false);
        }
        let changes = snapshot.changes(payload_json)?;
        if changes.initialize {
            transaction.execute(INSERT_TENANT, &[&platform, &company])?;
        }
        for (key, value) in changes.upsert {
            transaction
                .execute(UPSERT_SECTION, &[&platform, &company, &key, &value, &updated_at_ms])?;
        }
        for key in changes.delete {
            transaction.execute(DELETE_SECTION, &[&platform, &company, &key])?;
        }
        if let Some(skeys) = skeys {
            skeys.write_postgres(&mut transaction, tenant, updated_at_ms)?;
        }
        transaction.commit().context("commit PostgreSQL config write")?;
        Ok(true)
    }

    /// 读取指定租户和项目路径的覆盖 JSON。
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

    /// 按租户和项目路径写入项目覆盖。
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

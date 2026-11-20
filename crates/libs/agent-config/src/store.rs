//! 租户系统配置存储门面，在 SQLite 与 PostgreSQL 后端间保持统一契约。
//!
//! 全局配置按租户键隔离，项目覆盖额外以规范化项目路径区分；空路径会被明确拒绝。

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use vw_api_transport::id::TenantScope;

use agent_database::GlobalDatabase;
use agent_database::config::DatabaseProvider;
use secret_store::gateway_skey_store::GatewaySkeyReplacement;

mod postgres;
mod sections;

use postgres::PostgresSystemConfigStore;
use sections::{ConfigSnapshot, DELETE_SECTION, INSERT_TENANT, READ_CONFIG, UPSERT_SECTION};

const SYSTEM_CONFIGS_SCHEMA: &str =
    concat!("BEGIN IMMEDIATE;\n", include_str!("store/schema.sql"), "\nCOMMIT;");

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

    /// 按租户读取并组装系统配置分组。
    pub fn get(&self, tenant: &TenantScope) -> Result<Option<String>> {
        let snapshot = match &self.provider {
            SystemConfigStoreProvider::Sqlite(database) => {
                read_sqlite_snapshot(&database.connect_sqlite()?, tenant)?
            }
            SystemConfigStoreProvider::Postgres(store) => store.read_snapshot(tenant)?,
        };
        snapshot.payload()
    }

    /// 新增或覆盖租户全局配置。
    pub fn upsert(&self, tenant: &TenantScope, payload_json: &str) -> Result<()> {
        self.edit(tenant, payload_json, |_| Ok(true), None)?;
        Ok(())
    }

    /// 仅当租户配置仍匹配读取的快照（或仍不存在）时写入。
    /// 网关密钥更新和配置分组写入在同一事务提交。
    pub(crate) fn compare_and_swap(
        &self,
        tenant: &TenantScope,
        expected: Option<&str>,
        payload_json: &str,
        skeys: Option<&GatewaySkeyReplacement>,
    ) -> Result<bool> {
        self.edit(
            tenant,
            payload_json,
            |snapshot| Ok(snapshot.payload()?.as_deref() == expected),
            skeys,
        )
    }

    fn edit(
        &self,
        tenant: &TenantScope,
        payload_json: &str,
        can_write: impl FnOnce(&ConfigSnapshot) -> Result<bool>,
        skeys: Option<&GatewaySkeyReplacement>,
    ) -> Result<bool> {
        let updated_at_ms = current_timestamp_ms()?;
        match &self.provider {
            SystemConfigStoreProvider::Sqlite(database) => database
                .with_sqlite_transaction(TransactionBehavior::Immediate, |transaction| {
                    let platform = tenant.platform_id().as_ref();
                    let company = tenant.company_id().as_ref();
                    let snapshot = read_sqlite_snapshot(transaction, tenant)?;
                    if !can_write(&snapshot)? {
                        return Ok(false);
                    }
                    let changes = snapshot.changes(payload_json)?;
                    if changes.initialize {
                        transaction.execute(INSERT_TENANT, params![platform, company])?;
                    }
                    for (key, value) in changes.upsert {
                        transaction.execute(
                            UPSERT_SECTION,
                            params![platform, company, key, value, updated_at_ms],
                        )?;
                    }
                    for key in changes.delete {
                        transaction.execute(DELETE_SECTION, params![platform, company, key])?;
                    }
                    if let Some(skeys) = skeys {
                        skeys
                            .write_sqlite(transaction, tenant, updated_at_ms)
                            .context("replace tenant gateway skeys")?;
                    }
                    Ok(true)
                })
                .context("write tenant SQLite system config sections"),
            SystemConfigStoreProvider::Postgres(store) => {
                store.edit(tenant, payload_json, updated_at_ms, can_write, skeys)
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

fn read_sqlite_snapshot(connection: &Connection, tenant: &TenantScope) -> Result<ConfigSnapshot> {
    let mut statement = connection.prepare(READ_CONFIG)?;
    let rows = statement
        .query_map(params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
    ConfigSnapshot::from_rows(rows).context("read tenant SQLite system config")
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

impl crate::schema::channel_reload::ChannelConfigStore for SystemConfigStore {
    fn get(&self, tenant: &TenantScope) -> Result<Option<String>> {
        SystemConfigStore::get(self, tenant)
    }
    fn get_project(&self, tenant: &TenantScope, project: &str) -> Result<Option<String>> {
        SystemConfigStore::get_project(self, tenant, project)
    }
}

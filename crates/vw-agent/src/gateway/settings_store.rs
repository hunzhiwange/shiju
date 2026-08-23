//! 租户网关设置存储门面，按命名空间管理 SQLite 或 PostgreSQL 数据。
//!
//! 命名空间和配置键在进入 SQL 前拒绝空白值，列表结果按配置键稳定排序。

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use rusqlite::{OptionalExtension, params};
use vw_api_types::id::TenantScope;

use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;

mod postgres;

use postgres::PostgresGatewaySettingsStore;

const GATEWAY_SETTINGS_SCHEMA: &str = r#"
BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS gateway_settings (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    namespace TEXT NOT NULL CHECK (trim(namespace) <> ''),
    config_key TEXT NOT NULL CHECK (trim(config_key) <> ''),
    payload_json TEXT NOT NULL CHECK (trim(payload_json) <> ''),
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (platform_id, company_id, namespace, config_key)
);

COMMIT;
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
/// 单条网关设置的键和值。
pub(crate) struct GatewaySetting {
    /// 命名空间内的配置键。
    pub(crate) config_key: String,
    /// 原始 JSON 载荷。
    pub(crate) payload_json: String,
}

#[derive(Debug, Clone)]
/// 后端无关的租户网关设置仓库。
pub(crate) struct GatewaySettingsStore {
    provider: GatewaySettingsStoreProvider,
}

#[derive(Debug, Clone)]
enum GatewaySettingsStoreProvider {
    Sqlite(GlobalDatabase),
    Postgres(PostgresGatewaySettingsStore),
}

impl GatewaySettingsStore {
    /// 初始化与数据库提供者匹配的表结构。
    pub(crate) fn new(database: GlobalDatabase) -> Result<Self> {
        let provider = match database.provider() {
            DatabaseProvider::Sqlite => {
                database.initialize_sqlite_schema(|connection| {
                    connection
                        .execute_batch(GATEWAY_SETTINGS_SCHEMA)
                        .context("initialize SQLite gateway settings schema")?;
                    Ok(())
                })?;
                GatewaySettingsStoreProvider::Sqlite(database)
            }
            DatabaseProvider::Postgres => {
                GatewaySettingsStoreProvider::Postgres(PostgresGatewaySettingsStore::new(database)?)
            }
        };
        Ok(Self { provider })
    }

    /// 读取单条设置；不存在时返回 `None`。
    pub(crate) fn get(
        &self,
        tenant: &TenantScope,
        namespace: &str,
        config_key: &str,
    ) -> Result<Option<String>> {
        validate_identifier(namespace, "gateway settings namespace")?;
        validate_identifier(config_key, "gateway settings config key")?;

        match &self.provider {
            GatewaySettingsStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                connection
                    .query_row(
                        "SELECT payload_json
                         FROM gateway_settings
                         WHERE platform_id = ?1 AND company_id = ?2
                           AND namespace = ?3 AND config_key = ?4",
                        params![
                            tenant.platform_id().as_ref(),
                            tenant.company_id().as_ref(),
                            namespace,
                            config_key,
                        ],
                        |row| row.get(0),
                    )
                    .optional()
                    .context("read tenant SQLite gateway setting")
            }
            GatewaySettingsStoreProvider::Postgres(store) => {
                store.get(tenant, namespace, config_key)
            }
        }
    }

    /// 新增或覆盖一条设置并刷新时间戳。
    pub(crate) fn upsert(
        &self,
        tenant: &TenantScope,
        namespace: &str,
        config_key: &str,
        payload_json: &str,
    ) -> Result<()> {
        validate_identifier(namespace, "gateway settings namespace")?;
        validate_identifier(config_key, "gateway settings config key")?;
        validate_identifier(payload_json, "gateway settings payload")?;

        let updated_at_ms = current_timestamp_ms()?;
        match &self.provider {
            GatewaySettingsStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                connection
                    .execute(
                        "INSERT INTO gateway_settings (
                             platform_id, company_id, namespace, config_key, payload_json,
                             updated_at_ms
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                         ON CONFLICT (platform_id, company_id, namespace, config_key) DO UPDATE SET
                             payload_json = excluded.payload_json,
                             updated_at_ms = excluded.updated_at_ms",
                        params![
                            tenant.platform_id().as_ref(),
                            tenant.company_id().as_ref(),
                            namespace,
                            config_key,
                            payload_json,
                            updated_at_ms,
                        ],
                    )
                    .context("upsert tenant SQLite gateway setting")?;
                Ok(())
            }
            GatewaySettingsStoreProvider::Postgres(store) => {
                store.upsert(tenant, namespace, config_key, payload_json, updated_at_ms)
            }
        }
    }

    /// 删除单条设置并返回是否命中记录。
    pub(crate) fn delete(
        &self,
        tenant: &TenantScope,
        namespace: &str,
        config_key: &str,
    ) -> Result<bool> {
        validate_identifier(namespace, "gateway settings namespace")?;
        validate_identifier(config_key, "gateway settings config key")?;

        match &self.provider {
            GatewaySettingsStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                let changed = connection
                    .execute(
                        "DELETE FROM gateway_settings
                         WHERE platform_id = ?1 AND company_id = ?2
                           AND namespace = ?3 AND config_key = ?4",
                        params![
                            tenant.platform_id().as_ref(),
                            tenant.company_id().as_ref(),
                            namespace,
                            config_key,
                        ],
                    )
                    .context("delete tenant SQLite gateway setting")?;
                Ok(changed == 1)
            }
            GatewaySettingsStoreProvider::Postgres(store) => {
                store.delete(tenant, namespace, config_key)
            }
        }
    }

    /// 按配置键排序列出命名空间内全部设置。
    pub(crate) fn list(
        &self,
        tenant: &TenantScope,
        namespace: &str,
    ) -> Result<Vec<GatewaySetting>> {
        validate_identifier(namespace, "gateway settings namespace")?;

        match &self.provider {
            GatewaySettingsStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                let mut statement = connection.prepare(
                    "SELECT config_key, payload_json
                     FROM gateway_settings
                     WHERE platform_id = ?1 AND company_id = ?2 AND namespace = ?3
                     ORDER BY config_key",
                )?;
                statement
                    .query_map(
                        params![
                            tenant.platform_id().as_ref(),
                            tenant.company_id().as_ref(),
                            namespace,
                        ],
                        |row| {
                            Ok(GatewaySetting {
                                config_key: row.get(0)?,
                                payload_json: row.get(1)?,
                            })
                        },
                    )?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .context("list tenant SQLite gateway settings")
            }
            GatewaySettingsStoreProvider::Postgres(store) => store.list(tenant, namespace),
        }
    }
}

fn validate_identifier(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label} must not be empty");
    }
    Ok(())
}

fn current_timestamp_ms() -> Result<i64> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?;
    i64::try_from(elapsed.as_millis()).context("system timestamp exceeds database integer range")
}

//! 网关密钥元数据的 PostgreSQL 后端。
//!
//! 表结构与查询均强制携带平台和公司作用域，且仅保存密钥摘要。

use anyhow::{Context, Result, bail};
use vw_api_types::id::TenantScope;
use vw_config_types::gateway::GatewaySkey;

use crate::global_database::GlobalDatabase;

const GATEWAY_SKEYS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS gateway_skeys (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    skey_hash TEXT NOT NULL CHECK (trim(skey_hash) <> ''),
    masked_skey TEXT NOT NULL DEFAULT '',
    name TEXT NOT NULL,
    enabled BOOLEAN NOT NULL,
    expires_at TEXT,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id, skey_hash)
);
"#;

#[derive(Debug, Clone)]
/// 网关密钥元数据的 PostgreSQL 存储句柄。
pub(super) struct PostgresGatewaySkeyStore {
    database: GlobalDatabase,
}

impl PostgresGatewaySkeyStore {
    /// 初始化租户化表结构并创建后端。
    pub(super) fn new(database: GlobalDatabase) -> Result<Self> {
        database.initialize_postgres_schema(|transaction| {
            transaction
                .batch_execute(GATEWAY_SKEYS_SCHEMA)
                .context("initialize PostgreSQL gateway skey schema")?;
            Ok(())
        })?;
        Ok(Self { database })
    }

    /// 加载指定租户的掩码密钥元数据。
    pub(super) fn load(&self, tenant: &TenantScope) -> Result<Vec<GatewaySkey>> {
        let mut connection = self.database.connect_postgres()?;
        connection
            .query(
                "SELECT enabled, skey_hash, masked_skey, name, expires_at
                 FROM gateway_skeys
                 WHERE platform_id = $1 AND company_id = $2
                 ORDER BY created_at_ms ASC, skey_hash ASC",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref()],
            )
            .context("load tenant PostgreSQL gateway skeys")?
            .into_iter()
            .map(|row| {
                Ok(GatewaySkey {
                    enabled: row.try_get(0)?,
                    skey: None,
                    skey_hash: row.try_get(1)?,
                    masked_skey: row.try_get(2)?,
                    name: row.try_get(3)?,
                    expires_at: row.try_get(4)?,
                })
            })
            .collect()
    }

    /// 在单个事务内删除并重建指定租户的密钥元数据。
    pub(super) fn replace_all(
        &self,
        tenant: &TenantScope,
        skeys: &[GatewaySkey],
        timestamp_ms: i64,
    ) -> Result<()> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL gateway skey replacement")?;
        transaction
            .execute(
                "DELETE FROM gateway_skeys WHERE platform_id = $1 AND company_id = $2",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref()],
            )
            .context("delete tenant PostgreSQL gateway skeys")?;
        for skey in skeys {
            let changed = transaction
                .execute(
                    "INSERT INTO gateway_skeys (
                         platform_id, company_id, skey_hash, masked_skey, name, enabled,
                         expires_at, created_at_ms, updated_at_ms
                     ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)",
                    &[
                        &tenant.platform_id().as_ref(),
                        &tenant.company_id().as_ref(),
                        &skey.skey_hash,
                        &skey.masked_skey,
                        &skey.name,
                        &skey.enabled,
                        &skey.expires_at,
                        &timestamp_ms,
                    ],
                )
                .context("insert tenant PostgreSQL gateway skey metadata")?;
            if changed != 1 {
                bail!("PostgreSQL gateway skey insert affected an unexpected row count");
            }
        }
        transaction.commit().context("commit PostgreSQL gateway skey replacement")
    }
}

#[cfg(test)]
#[path = "postgres_tests.rs"]
mod postgres_tests;

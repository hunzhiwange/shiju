//! 按租户隔离的 ApiQL 数据库连接配置与密钥持久化。
//!
//! 非敏感元数据写入网关设置，连接 URL 经密钥存储间接引用，避免明文进入配置载荷。
//!
//! Tenant-scoped ApiQL connection persistence.

use anyhow::{Context, Result as AnyhowResult, bail};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::collections::HashSet;
use vw_api_types::id::TenantScope;

use crate::app::agent::gateway::ApiError;
use crate::gateway::settings_store::GatewaySettingsStore;
use crate::global_database::GlobalDatabase;
use crate::security::SecretStore;

const APIQL_CONNECTIONS_NAMESPACE: &str = "apiql.connections";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// ApiQL 支持的数据源类型。
pub(crate) enum ApiQlConnectionKind {
    Sqlite,
    Mysql,
    Postgres,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// 一个可供 ApiQL 查询的租户连接定义。
pub(crate) struct ApiQlConnection {
    /// 租户内稳定的连接标识。
    pub(crate) id: String,
    /// 面向配置界面的名称。
    pub(crate) name: String,
    /// 数据库后端类型。
    pub(crate) kind: ApiQlConnectionKind,
    /// 可选的人类可读说明。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(default = "default_true")]
    /// 是否允许查询该连接。
    pub(crate) enabled: bool,
    #[serde(default)]
    /// 是否限制为只读访问。
    pub(crate) read_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// 非 SQLite 后端的连接地址；持久化时会经密钥存储保护。
    pub(crate) connection_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// SQLite 数据库路径。
    pub(crate) sqlite_path: Option<String>,
    #[serde(default)]
    /// 最后更新时间，单位为毫秒。
    pub(crate) updated_at_ms: u64,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
pub(crate) static APIQL_CONNECTION_STORAGE_TEST_LOCK: tokio::sync::Mutex<()> =
    tokio::sync::Mutex::const_new(());

#[derive(Debug, Clone)]
/// 组合网关设置与密钥存储的租户连接仓库。
pub(crate) struct ApiQlConnectionStore {
    settings_store: GatewaySettingsStore,
    secret_store: SecretStore,
    tenant: TenantScope,
}

impl ApiQlConnectionStore {
    /// 为指定租户创建连接仓库。
    pub(crate) fn new(
        database: GlobalDatabase,
        secret_store: SecretStore,
        tenant: TenantScope,
    ) -> AnyhowResult<Self> {
        Ok(Self { settings_store: GatewaySettingsStore::new(database)?, secret_store, tenant })
    }

    /// 读取连接元数据并回填受保护的连接地址。
    pub(crate) fn load_connections(&self) -> AnyhowResult<Vec<ApiQlConnection>> {
        self.settings_store
            .list(&self.tenant, APIQL_CONNECTIONS_NAMESPACE)?
            .into_iter()
            .map(|setting| self.decode_connection(&setting.config_key, &setting.payload_json))
            .collect()
    }

    #[cfg(test)]
    /// 校验并保存整组连接，同时同步其敏感字段。
    pub(crate) fn save_connections(&self, connections: &[ApiQlConnection]) -> AnyhowResult<()> {
        let mut connection_ids = HashSet::with_capacity(connections.len());
        let mut prepared = Vec::with_capacity(connections.len());
        for connection in connections {
            if !connection_ids.insert(connection.id.as_str()) {
                bail!("duplicate ApiQL connection id");
            }
            let encrypted = self.encrypt_connection(connection)?;
            let payload =
                serde_json::to_string(&encrypted).context("encode ApiQL connection for storage")?;
            prepared.push((connection.id.as_str(), payload));
        }

        let existing = self.settings_store.list(&self.tenant, APIQL_CONNECTIONS_NAMESPACE)?;
        for (connection_id, payload) in prepared {
            self.settings_store.upsert(
                &self.tenant,
                APIQL_CONNECTIONS_NAMESPACE,
                connection_id,
                &payload,
            )?;
        }
        for setting in existing {
            if !connection_ids.contains(setting.config_key.as_str()) {
                self.settings_store.delete(
                    &self.tenant,
                    APIQL_CONNECTIONS_NAMESPACE,
                    &setting.config_key,
                )?;
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn encrypt_connection(&self, connection: &ApiQlConnection) -> AnyhowResult<ApiQlConnection> {
        let mut encrypted = connection.clone();
        if let Some(value) = &mut encrypted.connection_url {
            *value = self.secret_store.encrypt(value).context("encrypt ApiQL connection URL")?;
        }
        Ok(encrypted)
    }

    fn decode_connection(&self, config_key: &str, payload: &str) -> AnyhowResult<ApiQlConnection> {
        let mut connection: ApiQlConnection = serde_json::from_str(payload)
            .context("decode ApiQL connection from shared database")?;
        if connection.id != config_key {
            bail!("ApiQL connection id does not match its config key");
        }
        if let Some(value) = &mut connection.connection_url {
            *value = self.secret_store.decrypt(value).context("decrypt ApiQL connection URL")?;
        }
        Ok(connection)
    }
}

fn internal_storage_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::internal(error.to_string())
}

/// 使用当前全局数据库和密钥存储构建本地连接仓库。
pub(crate) fn local_connection_store(
    tenant: TenantScope,
) -> Result<ApiQlConnectionStore, ApiError> {
    let home = &crate::global::paths().home;
    let database = GlobalDatabase::for_home(home).map_err(internal_storage_error)?;
    let secret_dir = vw_config_types::paths::home_config_dir(home);
    ApiQlConnectionStore::new(database, SecretStore::new(&secret_dir, true), tenant)
        .map_err(internal_storage_error)
}

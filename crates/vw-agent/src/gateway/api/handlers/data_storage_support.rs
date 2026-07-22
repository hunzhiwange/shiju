//! AI-DATA 持久化辅助函数。
//!
//! 该模块把 settings、connections 存入共享数据库，reports 保持文件存储，并提供
//! 排序与时间戳工具，供 HTTP handler 保持一致的数据读写行为。

use std::collections::HashSet;

use anyhow::{Context, Result as AnyhowResult, bail};
use vw_api_types::data::{AiDataConnectionDto, AiDataReportDto, AiDataSettings};
use vw_api_types::id::TenantScope;

use crate::app::agent::gateway::ApiError;
use crate::gateway::settings_store::GatewaySettingsStore;
use crate::global_database::GlobalDatabase;
use crate::security::SecretStore;
use crate::storage;

const AI_DATA_SETTINGS_NAMESPACE: &str = "ai_data.settings";
const AI_DATA_CONNECTIONS_NAMESPACE: &str = "ai_data.connections";
const DEFAULT_CONFIG_KEY: &str = "default";

#[cfg(test)]
pub(crate) static AI_DATA_STORAGE_TEST_LOCK: tokio::sync::Mutex<()> =
    tokio::sync::Mutex::const_new(());

#[derive(Debug, Clone)]
pub(crate) struct AiDataConfigStore {
    settings_store: GatewaySettingsStore,
    secret_store: SecretStore,
    tenant: TenantScope,
}

impl AiDataConfigStore {
    pub(crate) fn new(
        database: GlobalDatabase,
        secret_store: SecretStore,
        tenant: TenantScope,
    ) -> AnyhowResult<Self> {
        Ok(Self { settings_store: GatewaySettingsStore::new(database)?, secret_store, tenant })
    }

    pub(super) fn load_settings_optional(&self) -> AnyhowResult<Option<AiDataSettings>> {
        self.settings_store
            .get(&self.tenant, AI_DATA_SETTINGS_NAMESPACE, DEFAULT_CONFIG_KEY)?
            .map(|payload| {
                serde_json::from_str(&payload)
                    .context("decode AI-DATA settings from shared database")
            })
            .transpose()
    }

    pub(super) fn load_settings(&self) -> AnyhowResult<AiDataSettings> {
        Ok(self.load_settings_optional()?.unwrap_or_default())
    }

    pub(super) fn save_settings(&self, settings: &AiDataSettings) -> AnyhowResult<()> {
        let payload = serde_json::to_string(settings).context("encode AI-DATA settings")?;
        self.settings_store.upsert(
            &self.tenant,
            AI_DATA_SETTINGS_NAMESPACE,
            DEFAULT_CONFIG_KEY,
            &payload,
        )
    }

    #[cfg(test)]
    pub(super) fn delete_settings(&self) -> AnyhowResult<()> {
        self.settings_store.delete(&self.tenant, AI_DATA_SETTINGS_NAMESPACE, DEFAULT_CONFIG_KEY)?;
        Ok(())
    }

    pub(crate) fn load_connections(&self) -> AnyhowResult<Vec<AiDataConnectionDto>> {
        self.settings_store
            .list(&self.tenant, AI_DATA_CONNECTIONS_NAMESPACE)?
            .into_iter()
            .map(|setting| self.decode_connection(&setting.config_key, &setting.payload_json))
            .collect()
    }

    pub(crate) fn save_connections(&self, connections: &[AiDataConnectionDto]) -> AnyhowResult<()> {
        let mut connection_ids = HashSet::with_capacity(connections.len());
        let mut prepared = Vec::with_capacity(connections.len());
        for connection in connections {
            if !connection_ids.insert(connection.id.as_str()) {
                bail!("duplicate AI-DATA connection id");
            }
            let encrypted = self.encrypt_connection(connection)?;
            let payload = serde_json::to_string(&encrypted)
                .context("encode AI-DATA connection for storage")?;
            prepared.push((connection.id.as_str(), payload));
        }

        let existing = self.settings_store.list(&self.tenant, AI_DATA_CONNECTIONS_NAMESPACE)?;
        for (connection_id, payload) in prepared {
            self.settings_store.upsert(
                &self.tenant,
                AI_DATA_CONNECTIONS_NAMESPACE,
                connection_id,
                &payload,
            )?;
        }
        for setting in existing {
            if !connection_ids.contains(setting.config_key.as_str()) {
                self.settings_store.delete(
                    &self.tenant,
                    AI_DATA_CONNECTIONS_NAMESPACE,
                    &setting.config_key,
                )?;
            }
        }
        Ok(())
    }

    fn encrypt_connection(
        &self,
        connection: &AiDataConnectionDto,
    ) -> AnyhowResult<AiDataConnectionDto> {
        let mut encrypted = connection.clone();
        encrypt_optional_secret(
            &self.secret_store,
            &mut encrypted.connection_url,
            "connection URL",
        )?;
        encrypt_optional_secret(&self.secret_store, &mut encrypted.auth_token, "auth token")?;
        for value in encrypted.headers.values_mut() {
            encrypt_secret(&self.secret_store, value, "header value")?;
        }
        Ok(encrypted)
    }

    fn decode_connection(
        &self,
        config_key: &str,
        payload: &str,
    ) -> AnyhowResult<AiDataConnectionDto> {
        let mut connection: AiDataConnectionDto = serde_json::from_str(payload)
            .context("decode AI-DATA connection from shared database")?;
        if connection.id != config_key {
            bail!("AI-DATA connection id does not match its config key");
        }
        decrypt_optional_secret(
            &self.secret_store,
            &mut connection.connection_url,
            "connection URL",
        )?;
        decrypt_optional_secret(&self.secret_store, &mut connection.auth_token, "auth token")?;
        for value in connection.headers.values_mut() {
            decrypt_secret(&self.secret_store, value, "header value")?;
        }
        Ok(connection)
    }
}

fn encrypt_optional_secret(
    store: &SecretStore,
    value: &mut Option<String>,
    field: &str,
) -> AnyhowResult<()> {
    if let Some(value) = value {
        encrypt_secret(store, value, field)?;
    }
    Ok(())
}

fn decrypt_optional_secret(
    store: &SecretStore,
    value: &mut Option<String>,
    field: &str,
) -> AnyhowResult<()> {
    if let Some(value) = value {
        decrypt_secret(store, value, field)?;
    }
    Ok(())
}

fn encrypt_secret(store: &SecretStore, value: &mut String, field: &str) -> AnyhowResult<()> {
    *value = store.encrypt(value).with_context(|| format!("encrypt AI-DATA {field}"))?;
    Ok(())
}

fn decrypt_secret(store: &SecretStore, value: &mut String, field: &str) -> AnyhowResult<()> {
    *value = store.decrypt(value).with_context(|| format!("decrypt AI-DATA {field}"))?;
    Ok(())
}

fn internal_storage_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::internal(error.to_string())
}

pub(crate) fn local_config_store(tenant: TenantScope) -> Result<AiDataConfigStore, ApiError> {
    let home = &crate::global::paths().home;
    let database = GlobalDatabase::for_home(home).map_err(internal_storage_error)?;
    let secret_dir = vw_config_types::paths::home_config_dir(home);
    AiDataConfigStore::new(database, SecretStore::new(&secret_dir, true), tenant)
        .map_err(internal_storage_error)
}

/// 读取 AI-DATA 设置。
///
/// # 返回值
///
/// 读取失败时返回默认设置，避免配置缺失阻断页面初始化。
pub(super) async fn load_settings(tenant: TenantScope) -> AiDataSettings {
    match crate::global_database::run_isolated(move || {
        local_config_store(tenant)
            .and_then(|store| store.load_settings().map_err(internal_storage_error))
    })
    .await
    .map_err(internal_storage_error)
    .and_then(|result| result)
    {
        Ok(settings) => settings,
        Err(error) => {
            tracing::warn!(target: "vw_gateway", error = %error, "failed to load AI-DATA settings");
            AiDataSettings::default()
        }
    }
}

/// 保存 AI-DATA 设置。
///
/// # 错误
///
/// 当底层 storage 写入失败时返回内部错误。
pub(super) async fn save_settings(
    tenant: TenantScope,
    settings: &AiDataSettings,
) -> Result<(), ApiError> {
    let settings = settings.clone();
    crate::global_database::run_isolated(move || {
        local_config_store(tenant)?.save_settings(&settings).map_err(internal_storage_error)
    })
    .await
    .map_err(internal_storage_error)?
}

/// 读取全部 AI-DATA 连接。
pub(super) async fn load_connections(tenant: TenantScope) -> Vec<AiDataConnectionDto> {
    match crate::global_database::run_isolated(move || {
        local_config_store(tenant)
            .and_then(|store| store.load_connections().map_err(internal_storage_error))
    })
    .await
    .map_err(internal_storage_error)
    .and_then(|result| result)
    {
        Ok(connections) => connections,
        Err(error) => {
            tracing::warn!(target: "vw_gateway", error = %error, "failed to load AI-DATA connections");
            Vec::new()
        }
    }
}

/// 保存全部 AI-DATA 连接。
///
/// # 错误
///
/// 当底层 storage 写入失败时返回内部错误。
pub(super) async fn save_connections(
    tenant: TenantScope,
    connections: &[AiDataConnectionDto],
) -> Result<(), ApiError> {
    let connections = connections.to_vec();
    crate::global_database::run_isolated(move || {
        local_config_store(tenant)?.save_connections(&connections).map_err(internal_storage_error)
    })
    .await
    .map_err(internal_storage_error)?
}

/// 读取全部 AI-DATA 报表。
pub(super) async fn load_reports() -> Vec<AiDataReportDto> {
    storage::read(&["ai_data", "reports"]).await.unwrap_or_default()
}

/// 保存全部 AI-DATA 报表。
///
/// # 错误
///
/// 当底层 storage 写入失败时返回内部错误。
pub(super) async fn save_reports(reports: &[AiDataReportDto]) -> Result<(), ApiError> {
    let owned = reports.to_vec();
    storage::write(&["ai_data", "reports"], &owned)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))
}

/// 按最近使用或更新时间倒序排列连接。
pub(super) fn sort_connections(connections: &mut [AiDataConnectionDto]) {
    connections.sort_by(|left, right| {
        let left_key = left.last_used_ms.unwrap_or(left.updated_at_ms);
        let right_key = right.last_used_ms.unwrap_or(right.updated_at_ms);
        right_key.cmp(&left_key).then_with(|| left.name.cmp(&right.name))
    });
}

/// 按更新时间倒序排列报表。
pub(super) fn sort_reports(reports: &mut [AiDataReportDto]) {
    reports.sort_by(|left, right| {
        right.updated_at_ms.cmp(&left.updated_at_ms).then_with(|| left.name.cmp(&right.name))
    });
}

/// 返回当前 Unix 毫秒时间戳。
///
/// # 返回值
///
/// 系统时间异常或数值溢出时返回安全的兜底值。
pub(super) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
#[path = "data_storage_support_tests.rs"]
mod data_storage_support_tests;

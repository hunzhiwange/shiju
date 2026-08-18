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
pub(crate) enum ApiQlConnectionKind {
    Sqlite,
    Mysql,
    Postgres,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ApiQlConnection {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) kind: ApiQlConnectionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(default = "default_true")]
    pub(crate) enabled: bool,
    #[serde(default)]
    pub(crate) read_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) connection_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) sqlite_path: Option<String>,
    #[serde(default)]
    pub(crate) updated_at_ms: u64,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
pub(crate) static APIQL_CONNECTION_STORAGE_TEST_LOCK: tokio::sync::Mutex<()> =
    tokio::sync::Mutex::const_new(());

#[derive(Debug, Clone)]
pub(crate) struct ApiQlConnectionStore {
    settings_store: GatewaySettingsStore,
    secret_store: SecretStore,
    tenant: TenantScope,
}

impl ApiQlConnectionStore {
    pub(crate) fn new(
        database: GlobalDatabase,
        secret_store: SecretStore,
        tenant: TenantScope,
    ) -> AnyhowResult<Self> {
        Ok(Self { settings_store: GatewaySettingsStore::new(database)?, secret_store, tenant })
    }

    pub(crate) fn load_connections(&self) -> AnyhowResult<Vec<ApiQlConnection>> {
        self.settings_store
            .list(&self.tenant, APIQL_CONNECTIONS_NAMESPACE)?
            .into_iter()
            .map(|setting| self.decode_connection(&setting.config_key, &setting.payload_json))
            .collect()
    }

    #[cfg(test)]
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

pub(crate) fn local_connection_store(
    tenant: TenantScope,
) -> Result<ApiQlConnectionStore, ApiError> {
    let home = &crate::global::paths().home;
    let database = GlobalDatabase::for_home(home).map_err(internal_storage_error)?;
    let secret_dir = vw_config_types::paths::home_config_dir(home);
    ApiQlConnectionStore::new(database, SecretStore::new(&secret_dir, true), tenant)
        .map_err(internal_storage_error)
}

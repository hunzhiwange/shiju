//! Persistence for the managed MCP catalog.

use serde::{Deserialize, Serialize};
use vw_api_types::{
    id::TenantScope,
    mcp_management::{
        McpCatalogRecord, McpCatalogScope, McpCliBinding, McpConnectionDefinition, McpFailureCode,
    },
};

use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};

mod catalog_create;
mod catalog_delete;
mod catalog_update;

#[cfg(test)]
mod catalog_create_tests;
#[cfg(test)]
mod catalog_delete_tests;
#[cfg(test)]
mod catalog_update_tests;

pub use catalog_delete::McpRevocation;

#[derive(Deserialize)]
#[serde(untagged)]
enum StoredCatalogEntry {
    Active(Box<StoredCatalogRecord>),
    Deleted(catalog_delete::DeletedCatalogRecord),
}

#[derive(Deserialize, Serialize)]
struct StoredCatalogRecord {
    record: McpCatalogRecord,
    connection: McpConnectionDefinition,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    bindings: Vec<McpCliBinding>,
}

pub struct McpCatalogStore(GatewaySettingsStore);

impl McpCatalogStore {
    pub fn new(database: GlobalDatabase) -> Result<Self, McpFailureCode> {
        GatewaySettingsStore::new(database)
            .map(Self)
            .map_err(|_| McpFailureCode::StorageUnavailable)
    }

    /// List the public projections in the authorized scope, excluding deleted entries.
    pub fn list(
        &self,
        authorized_scope: &McpCatalogScope,
    ) -> Result<Vec<McpCatalogRecord>, McpFailureCode> {
        let (tenant, namespace) = catalog_create::catalog_location(authorized_scope)?;
        let rows =
            self.0.list(tenant, &namespace).map_err(|_| McpFailureCode::StorageUnavailable)?;
        let mut records = Vec::new();
        for row in rows {
            let entry = serde_json::from_str(&row.payload_json)
                .map_err(|_| McpFailureCode::StorageUnavailable)?;
            if let StoredCatalogEntry::Active(stored) = entry {
                records.push(stored.record);
            }
        }
        Ok(records)
    }

    fn load_entry(
        &self,
        tenant: &TenantScope,
        namespace: &str,
        key: &str,
    ) -> Result<(String, StoredCatalogEntry), McpFailureCode> {
        let payload = self
            .0
            .get(tenant, namespace, key)
            .map_err(|_| McpFailureCode::StorageUnavailable)?
            .ok_or(McpFailureCode::NotFound)?;
        let entry =
            serde_json::from_str(&payload).map_err(|_| McpFailureCode::StorageUnavailable)?;
        Ok((payload, entry))
    }

    fn save_entry(
        &self,
        tenant: &TenantScope,
        namespace: &str,
        key: &str,
        previous: &str,
        entry: &impl Serialize,
    ) -> Result<(), McpFailureCode> {
        let payload =
            serde_json::to_string(entry).map_err(|_| McpFailureCode::StorageUnavailable)?;
        if !self
            .0
            .compare_exchange(tenant, namespace, key, previous, &payload)
            .map_err(|_| McpFailureCode::StorageUnavailable)?
        {
            return Err(McpFailureCode::StaleRevision);
        }
        Ok(())
    }
}

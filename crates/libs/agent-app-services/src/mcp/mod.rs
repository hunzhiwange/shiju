//! Managed MCP catalog storage and authorized native application projection.
use serde::{Deserialize, Serialize};
use vw_api_transport::id::TenantScope;
use vw_api_mcp_management::{McpCatalogRecord, McpCatalogScope, McpCliBinding, McpConnectionDefinition, McpFailureCode};
use app_settings_store::settings_store::GatewaySettingsStore;
use agent_database::GlobalDatabase;
mod catalog_create;
mod catalog_delete;
mod catalog_update;
pub mod credentials;
pub use catalog_delete::McpRevocation;
#[cfg(not(target_arch = "wasm32"))]
pub use credentials::McpExecutionConnection;
#[cfg(target_os = "macos")]
pub mod binding;
#[cfg(target_os = "macos")]
pub mod bulk;
#[cfg(target_os = "macos")]
pub mod data_migration;
#[cfg(target_os = "macos")]
pub mod import;
#[cfg(target_os = "macos")]
pub mod resync;
#[cfg(target_os = "macos")]
pub mod profile_slot;
#[cfg(target_os = "macos")]
pub use binding::McpBindingTarget;
#[cfg(target_os = "macos")]
pub use import::{McpImportEntry, McpImportOutcome, McpImportReport, McpImportSource};
#[cfg(target_os = "macos")]
pub use data_migration::{McpMigration, McpMigrationCommit};
#[cfg(target_os = "macos")]
pub use resync::{McpResyncResult, McpResyncTargetResult, McpResyncTask, resync_targets};
#[cfg(all(test, target_os = "macos"))]
mod binding_tests;
#[cfg(all(test, target_os = "macos"))]
mod bulk_tests;
#[cfg(test)]
mod catalog_create_tests;
#[cfg(test)]
mod catalog_delete_tests;
#[cfg(test)]
mod catalog_update_tests;
#[cfg(test)]
mod credentials_tests;
#[cfg(all(test, target_os = "macos"))]
mod data_migration_tests;
#[cfg(all(test, target_os = "macos"))]
mod import_tests;
#[cfg(all(test, target_os = "macos"))]
mod resync_tests;

#[derive(Deserialize)]
#[serde(untagged)]
enum StoredCatalogEntry {
    Active(Box<StoredCatalogRecord>),
    Deleted(catalog_delete::DeletedCatalogRecord),
}

#[derive(Deserialize, Serialize)]
pub struct StoredCatalogRecord {
    pub record: McpCatalogRecord,
    pub connection: McpConnectionDefinition,
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
        self.read_catalog(authorized_scope, None).map(|(records, _)| records)
    }

    /// Return the catalog and desired bindings visible on the authorized host in one read.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn list_for_host(
        &self,
        authorized_scope: &McpCatalogScope,
        host: &str,
    ) -> Result<(Vec<McpCatalogRecord>, Vec<McpCliBinding>), McpFailureCode> {
        self.read_catalog(authorized_scope, Some(host))
    }

    fn read_catalog(
        &self,
        authorized_scope: &McpCatalogScope,
        host: Option<&str>,
    ) -> Result<(Vec<McpCatalogRecord>, Vec<McpCliBinding>), McpFailureCode> {
        let (tenant, namespace) = catalog_create::catalog_location(authorized_scope)?;
        let rows =
            self.0.list(tenant, &namespace).map_err(|_| McpFailureCode::StorageUnavailable)?;
        let mut records = Vec::new();
        let mut bindings = Vec::new();
        for row in rows {
            let entry = serde_json::from_str(&row.payload_json)
                .map_err(|_| McpFailureCode::StorageUnavailable)?;
            if let StoredCatalogEntry::Active(stored) = entry {
                records.push(stored.record);
                if let Some(host) = host {
                    bindings.extend(
                        stored
                            .bindings
                            .into_iter()
                            .filter(|binding| binding.target.target_host_id == host),
                    );
                }
            }
        }
        Ok((records, bindings))
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

#[cfg(not(target_arch = "wasm32"))]
pub mod runtime;
#[cfg(not(target_arch = "wasm32"))]
pub mod revocation;
#[cfg(not(target_arch = "wasm32"))]
mod probe_snapshot;

#[cfg(not(target_arch = "wasm32"))]
pub mod connectivity;
#[cfg(not(target_arch = "wasm32"))]
pub mod connection_runtime;

pub mod acp_mount;

#[cfg(target_os = "macos")]
pub mod network_probe;
#[cfg(target_os = "macos")]
pub mod probe_host;
#[cfg(not(target_arch = "wasm32"))]
pub mod probe_jobs;
#[cfg(not(target_arch = "wasm32"))]
pub mod probe_jobs_host;

#[cfg(all(test, target_os = "macos"))]
mod host_migration_48_tests;

#[cfg(all(feature = "test-support", target_os = "macos"))]
pub mod acceptance;

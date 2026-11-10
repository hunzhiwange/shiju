//! Persistence for the managed MCP catalog.

use serde::{Deserialize, Serialize};
use vw_api_types::mcp_management::{McpCatalogRecord, McpConnectionDefinition, McpFailureCode};

use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};

mod catalog_create;
mod catalog_update;

#[cfg(test)]
mod catalog_create_tests;
#[cfg(test)]
mod catalog_update_tests;

#[derive(Deserialize, Serialize)]
struct StoredCatalogRecord {
    record: McpCatalogRecord,
    connection: McpConnectionDefinition,
}

pub struct McpCatalogStore(GatewaySettingsStore);

impl McpCatalogStore {
    pub fn new(database: GlobalDatabase) -> Result<Self, McpFailureCode> {
        GatewaySettingsStore::new(database)
            .map(Self)
            .map_err(|_| McpFailureCode::StorageUnavailable)
    }
}

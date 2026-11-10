//! Scoped creation of MCP catalog records in the gateway database.

use serde::Serialize;
use vw_agent_apps::mcp::catalog::McpCatalogDraft;
use vw_api_types::{
    id::TenantScope,
    mcp_management::{McpCatalogRecord, McpCatalogScope, McpConnectionDefinition, McpFailureCode},
};

use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};

const CATALOG_NAMESPACE: &str = "agent_apps.mcp.catalog.v1";

#[derive(Serialize)]
struct StoredCatalogRecord<'a> {
    record: &'a McpCatalogRecord,
    connection: &'a McpConnectionDefinition,
}

pub struct McpCatalogStore(GatewaySettingsStore);

impl McpCatalogStore {
    pub fn new(database: GlobalDatabase) -> Result<Self, McpFailureCode> {
        GatewaySettingsStore::new(database)
            .map(Self)
            .map_err(|_| McpFailureCode::StorageUnavailable)
    }

    /// Save only the catalog definition in the scope already authorized by the caller.
    pub fn create(
        &self,
        authorized_scope: &McpCatalogScope,
        draft: McpCatalogDraft,
    ) -> Result<McpCatalogRecord, McpFailureCode> {
        let (record, connection) = draft.into_record(authorized_scope.clone())?;
        let (tenant, namespace) = catalog_location(authorized_scope)?;
        let payload = serde_json::to_string(&StoredCatalogRecord {
            record: &record,
            connection: &connection,
        })
        .map_err(|_| McpFailureCode::StorageUnavailable)?;
        let inserted = self
            .0
            .insert_if_absent(tenant, &namespace, &record.record.id.0, &payload)
            .map_err(|_| McpFailureCode::StorageUnavailable)?;
        if !inserted {
            return Err(McpFailureCode::Conflict);
        }
        Ok(record)
    }
}

pub(super) fn catalog_location(
    scope: &McpCatalogScope,
) -> Result<(&TenantScope, String), McpFailureCode> {
    match scope {
        McpCatalogScope::Tenant { scope } => Ok((scope, CATALOG_NAMESPACE.into())),
        McpCatalogScope::Project { scope } => {
            let project = serde_json::to_string(scope.project_id())
                .map_err(|_| McpFailureCode::InvalidDefinition)?;
            Ok((scope.tenant_scope(), format!("{CATALOG_NAMESPACE}.project.{project}")))
        }
    }
}

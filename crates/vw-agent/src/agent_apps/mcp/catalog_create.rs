//! Scoped creation of MCP catalog records in the gateway database.

use vw_agent_apps::mcp::catalog::McpCatalogDraft;
use vw_api_types::{
    id::TenantScope,
    mcp_management::{McpCatalogRecord, McpCatalogScope, McpFailureCode},
};

use super::{McpCatalogStore, StoredCatalogRecord};

const CATALOG_NAMESPACE: &str = "agent_apps.mcp.catalog.v1";

impl McpCatalogStore {
    /// Save only the catalog definition in the scope already authorized by the caller.
    pub fn create(
        &self,
        authorized_scope: &McpCatalogScope,
        draft: McpCatalogDraft,
    ) -> Result<McpCatalogRecord, McpFailureCode> {
        let (record, connection) = draft.into_record(authorized_scope.clone())?;
        let (tenant, namespace) = catalog_location(authorized_scope)?;
        let stored = StoredCatalogRecord { record, connection };
        let payload =
            serde_json::to_string(&stored).map_err(|_| McpFailureCode::StorageUnavailable)?;
        let inserted = self
            .0
            .insert_if_absent(tenant, &namespace, &stored.record.record.id.0, &payload)
            .map_err(|_| McpFailureCode::StorageUnavailable)?;
        if !inserted {
            return Err(McpFailureCode::Conflict);
        }
        Ok(stored.record)
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

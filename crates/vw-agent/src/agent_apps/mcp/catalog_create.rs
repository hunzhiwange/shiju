//! Scoped creation of MCP catalog records in the gateway database.

use vw_agent_apps::mcp::catalog::McpCatalogDraft;
use vw_api_types::{
    id::TenantScope,
    mcp_management::{McpCatalogRecord, McpCatalogScope, McpFailureCode},
};

use super::{
    McpCatalogStore, StoredCatalogEntry, StoredCatalogRecord,
    credentials::validate_catalog_connection,
};

const CATALOG_NAMESPACE: &str = "agent_apps.mcp.catalog.v1";

impl McpCatalogStore {
    /// Save only the catalog definition in the scope already authorized by the caller.
    pub fn create(
        &self,
        authorized_scope: &McpCatalogScope,
        draft: McpCatalogDraft,
    ) -> Result<McpCatalogRecord, McpFailureCode> {
        let (record, connection) = draft.into_record(authorized_scope.clone())?;
        validate_catalog_connection(&connection)?;
        let (tenant, namespace) = catalog_location(authorized_scope)?;
        let mut stored = StoredCatalogRecord { record, connection, bindings: Vec::new() };
        let key = &stored.record.record.id.0;
        let payload =
            serde_json::to_string(&stored).map_err(|_| McpFailureCode::StorageUnavailable)?;
        let inserted = self
            .0
            .insert_if_absent(tenant, &namespace, key, &payload)
            .map_err(|_| McpFailureCode::StorageUnavailable)?;
        if inserted {
            return Ok(stored.record);
        }

        // Keep the retired revision as a fence for late cleanup reports.
        let (previous, entry) = self.load_entry(tenant, &namespace, key)?;
        let StoredCatalogEntry::Deleted(deleted) = entry else {
            return Err(McpFailureCode::Conflict);
        };
        if deleted.revocations.iter().any(|item| item.outcome != Some(Ok(()))) {
            return Err(McpFailureCode::Conflict);
        }
        stored.record.record.revision =
            deleted.deleted.revision.checked_add(1).ok_or(McpFailureCode::StorageUnavailable)?;
        stored.record.connection_revision = stored.record.record.revision;
        self.save_entry(tenant, &namespace, key, &previous, &stored).map_err(
            |code| match code {
                McpFailureCode::StaleRevision => McpFailureCode::Conflict,
                other => other,
            },
        )?;
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

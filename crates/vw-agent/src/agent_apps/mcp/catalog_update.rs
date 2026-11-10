//! Revision-checked updates of the private connection definition.

use vw_api_types::mcp_management::{
    McpCatalogRecord, McpCatalogScope, McpConnectionDefinition, McpFailureCode, McpRecordRef,
    McpTransport,
};

use super::{
    McpCatalogStore, StoredCatalogEntry,
    catalog_create::catalog_location,
    credentials::{retain_credentials, validate_catalog_connection},
};

impl McpCatalogStore {
    /// Update the connection in the caller-authorized scope, preserving display metadata.
    /// Omitted or masked credential entries are kept when the transport uses the same config field.
    pub fn update_connection(
        &self,
        authorized_scope: &McpCatalogScope,
        expected: &McpRecordRef,
        mut connection: McpConnectionDefinition,
    ) -> Result<McpCatalogRecord, McpFailureCode> {
        let (tenant, namespace) = catalog_location(authorized_scope)?;
        let key = &expected.id.0;
        #[cfg(target_os = "macos")]
        let _lock = super::binding::lock_record(key);
        let (previous, entry) = self.load_entry(tenant, &namespace, key)?;
        let StoredCatalogEntry::Active(mut stored) = entry else {
            return Err(McpFailureCode::NotFound);
        };
        if stored.record.record.revision != expected.revision {
            return Err(McpFailureCode::StaleRevision);
        }

        match (&stored.connection, &mut connection) {
            (
                McpConnectionDefinition::Stdio { env: previous, .. },
                McpConnectionDefinition::Stdio { env, .. },
            ) => retain_credentials(previous, env, str::eq),
            (
                McpConnectionDefinition::StreamableHttp { headers: previous, .. }
                | McpConnectionDefinition::Sse { headers: previous, .. },
                McpConnectionDefinition::StreamableHttp { headers, .. }
                | McpConnectionDefinition::Sse { headers, .. },
            ) => retain_credentials(previous, headers, str::eq_ignore_ascii_case),
            _ => {}
        }
        validate_catalog_connection(&connection)?;
        let record = &mut stored.record;
        record.record.revision =
            record.record.revision.checked_add(1).ok_or(McpFailureCode::StorageUnavailable)?;
        if stored.connection != connection {
            record.connection_revision = record
                .connection_revision
                .checked_add(1)
                .ok_or(McpFailureCode::StorageUnavailable)?;
        }
        record.transport = match connection {
            McpConnectionDefinition::Stdio { .. } => McpTransport::Stdio,
            McpConnectionDefinition::StreamableHttp { .. } => McpTransport::StreamableHttp,
            McpConnectionDefinition::Sse { .. } => McpTransport::Sse,
        };
        stored.connection = connection;

        self.save_entry(tenant, &namespace, key, &previous, &stored)?;
        Ok(stored.record)
    }
}

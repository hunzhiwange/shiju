//! Revision-checked updates of the private connection definition.

use std::collections::BTreeMap;

use vw_agent_apps::mcp::validation::validate_connection;
use vw_api_types::mcp_management::{
    McpCatalogRecord, McpCatalogScope, McpConfigValue, McpConnectionDefinition, McpFailureCode,
    McpRecordRef, McpTransport,
};

use super::{McpCatalogStore, StoredCatalogRecord, catalog_create::catalog_location};

impl McpCatalogStore {
    /// Update the connection in the caller-authorized scope, preserving display metadata.
    /// Omitted credential entries are kept when the transport uses the same config field.
    pub fn update_connection(
        &self,
        authorized_scope: &McpCatalogScope,
        expected: &McpRecordRef,
        mut connection: McpConnectionDefinition,
    ) -> Result<McpCatalogRecord, McpFailureCode> {
        let (tenant, namespace) = catalog_location(authorized_scope)?;
        let key = &expected.id.0;
        let previous = self
            .0
            .get(tenant, &namespace, key)
            .map_err(|_| McpFailureCode::StorageUnavailable)?
            .ok_or(McpFailureCode::NotFound)?;
        let mut stored: StoredCatalogRecord =
            serde_json::from_str(&previous).map_err(|_| McpFailureCode::StorageUnavailable)?;
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
        validate_connection(&connection).map_err(|_| McpFailureCode::InvalidDefinition)?;
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

        let payload =
            serde_json::to_string(&stored).map_err(|_| McpFailureCode::StorageUnavailable)?;
        let updated = self
            .0
            .compare_exchange(tenant, &namespace, key, &previous, &payload)
            .map_err(|_| McpFailureCode::StorageUnavailable)?;
        if !updated {
            return Err(McpFailureCode::StaleRevision);
        }
        Ok(stored.record)
    }
}

fn retain_credentials(
    previous: &BTreeMap<String, McpConfigValue>,
    updated: &mut BTreeMap<String, McpConfigValue>,
    same_key: fn(&str, &str) -> bool,
) {
    for (key, value) in previous {
        if matches!(value, McpConfigValue::Credential { .. })
            && !updated.keys().any(|new_key| same_key(key, new_key))
        {
            updated.insert(key.clone(), value.clone());
        }
    }
}

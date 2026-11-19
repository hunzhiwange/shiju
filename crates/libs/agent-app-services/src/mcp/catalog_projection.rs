//! Only exact, complete static presets may leave private storage as importable JSON.
use super::{McpCatalogStore, StoredCatalogEntry, catalog_create::catalog_location};
use vw_agent_apps::mcp::presets;
use vw_api_mcp_management::*;

fn public_preset(connection: &McpConnectionDefinition) -> Option<McpConnectionDefinition> {
    presets::list_presets().iter().find_map(|preset| {
        let draft = presets::create_preset_draft(preset.id, &[]).ok()??;
        (draft.connection == *connection).then_some(draft.connection)
    })
}

pub(super) fn project_summary(record: &mut McpCatalogRecord, connection: &McpConnectionDefinition) {
    if record.public_summary.is_none()
        && let Some(McpConnectionDefinition::Stdio { command, args, .. }) =
            public_preset(connection)
    {
        record.public_summary =
            Some(std::iter::once(command).chain(args).collect::<Vec<_>>().join(" "));
    }
}

impl McpCatalogStore {
    /// None selects all active records; Some selects exactly one ID within the authorized scope.
    pub fn safe_export(
        &self,
        scope: &McpCatalogScope,
        id: Option<&str>,
    ) -> Result<McpSafeExportResponse, McpFailureCode> {
        let (tenant, namespace) = catalog_location(scope)?;
        let rows =
            self.0.list(tenant, &namespace).map_err(|_| McpFailureCode::StorageUnavailable)?;
        let mut servers = serde_json::Map::new();
        let mut skipped = Vec::new();
        let mut found = false;
        for row in rows {
            let entry: StoredCatalogEntry = serde_json::from_str(&row.payload_json)
                .map_err(|_| McpFailureCode::StorageUnavailable)?;
            let StoredCatalogEntry::Active(stored) = entry else { continue };
            let key = stored.record.record.id;
            if id.is_some_and(|id| id != key.0) {
                continue;
            }
            found = true;
            match public_preset(&stored.connection) {
                Some(McpConnectionDefinition::Stdio { command, args, .. }) => {
                    servers.insert(key.0, serde_json::json!({"command": command, "args": args}));
                }
                _ => skipped.push(McpExportSkipped {
                    id: key,
                    reason: McpExportSkipReason::ProtectedDefinition,
                }),
            }
        }
        if !found && let Some(id) = id {
            skipped.push(McpExportSkipped {
                id: McpRecordId(id.into()),
                reason: McpExportSkipReason::NotFound,
            });
        }
        let empty = servers.is_empty();
        let json = if empty {
            None
        } else {
            Some(
                serde_json::to_string_pretty(&serde_json::json!({"mcpServers": servers}))
                    .map_err(|_| McpFailureCode::StorageUnavailable)?,
            )
        };
        Ok(McpSafeExportResponse {
            json,
            skipped,
            reason: empty.then_some(McpExportSkipReason::NoPublicDefinitions),
        })
    }
}

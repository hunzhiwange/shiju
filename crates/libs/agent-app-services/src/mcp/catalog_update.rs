//! Revision-checked metadata edits and private connection replacement.

use vw_api_mcp_management::{
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
        connection: McpConnectionDefinition,
    ) -> Result<McpCatalogRecord, McpFailureCode> {
        self.update_record(
            authorized_scope,
            expected,
            Some(connection),
            Default::default(),
            Default::default(),
        )
    }

    pub fn update_record(
        &self,
        authorized_scope: &McpCatalogScope,
        expected: &McpRecordRef,
        connection: Option<McpConnectionDefinition>,
        metadata: vw_api_mcp_management::McpMetadataPatch,
        summary: vw_api_mcp_management::McpPublicSummaryEdit,
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

        let mut connection = connection.unwrap_or_else(|| stored.connection.clone());
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
        let changed_connection = stored.connection != connection;
        use vw_api_mcp_management::{McpPublicSummaryEdit, McpTextEdit};
        fn edit(current: Option<String>, edit: McpTextEdit) -> Option<String> {
            match edit {
                McpTextEdit::Keep => current,
                McpTextEdit::Set(value) => Some(value),
                McpTextEdit::Clear => None,
            }
        }
        let draft = vw_agent_apps::mcp::catalog::McpCatalogDraft {
            id: key.clone(),
            name: edit(Some(stored.record.name.clone()), metadata.name),
            description: edit(stored.record.description.clone(), metadata.description),
            homepage: edit(stored.record.homepage.clone(), metadata.homepage),
            documentation: edit(stored.record.documentation.clone(), metadata.documentation),
            tags: metadata.tags.unwrap_or_else(|| stored.record.tags.clone()),
            connection: connection.clone(),
        };
        let (validated, _) = draft.into_record(authorized_scope.clone())?;
        let record = &mut stored.record;
        record.name = validated.name;
        record.description = validated.description;
        record.homepage = validated.homepage;
        record.documentation = validated.documentation;
        record.tags = validated.tags;
        if changed_connection {
            record.public_summary = None;
            super::catalog_projection::project_summary(record, &connection);
        }
        match summary {
            McpPublicSummaryEdit::Keep => {}
            McpPublicSummaryEdit::Clear => record.public_summary = None,
            McpPublicSummaryEdit::Set(value) => {
                // Deliberate public text only. Never derive it from the private connection.
                if value.is_empty()
                    || value.len() > 512
                    || value.trim() != value
                    || value.chars().any(char::is_control)
                {
                    return Err(McpFailureCode::InvalidDefinition);
                }
                // Address displays follow the same public-link policy as metadata.
                if !matches!(&connection, McpConnectionDefinition::Stdio { .. }) {
                    let check = vw_agent_apps::mcp::catalog::McpCatalogDraft {
                        id: key.clone(),
                        name: None,
                        description: None,
                        homepage: Some(value.clone()),
                        documentation: None,
                        tags: vec![],
                        connection: connection.clone(),
                    };
                    check.into_record(authorized_scope.clone())?;
                }
                record.public_summary = Some(value);
            }
        }

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

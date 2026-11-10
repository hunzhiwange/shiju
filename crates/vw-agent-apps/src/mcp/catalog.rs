//! Construction and validation of new managed MCP records.

use reqwest::Url;
use vw_api_types::mcp_management::{
    McpCatalogRecord, McpCatalogScope, McpConnectionDefinition, McpFailureCode, McpRecordRef,
    McpTransport,
};

use super::validation::{validate_connection, validate_new_name};

/// Input for creating one catalog record. The caller supplies the authorized scope separately.
pub struct McpCatalogDraft {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub tags: Vec<String>,
    pub connection: McpConnectionDefinition,
}

impl McpCatalogDraft {
    /// Validate the definition and separate the public record from the private connection.
    pub fn into_record(
        self,
        scope: McpCatalogScope,
    ) -> Result<(McpCatalogRecord, McpConnectionDefinition), McpFailureCode> {
        let id = validate_new_name(&self.id, &[]).map_err(|_| McpFailureCode::InvalidDefinition)?;
        validate_connection(&self.connection).map_err(|_| McpFailureCode::InvalidDefinition)?;

        let name = self.name.as_deref().map(str::trim).filter(|name| !name.is_empty());
        if name.is_some_and(|name| name.chars().any(char::is_control))
            || self.description.as_ref().is_some_and(|description| description.contains('\0'))
            || self.tags.iter().any(|tag| tag.chars().any(char::is_control))
            || !self.homepage.as_deref().is_none_or(is_public_link)
            || !self.documentation.as_deref().is_none_or(is_public_link)
        {
            return Err(McpFailureCode::InvalidDefinition);
        }

        let transport = match self.connection {
            McpConnectionDefinition::Stdio { .. } => McpTransport::Stdio,
            McpConnectionDefinition::StreamableHttp { .. } => McpTransport::StreamableHttp,
            McpConnectionDefinition::Sse { .. } => McpTransport::Sse,
        };
        let name = name.unwrap_or(&id.0).to_owned();
        let record = McpCatalogRecord {
            record: McpRecordRef { id, revision: 1 },
            scope,
            name,
            description: self.description,
            homepage: self.homepage,
            documentation: self.documentation,
            tags: self.tags,
            transport,
            connection_revision: 1,
        };
        Ok((record, self.connection))
    }
}

fn is_public_link(value: &str) -> bool {
    !value.chars().any(|character| character.is_whitespace() || character.is_control())
        && !value.contains('\\')
        && value.split_once("://").is_some_and(|(_, authority)| !authority.starts_with('/'))
        && Url::parse(value).is_ok_and(|url| {
            matches!(url.scheme(), "http" | "https")
                && url.has_host()
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none()
        })
}

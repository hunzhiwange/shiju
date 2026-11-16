//! Catalog draft inputs shared by host acceptance tests.
use vw_agent_apps::mcp::catalog::McpCatalogDraft;
use vw_api_mcp_management::McpConnectionDefinition;
pub fn http_draft(id: &str, url: &str) -> McpCatalogDraft {
    McpCatalogDraft {
        id: id.into(),
        name: None,
        description: None,
        homepage: None,
        documentation: None,
        tags: Vec::new(),
        connection: McpConnectionDefinition::StreamableHttp {
            url: url.into(),
            headers: Default::default(),
        },
    }
}

//! Shared fixture inputs for host MCP integration tests.
use agent_database::{GlobalDatabase, test_support::PostgresTestDatabase};
use vw_agent_apps::mcp::catalog::McpCatalogDraft;
use vw_api_mcp_management::McpConnectionDefinition;
pub(super) fn with_database(
    test: impl FnOnce(GlobalDatabase) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    if std::env::var_os("VIBEWINDOW_TEST_POSTGRES_URL").is_some() {
        let fixture = PostgresTestDatabase::from_environment()?;
        println!("backend: PostgreSQL");
        test(fixture.database().clone())?;
        fixture.cleanup()
    } else {
        let home = tempfile::tempdir()?;
        println!("backend: SQLite");
        test(GlobalDatabase::for_home(home.path())?)
    }
}

pub(super) fn http_draft(id: &str, url: &str) -> McpCatalogDraft {
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

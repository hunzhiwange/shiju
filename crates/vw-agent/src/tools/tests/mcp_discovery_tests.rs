//! Thin real AgentContextHost transport/configuration bindings.
use crate::tools::context::AgentContextHost;
use tool_mcp::host_migration_35_acceptance as acceptance;

#[tokio::test]
async fn discover_and_call_configured_mcp_tool_through_stdio_fixture() {
    let dir = tempfile::tempdir().expect("temp dir");
    acceptance::discover_and_call_configured_mcp_tool_through_stdio_fixture::<AgentContextHost>(dir.path()).await;
}

#[tokio::test]
async fn mcp_tool_from_spec_default_handler_reports_missing_configured_server() {
    acceptance::mcp_tool_from_spec_default_handler_reports_missing_configured_server::<AgentContextHost>().await;
}

#[tokio::test]
async fn mcp_tool_from_configured_server_uses_server_bound_handler() {
    acceptance::mcp_tool_from_configured_server_uses_server_bound_handler::<AgentContextHost>().await;
}

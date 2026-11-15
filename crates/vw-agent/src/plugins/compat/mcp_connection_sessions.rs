//! Host identity bridge; registry state belongs exclusively to plugin-mcp.
use std::{path::Path, sync::Arc};
use super::mcp_connections::{McpSessionConnections, host};
pub(crate) fn connections(session: &str, root: &Path) -> Arc<McpSessionConnections> {
    plugin_mcp::mcp_connection_sessions::connections(host(), crate::session::ui_store::current_tenant_scope(), session, root)
}
pub(crate) fn close_session(session: &str, root: &Path) {
    plugin_mcp::mcp_connection_sessions::close_session(host().as_ref(), crate::session::ui_store::current_tenant_scope(), session, root);
}
pub(crate) use plugin_mcp::mcp_connection_sessions::close_plugin;

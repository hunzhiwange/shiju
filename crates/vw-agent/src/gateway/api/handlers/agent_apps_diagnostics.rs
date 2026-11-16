//! Compose diagnostic HTTP routes with the existing native MCP host.
use std::path::PathBuf;
use axum::Router;
use crate::agent_apps::mcp::network_probe::NativeProbeHost;

pub(crate) fn router() -> Router {
    gateway_agent_apps::agent_apps_diagnostics::router::<NativeProbeHost>()
}
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    gateway_agent_apps::agent_apps_diagnostics::router_with_policy::<NativeProbeHost>(policy)
}

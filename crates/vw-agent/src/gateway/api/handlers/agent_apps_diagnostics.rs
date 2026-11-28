//! Compose diagnostics for native provider settings.
use std::path::PathBuf;
use axum::Router;

pub(crate) fn router() -> Router {
    gateway_agent_apps::agent_apps_diagnostics::router()
}
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    gateway_agent_apps::agent_apps_diagnostics::router_with_policy(policy)
}

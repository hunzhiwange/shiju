//! Workspace routing composed with the existing host policy.
pub(super) fn router() -> axum::Router {
    gateway_workspace::memory_router(authorize_workspace, super::lifecycle::status)
}

pub(super) fn authorize_workspace(
    policy: Option<&std::path::Path>, headers: &axum::http::HeaderMap,
    project: &vw_api_transport::ProjectScope, target: &str, write: bool,
) -> Result<(gateway_workspace::WorkspaceHost, String), axum::http::StatusCode> {
    super::access::authorize_workspace(policy, headers, project, target, write)
        .map(|(host, actor)| (gateway_workspace::WorkspaceHost { id: host.id, home: host.home }, actor))
}
#[cfg(test)]
#[path = "workspace_memory_tests.rs"]
mod tests;

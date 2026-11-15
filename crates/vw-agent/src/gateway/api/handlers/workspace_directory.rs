//! Workspace routing composed with the existing host policy.
pub(super) fn router() -> axum::Router {
    gateway_workspace::directory_router(super::workspace_memory::authorize_workspace)
}

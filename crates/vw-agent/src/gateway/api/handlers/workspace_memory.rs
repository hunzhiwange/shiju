//! Workspace routing composed with the existing host policy.
pub(super) fn router() -> axum::Router {
    gateway_workspace::memory_router(authorize_workspace, super::lifecycle::status)
}

pub(super) use gateway_workspace::authorize_workspace;

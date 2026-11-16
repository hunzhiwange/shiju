//! Install the application session/process protection host before routing.
pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> axum::Router<S> {
    crate::worktree::initialize_host();
    gateway_worktree::project_router()
}

//! Project route composition and application host callbacks.
pub(super) use gateway_project::{load_project, project_context_directory, project_scope};

fn project_host() -> gateway_project::ProjectHost {
    gateway_project::ProjectHost {
        session_count: |directory| crate::session::ui_store::load_agent_sessions_scoped(Some(directory)).len() as u32,
        instance_bootstrap: |directory| Box::pin(crate::project::instance_bootstrap(directory)),
    }
}

pub(crate) fn router<S>() -> axum::Router<S>
where S: Clone + Send + Sync + 'static,
{
    gateway_project::router(project_host()).merge(super::worktree_management::router())
}

#[cfg(test)]
use gateway_project::testing::*;

#[cfg(test)]
#[path = "../../../../../libs/gateway-project/src/project_tests.rs"]
mod project_tests;

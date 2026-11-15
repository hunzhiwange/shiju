//! Instance route composition and host integration.
use axum::{Router, http::HeaderMap};
use gateway_instance::host::{HostFuture, InstanceHost, InstanceQuery};
use crate::gateway::ApiError;

struct Host;

impl InstanceHost for Host {
    fn resolve_directory(query: &InstanceQuery, headers: &HeaderMap) -> String {
        crate::gateway::instance::resolve_directory(
            &crate::gateway::instance::InstanceQuery { directory: query.directory.clone() },
            headers,
        )
    }

    fn with_instance<T: Send + 'static>(
        directory: String,
        action: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static,
    ) -> HostFuture<Result<T, ApiError>> {
        Box::pin(crate::gateway::instance::with_instance(directory, action))
    }

    fn clear_approval_manager_for_directory(directory: &str) {
        crate::gateway::approval_state::clear_approval_manager_for_directory(directory);
    }
}

pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    gateway_instance::router::<S, Host>()
}

#[cfg(test)]
use gateway_instance::{PathInfo, VcsInfo, branch_for_worktree};
#[cfg(test)]
use gateway_instance::{path_get, vcs_get, instance_event_sse, instance_dispose};
#[cfg(test)]
use crate::{bus, gateway::approval_state};
#[cfg(test)]
#[path = "../../../../../libs/gateway-instance/src/instance_tests.rs"]
mod instance_tests;

//! PTY route composition and project instance binding.
use axum::{Router, http::HeaderMap};
use gateway_pty::host::{HostFuture, InstanceQuery, PtyHost};
use crate::gateway::ApiError;

struct Host;

impl PtyHost for Host {
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
}

pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    gateway_pty::routes::router::<S, Host>()
}

#[cfg(test)]
use gateway_pty::routes::{pty_list, pty_create, pty_get, pty_update, pty_remove};
#[cfg(test)]
use agent_pty as pty;
#[cfg(test)]
use axum::Json;
#[cfg(test)]
#[path = "../../../../../libs/gateway-pty/src/pty_tests.rs"]
mod pty_tests;

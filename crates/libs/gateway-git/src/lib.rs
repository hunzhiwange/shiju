//! Git HTTP routes and repository operations.
#![cfg(not(target_arch = "wasm32"))]
use std::{future::Future, pin::Pin, sync::Arc};
use gateway_types::ApiError;

mod git;
mod git_command;

/// Host binds the project instance and runs its existing bootstrap before listing.
pub trait Host: Send + Sync {
    fn list_directories(&self, directory: String) -> Pin<Box<dyn Future<Output = Result<Vec<String>, ApiError>> + Send + '_>>;
}
#[derive(Clone)]
pub(crate) struct WorktreeHost(pub Arc<dyn Host>);

pub fn router<S: Clone + Send + Sync + 'static>(host: Arc<dyn Host>) -> axum::Router<S> {
    git::router().layer(axum::Extension(WorktreeHost(host)))
}

#[cfg(test)]
mod test_support;

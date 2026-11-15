//! Git route composition with the existing project-instance host adapter.
use std::{future::Future, pin::Pin, sync::Arc};
use crate::gateway::{ApiError, instance::with_instance};

struct GitHost;
impl gateway_git::Host for GitHost {
    fn list_directories(&self, directory: String) -> Pin<Box<dyn Future<Output = Result<Vec<String>, ApiError>> + Send + '_>> {
        Box::pin(with_instance(directory, || Box::pin(async {
            crate::worktree::list_directories().await
                .map_err(|error| ApiError::bad_request(error.to_string()))
        })))
    }
}

pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> axum::Router<S> {
    gateway_git::router(Arc::new(GitHost))
}

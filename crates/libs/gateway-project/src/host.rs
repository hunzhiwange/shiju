//! Host actions retained at the application composition boundary.
use agent_project as project;
use gateway_types::ApiError;
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub struct ProjectHost {
    pub session_count: fn(&str) -> u32,
    pub instance_bootstrap: fn(PathBuf) -> project::BoxFuture<()>,
}

pub(super) async fn with_instance<T>(
    host: ProjectHost,
    directory: String,
    f: impl FnOnce() -> project::BoxFuture<Result<T, ApiError>> + Send + 'static,
) -> Result<T, ApiError> {
    let init: Box<dyn FnOnce() -> project::BoxFuture<()> + Send + 'static> = Box::new(move || {
        Box::pin(async move {
            let wt = project::instance::worktree();
            if !wt.trim().is_empty() {
                (host.instance_bootstrap)(PathBuf::from(wt)).await;
            }
        })
    });
    project::instance::provide(PathBuf::from(directory), Some(init), f)
        .await.map_err(|error| ApiError::bad_request(error.to_string()))?
}

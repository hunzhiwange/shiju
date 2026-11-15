//! Host-owned instance bootstrap and approval cache actions.
use std::{future::Future, pin::Pin};
use axum::http::HeaderMap;
use gateway_types::ApiError;
use serde::Deserialize;

pub type HostFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

#[derive(Debug, Deserialize)]
pub struct InstanceQuery {
    pub directory: Option<String>,
}

pub trait InstanceHost: Send + Sync + 'static {
    fn resolve_directory(query: &InstanceQuery, headers: &HeaderMap) -> String;
    fn with_instance<T: Send + 'static>(
        directory: String,
        action: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static,
    ) -> HostFuture<Result<T, ApiError>>;
    fn clear_approval_manager_for_directory(directory: &str);
}

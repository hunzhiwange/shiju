//! Host-owned tenant/instance binding and shared approval cache access.
use std::{future::Future, pin::Pin, sync::Arc};
use agent_approval::ApprovalManager;
use axum::http::HeaderMap;
use gateway_types::ApiError;
use serde::Deserialize;
use vw_api_transport::TenantScope;

pub type HostFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

#[derive(Debug, Deserialize)]
pub struct InstanceQuery {
    pub directory: Option<String>,
}

pub trait PermissionHost: Send + Sync + 'static {
    fn resolve_directory(query: &InstanceQuery, headers: &HeaderMap) -> String;
    fn with_tenant_scope<T: Send + 'static>(tenant: TenantScope, future: HostFuture<T>) -> HostFuture<T>;
    fn with_instance<T: Send + 'static>(
        directory: String,
        action: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static,
    ) -> HostFuture<Result<T, ApiError>>;
    fn approval_manager_for_current_instance() -> HostFuture<Arc<ApprovalManager>>;
}

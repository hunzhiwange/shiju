//! Actual instance and tenant binding supplied by the host assembly.
use axum::http::HeaderMap;
use gateway_instance::host::{InstanceQuery, HostFuture};
use gateway_types::ApiError;
use vw_api_transport::id::TenantScope;

pub trait MessageHost: Send + Sync + 'static {
    fn current_tenant_scope() -> TenantScope;
    fn resolve_directory(query: &InstanceQuery, headers: &HeaderMap) -> String;
    fn with_tenant_instance<T: Send + 'static>(
        tenant: TenantScope,
        directory: String,
        action: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static,
    ) -> HostFuture<Result<T, ApiError>>;
}

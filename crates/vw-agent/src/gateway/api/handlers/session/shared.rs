//! Instance/tenant assembly and compatibility imports for shared contracts.
use axum::http::HeaderMap;
use vw_api_transport::TenantScope;
use crate::gateway::{ApiError, instance::{InstanceQuery, with_instance}};
use crate::session::ui_store;
#[cfg(test)]
use crate::session as agent_session;
pub(super) use gateway_session::shared::{UiSessionListQuery, UiSessionCreateBody, session_api_error, lifecycle_api_error};
pub(super) fn has_explicit_directory(query: &InstanceQuery, headers: &HeaderMap) -> bool {
    gateway_session::shared::has_explicit_directory(&gateway_instance::host::InstanceQuery { directory: query.directory.clone() }, headers)
}
pub(super) fn resolve_scope_from_query(tenant: &TenantScope, query: &InstanceQuery, headers: &HeaderMap) -> Option<String> {
    gateway_session::shared::resolve_scope_from_query::<super::message_ops::Host>(tenant, &gateway_instance::host::InstanceQuery { directory: query.directory.clone() }, headers)
}
pub(super) async fn with_tenant_instance<T>(
    tenant: TenantScope,
    directory: String,
    f: impl FnOnce() -> crate::app::agent::project::BoxFuture<Result<T, ApiError>> + Send + 'static,
) -> Result<T, ApiError> {
    crate::app::agent::project::from_directory(&tenant, &directory)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    ui_store::with_tenant_scope(tenant, with_instance(directory, f)).await
}

#[cfg(test)]
#[path = "shared_tests.rs"]
mod shared_tests;

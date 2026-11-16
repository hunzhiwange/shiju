//! Domain handler coverage migrated from the agent host. The message port uses
//! real project context and persistence; process bootstrap remains a host boundary.
mod messages;
mod shared;

use axum::http::HeaderMap;
use gateway_instance::host::{HostFuture, InstanceQuery};
use gateway_types::ApiError;
use vw_api_transport::TenantScope;

struct MessageTestHost;

impl crate::message_host::MessageHost for MessageTestHost {
    fn current_tenant_scope() -> TenantScope {
        session_store::ui_store::current_tenant_scope()
    }

    fn resolve_directory(query: &InstanceQuery, headers: &HeaderMap) -> String {
        agent_gateway::instance::resolve_directory(
            &agent_gateway::instance::InstanceQuery { directory: query.directory.clone() },
            headers,
        )
    }

    fn with_tenant_instance<T: Send + 'static>(
        tenant: TenantScope,
        directory: String,
        action: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static,
    ) -> HostFuture<Result<T, ApiError>> {
        Box::pin(async move {
            agent_project::from_directory(&tenant, &directory)
                .await
                .map_err(|error| ApiError::bad_request(error.to_string()))?;
            session_store::ui_store::with_tenant_scope(
                tenant,
                agent_project::instance::provide(directory, None, action),
            )
            .await
            .map_err(|error| ApiError::bad_request(error.to_string()))?
        })
    }
}

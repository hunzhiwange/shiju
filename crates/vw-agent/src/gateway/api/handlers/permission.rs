//! 权限路由的宿主上下文组合；业务实现由 gateway-permission 持有。
use std::sync::Arc;
use axum::{Router, http::HeaderMap};
use gateway_permission::host::{HostFuture, InstanceQuery, PermissionHost};
use crate::gateway::{ApiError, approval_state, instance};

struct Host;

impl PermissionHost for Host {
    fn resolve_directory(query: &InstanceQuery, headers: &HeaderMap) -> String {
        instance::resolve_directory(
            &instance::InstanceQuery { directory: query.directory.clone() }, headers,
        )
    }

    fn with_tenant_scope<T: Send + 'static>(
        tenant: vw_api_transport::TenantScope,
        future: HostFuture<T>,
    ) -> HostFuture<T> {
        Box::pin(crate::session::ui_store::with_tenant_scope(tenant, future))
    }

    fn with_instance<T: Send + 'static>(
        directory: String,
        action: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static,
    ) -> HostFuture<Result<T, ApiError>> {
        Box::pin(instance::with_instance(directory, action))
    }

    fn approval_manager_for_current_instance() -> HostFuture<Arc<crate::approval::ApprovalManager>> {
        Box::pin(approval_state::approval_manager_for_current_instance())
    }
}

pub(crate) fn router<S>() -> Router<S>
where S: Clone + Send + Sync + 'static,
{
    gateway_permission::router::<S, Host>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn router_builds_with_host_ports() { let _: axum::Router<()> = super::router(); }
}

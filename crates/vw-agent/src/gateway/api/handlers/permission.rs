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

// These imports bind the existing host integration tests to the migrated implementation.
#[cfg(test)]
use {
    axum::{Json, extract::{Extension, Path, Query}},
    crate::{approval::{PendingNonCliApprovalRequest, PendingApprovalError, ApprovalResponse}, config,
        config::schema::save_config, permission::next as permission_next},
    instance::with_instance,
    gateway_permission::{PermissionReplyRequest, permission_request_from_pending,
        reply_pending_request, persist_non_cli_approval, map_pending_approval_error},
};

#[cfg(test)]
async fn permission_list(
    tenant: Extension<vw_api_transport::TenantScope>, query: Query<InstanceQuery>, headers: HeaderMap,
) -> Result<Json<Vec<permission_next::Request>>, ApiError> {
    gateway_permission::permission_list::<Host>(tenant, query, headers).await
}

#[cfg(test)]
async fn permission_reply(
    tenant: Extension<vw_api_transport::TenantScope>, query: Query<InstanceQuery>, headers: HeaderMap,
    request_id: Path<String>, body: Json<PermissionReplyRequest>,
) -> Result<Json<bool>, ApiError> {
    gateway_permission::permission_reply::<Host>(tenant, query, headers, request_id, body).await
}

#[cfg(test)]
#[path = "permission_tests.rs"]
mod permission_tests;

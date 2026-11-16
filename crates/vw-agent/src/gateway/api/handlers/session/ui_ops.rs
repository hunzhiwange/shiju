pub(super) struct InstanceHost;
pub(super) type Host = gateway_session::native_ui::NativeUiHost<InstanceHost>;
impl gateway_session::message_host::MessageHost for InstanceHost {
    fn current_tenant_scope() -> vw_api_transport::TenantScope {
        crate::session::ui_store::current_tenant_scope()
    }
    fn resolve_directory(
        query: &gateway_instance::host::InstanceQuery,
        headers: &axum::http::HeaderMap,
    ) -> String {
        crate::gateway::instance::resolve_directory(
            &crate::gateway::instance::InstanceQuery { directory: query.directory.clone() },
            headers,
        )
    }
    fn with_tenant_instance<T: Send + 'static>(
        tenant: vw_api_transport::TenantScope,
        directory: String,
        action: impl FnOnce() -> gateway_instance::host::HostFuture<Result<T, gateway_types::ApiError>>
        + Send
        + 'static,
    ) -> gateway_instance::host::HostFuture<Result<T, gateway_types::ApiError>> {
        Box::pin(super::shared::with_tenant_instance(tenant, directory, action))
    }
}

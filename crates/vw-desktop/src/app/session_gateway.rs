//! 当前桌面端点的租户作用域装配；请求直接调用领域网关函数。

pub(crate) fn active_tenant_scope() -> vw_api_project::TenantScopeDto {
    crate::app::config::gateway_client_endpoint().tenant_scope().clone()
}

#[cfg(test)]
fn gateway_client() -> Result<vw_gateway_client::GatewayClient, String> {
    crate::app::config::gateway_client()
}

#[cfg(test)]
#[path = "session_gateway_tests.rs"]
mod session_gateway_tests;

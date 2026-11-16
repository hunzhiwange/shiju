//! 真实宿主租户中间件和工作树池接线验收。
use super::*;
use vw_api_transport::id::TenantScope;
use agent_project as project;
pub(super) async fn test_default_tenant_scope(
    mut request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if request.extensions().get::<TenantScope>().is_none() {
        request.extensions_mut().insert(project::default_tenant_scope());
    }
    next.run(request).await
}
#[path = "agent_scheduler_tests.rs"]
mod agent_scheduler_tests;

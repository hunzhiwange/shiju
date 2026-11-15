//! Native routes composed with the existing scoped aggregation owner.
use super::*;
use gateway_app_providers::omp::{OmpScope, ScopedOmp};
struct ScopedHost;
impl OmpScope for ScopedHost {
    fn failure(&self, status: StatusCode, code: &'static str) -> Response {
        super::super::aggregation_omp::failure(status, code)
    }
    fn read(&self, policy: Option<std::path::PathBuf>, tenant: TenantScope,
        headers: axum::http::HeaderMap, query: vw_api_app_aggregation::AggregationQuery)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
        Box::pin(super::super::aggregation_omp::read_scoped(policy, tenant, headers, query))
    }
}
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from))
}
pub(in super::super) fn router_with_policy(path: Option<std::path::PathBuf>) -> Router {
    gateway_app_providers::omp::router_with_policy(path.clone())
        .merge(super::super::aggregation_omp::router_with_policy(path))
        .layer(Extension(ScopedOmp(std::sync::Arc::new(ScopedHost))))
}

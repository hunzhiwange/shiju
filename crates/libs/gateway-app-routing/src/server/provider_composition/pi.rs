//! Compose native Pi routes with the existing scoped aggregation owner.
use axum::{Extension, Router, response::Response};
use vw_api_transport::TenantScope;

use gateway_app_providers::pi::{PiScope, ScopedPi};
struct ScopedHost;
impl PiScope for ScopedHost {
    fn invalid_request(&self) -> Response {
        gateway_aggregation::aggregation_pi::invalid_request()
    }
    fn read(&self, policy: Option<std::path::PathBuf>, tenant: TenantScope,
        headers: axum::http::HeaderMap, query: vw_api_app_aggregation::AggregationQuery)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
        Box::pin(gateway_aggregation::aggregation_pi::read(policy, tenant, headers, query))
    }
}
pub fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from))
}
pub fn router_with_policy(path: Option<std::path::PathBuf>) -> Router {
    gateway_app_providers::pi::router_with_policy(path.clone())
        .merge(gateway_aggregation::aggregation_pi::router_with_policy(path))
        .layer(Extension(ScopedPi(std::sync::Arc::new(ScopedHost))))
}

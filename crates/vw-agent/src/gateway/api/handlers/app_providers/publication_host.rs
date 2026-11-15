//! Strongly typed adapter to the existing gateway publication transaction.
#![cfg(target_os = "macos")]
use super::*;
use super::super::app_routing::RoutingControl;
use gateway_app_providers::{ProviderPublication, PublicationControl};
struct PublicationHost(Option<RoutingControl>);
impl ProviderPublication for PublicationHost {
    fn headers(&self, headers: axum::http::HeaderMap) -> Result<axum::http::HeaderMap, StatusCode> {
        super::super::aggregation_resync::provider_headers(headers)
    }
    fn configure(&self, store: AppProviderStore, database: &GlobalDatabase,
        policy: Option<&std::path::Path>, headers: &axum::http::HeaderMap,
        tenant: &TenantScope, host: &str, app: AppId,
        query: vw_api_app_aggregation::AggregationWriteQuery) -> Result<AppProviderStore, StatusCode> {
        super::super::aggregation_resync::configure(store, database, policy, headers,
            tenant, host, app, query, self.0.clone())
    }
}
pub(super) fn control(routing: Option<RoutingControl>) -> PublicationControl {
    PublicationControl(std::sync::Arc::new(PublicationHost(routing)))
}
pub(super) fn input(input: PublicationInput) -> gateway_app_providers::PublicationInput {
    gateway_app_providers::PublicationInput { query: input.query, routing: Some(control(input.routing)) }
}
pub(super) async fn inject(routing: Option<Extension<RoutingControl>>,
    mut request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    request.extensions_mut().insert(control(routing.map(|Extension(c)| c)));
    next.run(request).await
}

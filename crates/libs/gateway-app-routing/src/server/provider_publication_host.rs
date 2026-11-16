//! Concrete publication adapter using the native routing grant.
use axum::http::StatusCode;
use app_provider_services::AppProviderStore;
use agent_database::GlobalDatabase;
use vw_api_transport::TenantScope;
use vw_api_app_identity::AppId;
use gateway_app_providers::{ProviderPublication, PublicationControl};
use super::lifecycle_api::RoutingControl;
struct PublicationHost(Option<RoutingControl>);
impl ProviderPublication for PublicationHost {
    fn headers(&self, headers: axum::http::HeaderMap) -> Result<axum::http::HeaderMap, StatusCode> {
        super::record_resync::provider_headers(headers)
    }
    fn configure(&self, store: AppProviderStore, database: &GlobalDatabase,
        policy: Option<&std::path::Path>, headers: &axum::http::HeaderMap,
        tenant: &TenantScope, host: &str, app: AppId,
        query: vw_api_app_aggregation::AggregationWriteQuery) -> Result<AppProviderStore, StatusCode> {
        super::record_resync::configure(store, database, policy, headers,
            tenant, host, app, query, self.0.clone())
    }
}
pub fn control(routing: Option<RoutingControl>) -> PublicationControl {
    PublicationControl(std::sync::Arc::new(PublicationHost(routing)))
}

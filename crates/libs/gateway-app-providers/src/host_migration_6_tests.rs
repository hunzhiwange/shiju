//! Provider domain scenarios registered by their owning crate.
#[cfg(target_os = "macos")]
use crate::*;

#[cfg(target_os = "macos")]
async fn publication(mut request: axum::extract::Request, next: axum::middleware::Next) -> axum::response::Response {
    request.extensions_mut().insert(publication_control());
    next.run(request).await
}

#[cfg(target_os = "macos")]
mod claude_tests;
#[cfg(target_os = "macos")]
mod codex_catalog_tests;
#[cfg(target_os = "macos")]
mod claude_desktop_tests;
#[cfg(target_os = "macos")]
mod agy_tests;
#[cfg(target_os = "macos")]
mod catalog_tests;
mod basic_tests;
mod create_tests;

fn record_router() -> axum::Router {
    crate::router()
        .merge(crate::basic::router())
        .merge(crate::duplicate::router())
}
#[cfg(target_os = "macos")]
mod codex_tests;
#[cfg(target_os = "macos")]
mod delete_tests;

#[cfg(target_os = "macos")]
mod api_security_tests;

#[cfg(target_os = "macos")]
fn publication_control() -> crate::PublicationControl {
 use axum::http::{HeaderMap, StatusCode};
 use app_provider_services::AppProviderStore;
 use agent_database::GlobalDatabase;
 use vw_api_transport::TenantScope;
 use vw_api_app_identity::AppId;
 use vw_api_app_aggregation::AggregationWriteQuery;
 struct Adapter<F, G>(F, G);
 impl<F, G> crate::ProviderPublication for Adapter<F, G>
 where F: Fn(HeaderMap) -> Result<HeaderMap, StatusCode> + Send + Sync,
 G: Fn(AppProviderStore, &GlobalDatabase, Option<&std::path::Path>, &HeaderMap, &TenantScope, &str, AppId, AggregationWriteQuery) -> Result<AppProviderStore, StatusCode> + Send + Sync {
  fn headers(&self, headers: HeaderMap) -> Result<HeaderMap, StatusCode> { (self.0)(headers) }
  fn configure(&self, store: AppProviderStore, database: &GlobalDatabase, policy: Option<&std::path::Path>, headers: &HeaderMap, tenant: &TenantScope, host: &str, app: AppId, query: AggregationWriteQuery) -> Result<AppProviderStore, StatusCode> { (self.1)(store, database, policy, headers, tenant, host, app, query) }
 }
 let a = gateway_app_routing::server::provider_publication_host::control(None);
 let c = a.clone();
 crate::PublicationControl(std::sync::Arc::new(Adapter(
 move |headers| a.0.headers(headers),
 move |store, database: &GlobalDatabase, policy: Option<&std::path::Path>, headers: &HeaderMap, tenant: &TenantScope, host: &str, app, query| c.0.configure(store, database, policy, headers, tenant, host, app, query),
 )))
}

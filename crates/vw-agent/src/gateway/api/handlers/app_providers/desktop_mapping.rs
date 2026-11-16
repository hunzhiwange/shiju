#[cfg(target_os = "macos")]
pub(crate) use gateway_app_routing::server::provider_composition::desktop_mapping::*;
#[cfg(not(target_os = "macos"))]
mod portable {
//! Host composition for scoped Desktop mapping publication.
use super::super::*;
use gateway_app_providers::desktop_mapping::{DesktopMappingControl, DesktopMappingHost};
struct Host {
    #[cfg(target_os = "macos")]
    routing: Option<super::super::super::app_routing::RoutingControl>,
}
impl DesktopMappingHost for Host {
    fn read_form(&self, policy: Option<&std::path::Path>, tenant: TenantScope,
        db: Option<Extension<GlobalDatabase>>, headers: axum::http::HeaderMap, id: String,
        query: vw_api_app_aggregation::AggregationQuery) -> Result<Response, StatusCode> {
        super::super::super::aggregation_desktop::read_form(policy, tenant, db, headers, id, query)
    }
    #[cfg(target_os = "macos")]
    fn save_mapping(&self, policy: Option<&std::path::Path>, tenant: TenantScope,
        db: Option<Extension<GlobalDatabase>>, headers: axum::http::HeaderMap, id: String,
        request: vw_api_app_aggregation::DesktopMappingChange) -> Result<Response, StatusCode> {
        super::super::super::aggregation_desktop::save_mapping(policy, tenant, db, headers, id, request,
            self.routing.clone().map(Extension))
    }
}
async fn inject(
    #[cfg(target_os = "macos")] routing: Option<Extension<super::super::super::app_routing::RoutingControl>>,
    mut request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    request.extensions_mut().insert(DesktopMappingControl(std::sync::Arc::new(Host {
        #[cfg(target_os = "macos")]
        routing: routing.map(|Extension(c)| c),
    })));
    next.run(request).await
}
pub(crate) fn router() -> Router {
    compose(gateway_app_providers::desktop_mapping::router())
}
pub(crate) fn router_with_policy(path: Option<std::path::PathBuf>) -> Router {
    compose(gateway_app_providers::desktop_mapping::router_with_policy(path))
}
fn compose(router: Router) -> Router {
    router.layer(axum::middleware::from_fn(inject))
}
}
#[cfg(not(target_os = "macos"))]
pub(crate) use portable::*;

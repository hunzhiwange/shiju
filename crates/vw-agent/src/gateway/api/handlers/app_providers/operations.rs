#[cfg(target_os = "macos")]
pub(crate) use gateway_app_routing::server::provider_composition::operations::*;
#[cfg(not(target_os = "macos"))]
mod portable {
//! Compose the unique provider registry with aggregation routes.
use super::super::*;
pub(crate) use gateway_app_providers::operations::{State, aggregation};
struct ScopedHost;
impl gateway_app_providers::operations::OperationHost for ScopedHost {
    #[cfg(target_os = "macos")]
    fn headers(&self, headers: axum::http::HeaderMap) -> Result<axum::http::HeaderMap, StatusCode> {
        super::super::super::aggregation_resync::provider_headers(headers)
    }
    fn error(&self, status: StatusCode) -> Response {
        super::super::super::aggregation_publish::operation_error(status)
    }
}
#[cfg(test)]
pub(crate) fn router_with_policy(policy: Option<std::path::PathBuf>) -> Router {
    router_with_editor(policy.clone(), editor_merge::Editor::new(policy))
}
pub(crate) fn router_with_editor(policy: Option<std::path::PathBuf>, editor: editor_merge::Editor) -> Router {
    let routes = Router::new().merge(super::super::super::aggregation_publish::router())
        .merge(super::super::super::aggregation_exit::router()).merge(super::super::super::aggregation_restart::router());
    gateway_app_providers::operations::router_with_editor(policy, editor, routes)
        .layer(Extension(gateway_app_providers::operations::OperationControl(std::sync::Arc::new(ScopedHost))))
}
}
#[cfg(not(target_os = "macos"))]
pub(crate) use portable::*;

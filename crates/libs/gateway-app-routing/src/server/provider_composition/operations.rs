//! Compose the unique provider registry with aggregation routes.
use axum::{Extension, Router, http::StatusCode, response::Response};

pub use gateway_app_providers::operations::{State, aggregation};
struct ScopedHost;
impl gateway_app_providers::operations::OperationHost for ScopedHost {
    #[cfg(target_os = "macos")]
    fn headers(&self, headers: axum::http::HeaderMap) -> Result<axum::http::HeaderMap, StatusCode> {
        gateway_aggregation::aggregation_resync::provider_headers(headers)
    }
    fn error(&self, status: StatusCode) -> Response {
        gateway_aggregation::aggregation_publish::operation_error(status)
    }
}
pub fn router_with_policy(policy: Option<std::path::PathBuf>) -> Router {
    router_with_editor(policy.clone(), gateway_app_providers::editor_merge::Editor::new(policy))
}
pub fn router_with_editor(policy: Option<std::path::PathBuf>, editor: gateway_app_providers::editor_merge::Editor) -> Router {
    use crate::server::{lifecycle_api::local_application_control, snapshot_host::SnapshotHost};

    let routes = Router::new()
        .merge(gateway_aggregation::aggregation_publish::router(SnapshotHost))
        .merge(gateway_aggregation::aggregation_exit::router(SnapshotHost))
        .route_layer(axum::middleware::from_fn(local_application_control))
        .merge(gateway_aggregation::aggregation_restart::router(SnapshotHost));
    gateway_app_providers::operations::router_with_editor(policy, editor, routes)
        .layer(Extension(gateway_app_providers::operations::OperationControl(std::sync::Arc::new(ScopedHost))))
}

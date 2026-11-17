//! Host composition for Claude Desktop aggregation.
use axum::{Extension, Router, http::{HeaderMap, StatusCode}, response::Response};
use std::path::{Path, PathBuf};
use crate::global_database::GlobalDatabase;
use vw_api_transport::TenantScope;
use vw_api_app_aggregation::{AggregationQuery, DesktopMappingChange};
use super::aggregation_codex_settings::SnapshotHost;
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use gateway_aggregation::aggregation_desktop::{finish, failure};
pub(super) fn router() -> Router {
    let router =
        router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from));
    match GlobalDatabase::for_home(&crate::global::paths().home) {
        Ok(database) => router.layer(Extension(database)),
        Err(_) => router,
    }
}
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    gateway_aggregation::aggregation_desktop::router_with_policy(policy, SnapshotHost)
}
pub(super) fn read_form(policy: Option<&Path>, tenant: TenantScope, db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, id: String, query: AggregationQuery) -> Result<Response, StatusCode> {
    gateway_aggregation::aggregation_desktop::read_form(SnapshotHost, policy, tenant, db, headers, id, query)
}
#[cfg(target_os = "macos")]
pub(super) fn save_mapping(policy: Option<&Path>, tenant: TenantScope, db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, id: String, request: DesktopMappingChange,
    control: Option<Extension<super::app_routing::RoutingControl>>) -> Result<Response, StatusCode> {
    gateway_aggregation::aggregation_desktop::save_mapping(SnapshotHost, policy, tenant, db, headers, id, request, control)
}

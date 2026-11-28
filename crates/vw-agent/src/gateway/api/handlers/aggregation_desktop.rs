//! Host composition for project-scoped Claude Desktop mappings.
use axum::{Extension, http::{HeaderMap, StatusCode}, response::Response};
use std::path::Path;
use crate::global_database::GlobalDatabase;
use vw_api_transport::TenantScope;
use vw_api_app_aggregation::{AggregationQuery, DesktopMappingChange};
#[cfg(target_os = "macos")]
use gateway_app_routing::server::snapshot_host::SnapshotHost;
#[cfg(not(target_os = "macos"))]
#[derive(Clone)]
struct SnapshotHost;
#[cfg(not(target_os = "macos"))]
impl gateway_aggregation::publication_host::SnapshotHost for SnapshotHost {}
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use gateway_aggregation::aggregation_desktop::{finish, failure};
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

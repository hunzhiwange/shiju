//! Host operations used by routing observation endpoints.
use super::ControlError;
use axum::{Extension, http::HeaderMap};
use std::{future::Future, net::IpAddr, path::PathBuf};
use vw_api_app_identity::AppId;
use vw_api_app_routing::RoutingRequestsStatus;
use vw_api_transport::id::{ProjectId, ProjectScope, TenantScope};

pub trait RoutingObservation: Clone + Send + Sync + 'static {
    fn routing_status(&self) -> vw_api_app_routing::RoutingStatus;
    fn ready(&self) -> bool;

    fn authorize(
        control: Option<Extension<Self>>,
        headers: HeaderMap,
        tenant: TenantScope,
        host: String,
        app: Option<(ProjectId, AppId)>,
        manage: bool,
    ) -> impl Future<Output = Result<(Self, Option<PathBuf>), ControlError>> + Send;

    fn config_bind(&self) -> impl Future<Output = Result<IpAddr, ControlError>> + Send;

    fn request_status(&self, project: &ProjectScope, host: &str, app: AppId)
        -> RoutingRequestsStatus;
}

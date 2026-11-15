//! Host paths and aggregation lifecycle assembly.
pub(crate) use gateway_app_routing::server::lifecycle_api::*;
pub(crate) use gateway_app_routing::server::ControlError;
use crate::{app_routing::RoutingListener, global_database::GlobalDatabase};
use axum::Router;
use std::{path::PathBuf, sync::Arc};
use vw_api_transport::id::TenantScope;
fn paths() -> RoutingPaths {
    let paths = crate::global::paths();
    RoutingPaths { home: paths.home.clone(), config: paths.config.clone(), state: paths.state.clone() }
}
fn lifecycle(database: GlobalDatabase, policy: PathBuf, tenant: TenantScope, host: String)
    -> crate::gateway::aggregation_lifecycle::AggregationLifecycle {
    crate::gateway::aggregation_lifecycle::AggregationLifecycle { database, policy, tenant, host }
}
pub(crate) async fn configure(app: Router, routing: &Arc<RoutingListener>) -> Router {
    gateway_app_routing::server::lifecycle_api::configure(app, routing, paths(), lifecycle).await
}
#[cfg(test)]
pub(crate) async fn configure_with_policy(app: Router, routing: &Arc<RoutingListener>, policy: PathBuf, database: GlobalDatabase) -> Router {
    gateway_app_routing::server::lifecycle_api::configure_with_policy(app, routing, policy, database, paths(), lifecycle).await
}

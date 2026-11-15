//! Additional authorization for existing scoped supplier writes.
use crate::{
    app_providers::AppProviderStore, gateway::aggregation_resync::RecordPublicationContext,
    global_database::GlobalDatabase,
};
use axum::http::{HeaderMap, StatusCode};
use std::path::Path;
use vw_api_app_identity::AppId;
use vw_api_app_aggregation::AggregationWriteQuery;
use vw_api_transport::id::TenantScope;

#[allow(clippy::too_many_arguments)]
pub(crate) fn configure(
    store: AppProviderStore,
    db: &GlobalDatabase,
    policy: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    host: &str,
    app: AppId,
    query: AggregationWriteQuery,
    routing: Option<super::app_routing::RoutingControl>,
) -> Result<AppProviderStore, StatusCode> {
    gateway_aggregation::aggregation_resync::configure(super::aggregation_codex_settings::SnapshotHost, store, db, policy, headers, tenant, host, app, query, routing)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn configure_with_context(
    store: AppProviderStore,
    db: &GlobalDatabase,
    policy: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    host: &str,
    app: AppId,
    query: AggregationWriteQuery,
    routing: Option<super::app_routing::RoutingControl>,
) -> Result<(AppProviderStore, Option<RecordPublicationContext>), StatusCode> {
    gateway_aggregation::aggregation_resync::configure_with_context(super::aggregation_codex_settings::SnapshotHost, store, db, policy, headers, tenant, host, app, query, routing)
}
pub(crate) use gateway_aggregation::aggregation_resync::provider_headers;

use super::aggregation_codex_settings::SnapshotHost;
impl gateway_aggregation::resync_host::ResyncHost for SnapshotHost {
    type Context = RecordPublicationContext;
    fn editor_context(&self, policy: Option<&Path>, headers: &HeaderMap, tenant: &TenantScope,
        host: &str, write: bool) -> Result<(std::path::PathBuf, std::path::PathBuf), StatusCode> {
        super::agent_apps::provider_editor_context(policy, headers, tenant, host, write)
    }
    fn attach(&self, store: AppProviderStore, p: gateway_aggregation::publication_host::Publication,
        routing: Option<Self::Routing>) -> (AppProviderStore, Self::Context) {
        let context = RecordPublicationContext {
            database: p.database, project: p.project, host: p.host, app: p.app, home: p.home,
            root: p.root, routing: routing.map(|control| control.routing), expected: p.expected,
            principal: p.principal, revision: p.revision, failure: Default::default(),
        };
        (store.with_resync(context.clone()), context)
    }
}

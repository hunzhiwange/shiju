//! Host binding for aggregation startup and shutdown service.
use agent_database::GlobalDatabase;
use app_routing_services::RoutingListener;
use vw_api_transport::id::TenantScope;
use std::{path::PathBuf, sync::Arc};
use axum::http::StatusCode;
#[derive(Clone)]
pub(crate) struct AggregationLifecycle {
    pub(crate) database: GlobalDatabase,
    pub(crate) policy: PathBuf,
    pub(crate) tenant: TenantScope,
    pub(crate) host: String,
}

pub(crate) use app_routing_services::lifecycle_host::DetachResult;
impl AggregationLifecycle {
    fn service(&self) -> agent_gateway::aggregation_lifecycle::AggregationLifecycle {
        agent_gateway::aggregation_lifecycle::AggregationLifecycle { database: self.database.clone(),
            policy: self.policy.clone(), tenant: self.tenant.clone(), host: self.host.clone() }
    }
    pub(crate) fn resume(&self, routing: Arc<RoutingListener>) -> Result<(), StatusCode> {
        self.service().resume(&super::aggregation_desktop_record::Host, routing)
    }
    pub(crate) fn detach(&self) -> DetachResult {
        self.service().detach(&super::aggregation_desktop_record::Host)
    }
}
impl app_routing_services::lifecycle_host::AggregationLifecycleHost for AggregationLifecycle {
    fn resume(&self, routing: Arc<RoutingListener>) -> Result<(), StatusCode> { self.resume(routing) }
    fn detach(&self) -> DetachResult { self.detach() }
}

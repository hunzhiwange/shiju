//! Host boundary for the existing aggregation recovery and detach operations.
use crate::RoutingListener;
use axum::http::StatusCode;
use std::sync::Arc;
use vw_api_app_failover::FailoverQueueScope;

#[derive(Default)]
pub struct DetachResult {
    pub retired: Vec<FailoverQueueScope>,
    pub failure: Option<StatusCode>,
}

/// Both operations retain their original synchronous semantics.
pub trait AggregationLifecycleHost: Send + Sync {
    fn resume(&self, routing: Arc<RoutingListener>) -> Result<(), StatusCode>;
    fn detach(&self) -> DetachResult;
}

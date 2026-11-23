//! Host boundary for saved route recovery and aggregation detach.
use crate::{RouteRestore, RoutingListener};
use axum::http::StatusCode;
use std::{path::PathBuf, sync::Arc};
use vw_agent_apps::providers::live::commit::AppState;
use vw_api_app_failover::FailoverQueueScope;

#[derive(Default)]
pub struct DetachResult {
    pub retired: Vec<FailoverQueueScope>,
    pub failure: Option<StatusCode>,
}

pub struct RestoreApp {
    pub scope: FailoverQueueScope,
    pub home: PathBuf,
    pub root: PathBuf,
    pub principal: String,
    pub expected: AppState,
}

/// Both operations retain their original synchronous semantics.
pub trait AggregationLifecycleHost: Send + Sync {
    fn resume(&self, routing: Arc<RoutingListener>) -> Result<(), StatusCode>;
    fn detach(&self) -> DetachResult;
    fn restore(
        &self,
        _routes: &RouteRestore,
        _app: RestoreApp,
        _authorized: &dyn Fn() -> bool,
    ) -> Result<(), StatusCode> {
        Err(StatusCode::NOT_IMPLEMENTED)
    }
}

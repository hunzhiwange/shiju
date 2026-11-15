//! Completed query snapshots shared with presentation.
use std::sync::Arc;
use vw_api_apps_usage::apps_usage::UsageQueryResult;

#[derive(Debug, PartialEq)]
pub enum UsageCacheResult {
    Current(UsageQueryResult),
    /// Display the original data and success time alongside the current failure.
    Stale {
        last_success: Arc<UsageQueryResult>,
        failure: UsageQueryResult,
    },
}

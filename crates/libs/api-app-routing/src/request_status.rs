//! Bounded, content-free request lifecycle snapshots. Terminal rows are events
//! identified by request_id; clients deduplicate repeated polling by that ID.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingRequestState {
    Active,
    Completed,
    Failed,
    Cancelled,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutingRequestStatus {
    pub request_id: String,
    pub state: RoutingRequestState,
    /// Elapsed time to first delivered SSE output event; absent for JSON.
    pub first_token_ms: Option<u64>,
    pub elapsed_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutingRequestsStatus {
    pub active_count: usize,
    /// At most 1024 active rows plus the latest 256 terminal rows globally.
    /// History is volatile and may be evicted between polls.
    pub requests: Vec<RoutingRequestStatus>,
}

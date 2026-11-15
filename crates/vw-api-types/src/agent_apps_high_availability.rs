//! Compatibility exports for application failover contracts.

pub use vw_api_app_failover::{
    FailoverQueueQuery,
    FailoverQueueScope,
    FailoverQueueVersion,
    FailoverQueueEntry,
    FailoverQueueResponse,
    FailoverCandidatesResponse,
    FailoverQueueMemberRequest,
    FailoverQueueReorderRequest,
    FailoverQueueEnabledRequest,
    FailoverPolicy,
    CircuitBreakerPolicy,
    RetryTimeoutPolicy,
    FailoverHealthState,
    FailoverHealthResponse,
    FailoverFailureReason,
    FailoverRouteResponse,
};

//! Codex process hints are owned by gateway-aggregation.
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(super) use gateway_aggregation::aggregation_clients::router;
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use gateway_aggregation::aggregation_clients::{router, router_with_policy};

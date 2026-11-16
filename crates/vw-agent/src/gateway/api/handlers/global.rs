//! 全局路由组合；实现由 gateway-global 持有。

pub(crate) use gateway_global::router;

pub(crate) fn stateful_router() -> axum::Router<crate::gateway::AppState> {
    gateway_global::stateful_router()
}

#[cfg(test)]
use crate::config;
#[cfg(test)]
use crate::gateway::AppState;
#[cfg(test)]
use axum::Router;
#[cfg(test)]
use serde_json::Value;

#[cfg(test)]
#[path = "global_tests.rs"]
mod global_tests;

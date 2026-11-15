//! 全局路由组合；实现由 gateway-global 持有。

pub(crate) use gateway_global::router;

pub(crate) fn stateful_router() -> axum::Router<crate::gateway::AppState> {
    gateway_global::stateful_router()
}

#[cfg(test)]
use gateway_global::*;
#[cfg(test)]
use crate::{config, bus};
#[cfg(test)]
use crate::gateway::AppState;
#[cfg(test)]
use axum::{Json, Router};
#[cfg(test)]
use serde_json::Value;
#[cfg(test)]
use std::time::Duration;
#[cfg(test)]
use std::collections::HashMap;

#[cfg(test)]
#[path = "global_tests.rs"]
mod global_tests;

//! Domain policy coverage; extension types are unused by these handlers.
use agent_config::runtime::Config;
use gateway_types::{GatewayRateLimiter, IdempotencyStore};
use gateway_pairing::pairing::PairingGuard;
use model_types::Provider;
use memory_types::memory::NoneMemory;
use parking_lot::Mutex;
use std::{sync::Arc, time::Duration};
use crate::request_policy::*;
use crate::util::*;
use axum::http::StatusCode;
use crate::{health, node_control::NodeControlRequest};
type AppState = gateway_types::AppState<(), (), ()>;
mod policy;
mod acceptance;
mod node_service;
mod router;

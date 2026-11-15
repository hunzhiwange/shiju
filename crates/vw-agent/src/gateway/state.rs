//! Host bindings for the gateway state contract.
use std::{collections::HashMap, sync::Arc};
use parking_lot::Mutex;
use vw_api_transport::id::TenantScope;
use crate::agent::loop_::query_engine::QueryEngine;

pub(crate) type SharedQueryEngine = Arc<tokio::sync::Mutex<QueryEngine>>;
pub(crate) type SessionQueryEngineStore =
    Arc<tokio::sync::Mutex<HashMap<(TenantScope, String), SharedQueryEngine>>>;
pub(crate) type GatewayChannels = gateway_types::state::GatewayChannels<
    Arc<Mutex<HashMap<TenantScope, Arc<crate::channels::ChannelRuntimeContext>>>>,
    Arc<crate::channels::WhatsAppChannel>,
>;
pub type AppState = gateway_types::state::AppState<
    SessionQueryEngineStore,
    GatewayChannels,
    crate::hooks::HookRuntime,
>;

// This assertion covers the concrete host bindings, so remains with their owner.
#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;

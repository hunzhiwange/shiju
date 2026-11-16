//! Gateway application services.
#[cfg(not(target_arch = "wasm32"))]
pub mod request_policy;
#[cfg(not(target_arch = "wasm32"))]
pub mod util;
#[cfg(not(target_arch = "wasm32"))]
pub mod webhook;
#[cfg(not(target_arch = "wasm32"))]
pub mod webhook_ingress;
#[cfg(not(target_arch = "wasm32"))]
pub mod ws;
#[cfg(all(not(target_arch = "wasm32"), any(test, feature = "test-support")))]
pub mod pairing;
#[cfg(not(target_arch = "wasm32"))]
pub mod client_address;
#[cfg(not(target_arch = "wasm32"))]
pub mod router;
#[cfg(not(target_arch = "wasm32"))]
pub mod runtime;
#[cfg(not(target_arch = "wasm32"))]
pub mod sse;
#[cfg(not(target_arch = "wasm32"))]
pub mod health;
#[cfg(not(target_arch = "wasm32"))]
pub mod node_control;
#[cfg(not(target_arch = "wasm32"))]
pub mod openai_compat;
#[cfg(not(target_arch = "wasm32"))]
pub mod middleware;

#[cfg(not(target_arch = "wasm32"))]
pub mod handlers;
#[cfg(not(target_arch = "wasm32"))]
pub mod instance;

#[cfg(not(target_arch = "wasm32"))]
pub mod chat;
#[cfg(not(target_arch = "wasm32"))]
pub mod chat_host;
#[cfg(not(target_arch = "wasm32"))]
pub mod bootstrap;
#[cfg(not(target_arch = "wasm32"))]
pub mod route_catalog;
#[cfg(not(target_arch = "wasm32"))]
pub mod channel_reload;
#[cfg(not(target_arch = "wasm32"))]
pub mod channel_runtime;
#[cfg(not(target_arch = "wasm32"))]
pub mod api;
#[cfg(not(target_arch = "wasm32"))]
pub mod approval_state;
#[cfg(target_os = "macos")]
pub mod aggregation_resync;
pub mod agent_apps;
pub mod agent_apps_diagnostic_jobs;
#[cfg(not(target_arch = "wasm32"))]
pub mod agent;
pub mod agent_apps_diagnostic_executors;
// Diagnostic fixtures live with app-diagnostics-runtime.
#[cfg(target_os = "macos")]
pub mod agent_apps_high_availability_service;
#[cfg(target_os = "macos")]
pub mod publication_context;
#[cfg(target_os = "macos")]
pub mod aggregation_desktop_record;
#[cfg(target_os = "macos")]
pub mod aggregation_lifecycle;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_15_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_18_tests;

//! Gateway application services.
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
#[cfg(test)]
mod diagnostic_stream_fixtures;
#[cfg(target_os = "macos")]
pub mod agent_apps_high_availability_service;
#[cfg(target_os = "macos")]
pub mod publication_context;
#[cfg(target_os = "macos")]
pub mod aggregation_desktop_record;
#[cfg(target_os = "macos")]
pub mod aggregation_lifecycle;

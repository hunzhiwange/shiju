//! Tenant scoped model catalog HTTP routes and discovery tasks.
#![cfg(not(target_arch = "wasm32"))]
use axum::http::{HeaderMap, StatusCode};
use std::path::{Path, PathBuf};
use vw_api_transport::id::TenantScope;

/// Existing host policy operations, injected without exposing policy records.
#[derive(Clone, Copy)]
pub struct Authorization {
    pub provider_config_home: fn(Option<&Path>, &HeaderMap, &TenantScope, &str) -> Result<PathBuf, StatusCode>,
    pub authorize_provider_config_access: fn(Option<&Path>, &HeaderMap, &TenantScope, &str, bool) -> Result<(), StatusCode>,
    #[cfg(target_os = "macos")]
    pub provider_discovery_host: fn(Option<&Path>, &HeaderMap, &TenantScope, &str) -> Result<(vw_agent_apps::apps::discovery::read::LocalReadEnvironment, String), StatusCode>,
}

mod model_catalog;
mod model_catalog_edits;
#[cfg(target_os = "macos")]
mod model_discovery;
pub use model_catalog::router_with_policy;

#[cfg(all(test, target_os = "macos"))]
mod host_migration_16_tests;

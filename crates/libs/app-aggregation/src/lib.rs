//! Shared native provider configuration and failover guards.

pub use app_aggregation_catalog::{model_ids, omo_read};

pub const CATALOG_FILENAME: &str = "shiju-model-catalog.json";

#[cfg(target_os = "macos")]
pub mod failover_guard;
#[cfg(all(test, target_os = "macos"))]
mod failover_guard_tests;

#[cfg(target_os = "macos")]
pub mod omo_scope;

#[cfg(target_os = "macos")]
pub mod omo_write;
#[cfg(all(test, target_os = "macos"))]
mod omo_write_tests;

#[cfg(target_os = "macos")]
pub mod omo_slim;
#[cfg(all(test, target_os = "macos"))]
mod omo_slim_tests;

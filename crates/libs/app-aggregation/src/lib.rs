//! Host-local aggregation membership, settings and OMO transactions.

pub use app_aggregation_catalog::{codex_classic, codex_review, codex_subagent, model_ids, omo_read};

pub const CATALOG_FILENAME: &str = "shiju-model-catalog.json";

pub mod member_keys;
#[cfg(all(test, target_os = "macos"))]
mod member_keys_tests;

pub mod member_validation;
#[cfg(all(test, target_os = "macos"))]
mod member_validation_tests;

#[cfg(target_os = "macos")]
pub mod membership;
#[cfg(all(test, target_os = "macos"))]
mod membership_tests;

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

pub mod codex_settings;

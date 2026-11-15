//! Application gateway composition exports. Shared authorization owns no handlers.
#![cfg(not(target_arch = "wasm32"))]
pub use app_access_service::{access, UsageSigningCredentialRefs, MAX_PROBES_PER_JOB};

pub mod read;
pub mod preferences;
pub mod launch;
mod launch_identities;
mod launch_sessions;

pub mod lifecycle;
pub mod usage_read;
#[cfg(target_os = "macos")]
pub mod usage_refresh;
#[cfg(target_os = "macos")]
pub mod usage_codex;
#[cfg(target_os = "macos")]
pub mod usage_native;
#[cfg(target_os = "macos")]
pub mod apps_usage_refresh;
#[cfg(target_os = "macos")]
pub mod apps_usage_script;

#[cfg(all(test, target_os = "macos"))]
mod usage_refresh_tests;

#[cfg(all(test, target_os = "macos"))]
mod usage_read_tests;

#[cfg(all(test, target_os = "macos"))]
mod apps_usage_refresh_tests;

#[cfg(all(test, target_os = "macos"))]
mod apps_usage_script_tests;

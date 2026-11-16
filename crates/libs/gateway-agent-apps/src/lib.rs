//! Application gateway composition exports. Shared authorization owns no handlers.
#![cfg(not(target_arch = "wasm32"))]
pub use app_access_service::{access, UsageSigningCredentialRefs, MAX_PROBES_PER_JOB};

pub mod agent_apps_diagnostics;
pub mod agent_apps_data;
pub mod agent_apps_error;
mod data_access;
#[cfg(test)]
mod agent_apps_error_tests;

pub mod agent_apps_skills;
pub mod icons;
pub mod host_context;
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

mod auth_access;
mod agent_apps_auth_binding;
mod agent_apps_auth_cancel;
mod agent_apps_auth_default;
mod agent_apps_auth_logout;
mod agent_apps_auth_poll;
pub mod agent_apps_auth_read;
mod agent_apps_auth_remove;
pub mod agent_apps_auth_start;

#[cfg(all(test, target_os = "macos"))]
mod host_migration_4_tests;

#[cfg(all(test, target_os = "macos"))]
mod host_migration_3_tests;

#[cfg(all(test, unix))]
mod host_migration_2_tests;

#[cfg(test)]
mod host_migration_13_tests;

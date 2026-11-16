//! Application account ownership and authorized import orchestration.
pub mod auth_accounts;
#[cfg(target_os = "macos")]
pub mod data_import;

pub mod mcp;

pub mod auth_host;
pub mod usage_settings;

pub mod store;
mod sqlite;
mod postgres;
pub mod prompts_store;
#[cfg(target_os = "macos")]
pub mod shared_snapshot;
#[cfg(target_os = "macos")]
pub mod shared_snapshot_import;

#[cfg(test)]
mod store_tests;
#[cfg(test)]
mod sqlite_tests;
#[cfg(test)]
mod postgres_tests;
#[cfg(test)]
mod usage_settings_tests;

pub mod preferences_import;

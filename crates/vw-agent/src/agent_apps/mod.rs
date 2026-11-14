//! Scoped native application storage.
pub mod mcp;
pub mod store;

#[cfg(test)]
mod store_tests;

#[cfg(test)]
mod sqlite_tests;

mod sqlite;

#[cfg(test)]
mod postgres_tests;

mod postgres;

pub mod auth_accounts;
pub mod usage_settings;

#[cfg(test)]
mod usage_settings_tests;

pub mod prompts_store;

#[cfg(target_os = "macos")]
pub mod shared_snapshot;
#[cfg(target_os = "macos")]
pub mod shared_snapshot_import;

pub mod preferences_import;

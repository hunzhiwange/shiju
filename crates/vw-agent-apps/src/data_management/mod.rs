//! Explicitly selected data sources; no domain writes or automatic discovery.
pub mod source;
mod source_files;
mod source_json;

#[cfg(test)]
mod source_tests;

pub mod portable_policy;
#[cfg(test)]
mod portable_policy_tests;

#[cfg(target_os = "macos")]
pub mod safety_snapshot;
#[cfg(target_os = "macos")]
mod safety_snapshot_files;
#[cfg(all(test, target_os = "macos"))]
mod safety_snapshot_tests;

#[cfg(target_os = "macos")]
pub mod backup_create;
#[cfg(target_os = "macos")]
mod backup_create_files;
#[cfg(all(test, target_os = "macos"))]
mod backup_create_tests;

#[cfg(target_os = "macos")]
pub mod backup_list;
#[cfg(all(test, target_os = "macos"))]
mod backup_list_tests;

pub mod sql_stage;
#[cfg(test)]
mod sql_stage_tests;

#[cfg(target_os = "macos")]
pub mod backup_delete;
#[cfg(all(test, target_os = "macos"))]
mod backup_delete_tests;

#[cfg(target_os = "macos")]
pub mod backup_rename;
#[cfg(all(test, target_os = "macos"))]
mod backup_rename_tests;

#[cfg(target_os = "macos")]
pub mod locations;
#[cfg(all(test, target_os = "macos"))]
mod locations_tests;

pub mod preview;
#[cfg(test)]
mod preview_tests;

#[cfg(target_os = "macos")]
pub mod retention;
#[cfg(all(test, target_os = "macos"))]
mod retention_tests;

#[cfg(target_os = "macos")]
pub mod export;
#[cfg(target_os = "macos")]
mod export_files;
#[cfg(all(test, target_os = "macos"))]
mod export_tests;

pub mod shared_snapshot;

#[cfg(target_os = "macos")]
pub mod location_delete;
#[cfg(all(test, target_os = "macos"))]
mod location_delete_tests;

#[cfg(target_os = "macos")]
pub mod backup_api;

pub mod mcp;
#[cfg(test)]
mod mcp_tests;

pub mod providers;
#[cfg(test)]
mod providers_tests;

#[cfg(target_os = "macos")]
pub mod skills;
#[cfg(all(test, target_os = "macos"))]
mod skills_tests;

pub mod profiles;
#[cfg(test)]
mod profiles_tests;

pub mod prompts;
#[cfg(test)]
mod prompts_tests;

pub mod preferences;

pub mod usage;
#[cfg(test)]
mod usage_tests;

//! Host-local backup files, snapshots and portable exports.
#[cfg(target_os = "macos")]
use app_data_format::portable_policy;

#[cfg(target_os = "macos")]
pub mod backup_create;
#[cfg(target_os = "macos")]
mod backup_create_files;
#[cfg(all(test, target_os = "macos"))]
mod backup_create_tests;
#[cfg(target_os = "macos")]
pub mod backup_delete;
#[cfg(all(test, target_os = "macos"))]
mod backup_delete_tests;
#[cfg(target_os = "macos")]
pub mod backup_list;
#[cfg(all(test, target_os = "macos"))]
mod backup_list_tests;
#[cfg(target_os = "macos")]
pub mod backup_rename;
#[cfg(all(test, target_os = "macos"))]
mod backup_rename_tests;
#[cfg(target_os = "macos")]
pub mod export;
#[cfg(target_os = "macos")]
mod export_files;
#[cfg(all(test, target_os = "macos"))]
mod export_tests;
#[cfg(target_os = "macos")]
pub mod location_delete;
#[cfg(all(test, target_os = "macos"))]
mod location_delete_tests;
#[cfg(target_os = "macos")]
pub mod locations;
#[cfg(all(test, target_os = "macos"))]
mod locations_tests;
#[cfg(target_os = "macos")]
pub mod retention;
#[cfg(all(test, target_os = "macos"))]
mod retention_tests;
#[cfg(target_os = "macos")]
pub mod safety_snapshot;
#[cfg(target_os = "macos")]
mod safety_snapshot_files;
#[cfg(all(test, target_os = "macos"))]
mod safety_snapshot_tests;
#[cfg(target_os = "macos")]
pub mod backup_api;

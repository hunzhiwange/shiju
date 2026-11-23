//! Host-local backup and recovery orchestration.
#[cfg(target_os = "macos")]
use app_data_backup::{backup_api, backup_create, backup_delete, backup_list, retention, safety_snapshot};

#[cfg(target_os = "macos")]
pub mod periodic;

pub mod post_import;

#[cfg(target_os = "macos")]
pub mod backup_restore;

#[cfg(all(test, target_os = "macos"))]
mod backup_restore_tests;

#[cfg(target_os = "macos")]
pub mod recovery;
#[cfg(all(test, target_os = "macos"))]
mod recovery_tests;

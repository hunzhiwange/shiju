//! Scoped data import and recovery orchestration.
use app_data_format::source;
#[cfg(test)]
use app_data_format::preview;
#[cfg(test)]
use app_data_projection::usage;
#[cfg(target_os = "macos")]
use app_data_backup::{backup_api, backup_create, backup_delete, backup_list, export, retention, safety_snapshot};

#[cfg(target_os = "macos")]
pub mod periodic;

pub mod apply;
#[cfg(test)]
mod apply_tests;

#[cfg(target_os = "macos")]
pub mod rollback;
#[cfg(all(test, target_os = "macos"))]
mod rollback_tests;

#[cfg(target_os = "macos")]
pub mod operations;

pub mod post_import;

#[cfg(target_os = "macos")]
pub mod backup_restore;

#[cfg(target_os = "macos")]
pub mod package_import;
#[cfg(all(test, target_os = "macos"))]
mod package_import_tests;

#[cfg(all(test, target_os = "macos"))]
mod backup_restore_tests;

#[cfg(target_os = "macos")]
pub mod recovery;
#[cfg(all(test, target_os = "macos"))]
mod recovery_tests;

#[cfg(test)]
mod source_test_support;

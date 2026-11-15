//! Explicitly selected data sources; no domain writes or automatic discovery.
pub use app_data_format::{portable_policy, preview, shared_snapshot, source, sql_stage};

#[cfg(target_os = "macos")]
pub use app_data_backup::{
    backup_api, backup_create, backup_delete, backup_list, backup_rename, export, location_delete,
    locations, retention, safety_snapshot,
};

pub use app_data_projection::providers;

pub use app_data_projection::mcp;

#[cfg(target_os = "macos")]
pub use app_data_projection::skills;

pub use app_data_projection::profiles;

pub use app_data_projection::prompts;

pub use app_data_projection::preferences;

pub use app_data_projection::usage;

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

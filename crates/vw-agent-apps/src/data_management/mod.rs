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

pub use app_data_management::{apply, post_import};
#[cfg(target_os = "macos")]
pub use app_data_management::{backup_restore, operations, package_import, periodic, recovery, rollback};

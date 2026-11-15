//! Compatibility exports for application domain libraries.
//! Implementations and tests live with their domain owners in crates/libs.

pub use app_icons as icons;

pub use app_device_sync as device_sync;

pub use app_operations as apps;

// Provider modes and publication are owned by app-providers.
pub mod providers;

pub mod auth;

pub mod models;

pub mod usage;

pub mod routing;

// Aggregation services are owned by app-providers.
pub mod aggregation;

pub mod mcp;

pub mod high_availability;

pub use app_diagnostics as diagnostics;

pub use app_skills as skills;

#[cfg(target_os = "macos")]
pub mod workspace_paths;

pub mod prompts;

pub mod sessions;

#[cfg(target_os = "macos")]
pub use app_workspace_memory::workspace_files;

#[cfg(target_os = "macos")]
pub use app_workspace_memory::hermes_memory;

#[cfg(target_os = "macos")]
pub use app_workspace_memory::daily_memory_files;

#[cfg(target_os = "macos")]
pub use app_openclaw::openclaw_model;

#[cfg(target_os = "macos")]
pub use app_openclaw_config_rules::catalog as openclaw_catalog;

#[cfg(target_os = "macos")]
pub use app_openclaw::openclaw_defaults;

#[cfg(target_os = "macos")]
pub use app_openclaw::openclaw_env;

#[cfg(target_os = "macos")]
pub use app_openclaw::openclaw_tools;

#[cfg(target_os = "macos")]
pub use app_openclaw::openclaw_health;

#[cfg(target_os = "macos")]
pub use app_workspace_memory::hermes_dashboard;

#[cfg(target_os = "macos")]
pub use app_workspace_memory::daily_memory_list;

#[cfg(target_os = "macos")]
pub use app_workspace_memory::hermes_memory_limits;

#[cfg(target_os = "macos")]
pub use app_workspace_memory::daily_memory_search;

pub use app_project_profiles as project_profiles;

pub mod data_management;

//! Native application business logic for VibeWindow.
//!
//! Domain implementations belong in this crate; storage and runtime adapters
//! remain with callers. The preserved `upstream` snapshot is reference material
//! and is not part of this library's module tree or build.

pub use app_icons as icons;

pub use app_device_sync as device_sync;

pub use app_operations as apps;

// Provider modes and publication are owned by app-providers.
pub mod providers;

pub mod auth;

pub mod models;

pub mod usage;

pub mod routing;

// Catalog rules are owned by app-aggregation-catalog; this module retains service orchestration.
pub mod aggregation;

pub mod mcp;

pub mod high_availability;

pub mod diagnostics;

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

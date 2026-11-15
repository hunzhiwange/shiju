//! Native application business logic for VibeWindow.
//!
//! Domain implementations belong in this crate; storage and runtime adapters
//! remain with callers. The preserved `upstream` snapshot is reference material
//! and is not part of this library's module tree or build.

pub use app_icons as icons;

pub mod device_sync;

pub mod apps;

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

pub mod skills;

#[cfg(target_os = "macos")]
pub mod workspace_paths;

pub mod prompts;

pub mod sessions;

#[cfg(target_os = "macos")]
pub mod workspace_files;
#[cfg(all(test, target_os = "macos"))]
mod workspace_files_tests;

#[cfg(target_os = "macos")]
pub mod hermes_memory;
#[cfg(all(test, target_os = "macos"))]
mod hermes_memory_tests;

#[cfg(target_os = "macos")]
pub mod daily_memory_files;
#[cfg(all(test, target_os = "macos"))]
mod daily_memory_files_tests;

#[cfg(target_os = "macos")]
pub mod openclaw_model;
#[cfg(all(test, target_os = "macos"))]
mod openclaw_model_tests;

#[cfg(target_os = "macos")]
pub use app_openclaw_config_rules::catalog as openclaw_catalog;

#[cfg(target_os = "macos")]
pub mod openclaw_defaults;

#[cfg(target_os = "macos")]
pub mod openclaw_env;
#[cfg(all(test, target_os = "macos"))]
mod openclaw_env_tests;

#[cfg(target_os = "macos")]
pub mod openclaw_tools;
#[cfg(all(test, target_os = "macos"))]
mod openclaw_tools_tests;

#[cfg(target_os = "macos")]
pub mod openclaw_health;
#[cfg(all(test, target_os = "macos"))]
mod openclaw_health_tests;

#[cfg(target_os = "macos")]
pub mod hermes_dashboard;

#[cfg(target_os = "macos")]
pub mod daily_memory_list;
#[cfg(all(test, target_os = "macos"))]
mod daily_memory_list_tests;

#[cfg(target_os = "macos")]
pub mod hermes_memory_limits;
#[cfg(all(test, target_os = "macos"))]
mod hermes_memory_limits_tests;

#[cfg(target_os = "macos")]
pub mod daily_memory_search;
#[cfg(all(test, target_os = "macos"))]
mod daily_memory_search_tests;

pub mod project_profiles;

pub mod data_management;

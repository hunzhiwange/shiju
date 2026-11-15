//! Authorized workspace and native memory file services.

#[cfg(target_os = "macos")]
pub mod daily_memory_files;
#[cfg(all(test, target_os = "macos"))]
mod daily_memory_files_tests;

#[cfg(target_os = "macos")]
pub mod daily_memory_list;
#[cfg(all(test, target_os = "macos"))]
mod daily_memory_list_tests;

#[cfg(target_os = "macos")]
pub mod daily_memory_search;
#[cfg(all(test, target_os = "macos"))]
mod daily_memory_search_tests;

#[cfg(target_os = "macos")]
pub mod hermes_dashboard;

#[cfg(target_os = "macos")]
pub mod hermes_memory;
#[cfg(all(test, target_os = "macos"))]
mod hermes_memory_tests;

#[cfg(target_os = "macos")]
pub mod hermes_memory_limits;
#[cfg(all(test, target_os = "macos"))]
mod hermes_memory_limits_tests;

#[cfg(target_os = "macos")]
pub mod workspace_files;
#[cfg(all(test, target_os = "macos"))]
mod workspace_files_tests;

#[cfg(all(test, target_os = "macos"))]
use app_workspace_paths as workspace_paths;

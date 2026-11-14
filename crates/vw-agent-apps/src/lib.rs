//! Native application business logic for VibeWindow.
//!
//! Domain implementations belong in this crate; storage and runtime adapters
//! remain with callers. The preserved `upstream` snapshot is reference material
//! and is not part of this library's module tree or build.

pub mod icons;

pub mod apps;

pub mod providers;

pub mod auth;

pub mod models;

pub mod usage;

pub mod routing;

pub mod aggregation;

pub mod mcp;

pub mod high_availability;

pub mod diagnostics;

pub mod prompts;

#[cfg(target_os = "macos")]
pub mod workspace_paths;
#[cfg(all(test, target_os = "macos"))]
mod workspace_paths_tests;

#[cfg(target_os = "macos")]
pub mod hermes_memory;
#[cfg(all(test, target_os = "macos"))]
mod hermes_memory_tests;

#[cfg(target_os = "macos")]
pub mod workspace_files;
#[cfg(all(test, target_os = "macos"))]
mod workspace_files_tests;

#[cfg(target_os = "macos")]
pub mod daily_memory_files;
#[cfg(all(test, target_os = "macos"))]
mod daily_memory_files_tests;

#[cfg(target_os = "macos")]
pub mod openclaw_catalog;
#[cfg(all(test, target_os = "macos"))]
mod openclaw_catalog_tests;

#[cfg(target_os = "macos")]
pub mod openclaw_model;
#[cfg(all(test, target_os = "macos"))]
mod openclaw_model_tests;

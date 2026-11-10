//! Business rules for the managed MCP catalog.

pub mod catalog;
pub mod presets;
pub mod validation;

#[cfg(test)]
mod presets_tests;
#[cfg(test)]
mod validation_tests;

#[cfg(target_os = "macos")]
pub mod native_file;
#[cfg(all(test, target_os = "macos"))]
mod native_file_tests;

#[cfg(target_os = "macos")]
pub mod adapters;

//! Usage script template selection, host-side preparation, and bounded execution.

pub mod config;
#[cfg(target_os = "macos")]
pub mod runtime;

#[cfg(test)]
mod config_tests;

#[cfg(all(test, target_os = "macos"))]
mod runtime_tests;

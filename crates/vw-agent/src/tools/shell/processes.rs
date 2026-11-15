//! Compatibility exports for the unique tool-shell implementation.
pub use tool_shell::shell::processes::*;

#[cfg(test)]
use std::{time::Duration, path::Path};
#[cfg(all(test, windows))]
use std::path::PathBuf;
#[cfg(test)]
#[path = "processes_tests.rs"]
mod tests;
#[cfg(all(test, unix))]
#[path = "termination_tests.rs"]
mod termination_tests;
#[cfg(all(test, windows))]
#[path = "termination_windows_tests.rs"]
mod termination_windows_tests;

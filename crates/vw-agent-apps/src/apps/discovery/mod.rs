//! Execution-host installation path discovery.
#[cfg(unix)]
pub mod posix;
#[cfg(all(test, unix))]
mod posix_tests;
pub mod windows;
#[cfg(test)]
mod windows_tests;

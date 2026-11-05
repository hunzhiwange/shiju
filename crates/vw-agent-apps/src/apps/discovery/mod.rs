//! Execution-host installation path discovery.
#[cfg(unix)]
pub mod posix;
#[cfg(all(test, unix))]
mod posix_tests;
pub mod windows;
#[cfg(test)]
mod windows_tests;
pub mod wsl;
#[cfg(test)]
mod wsl_tests;

pub mod claude_desktop;
#[cfg(all(test, unix))]
mod claude_desktop_tests;

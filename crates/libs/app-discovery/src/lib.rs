//! Execution-host installation path discovery.
#[cfg(unix)]
pub mod posix;
#[cfg(all(test, unix))]
mod posix_tests;
pub mod windows;
#[cfg(test)]
mod windows_tests;
pub use app_catalog::wsl;

pub use app_providers::desktop_presence as claude_desktop;

pub mod report;
#[cfg(test)]
mod report_tests;

#[cfg(unix)]
pub mod version;
#[cfg(all(test, unix))]
mod version_tests;

#[cfg(unix)]
pub mod read;

pub mod local_agy;
pub mod local_omp;
pub mod native;
#[cfg(all(test, unix))]
mod node_probe_tests;

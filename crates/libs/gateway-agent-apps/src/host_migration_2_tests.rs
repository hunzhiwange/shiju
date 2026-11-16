//! Domain-owned application route and authorization contracts.
use crate::{access, lifecycle, read, preferences, launch};
#[cfg(unix)]
mod launch_tests;
#[cfg(unix)]
mod lifecycle_tests;
#[cfg(unix)]
mod preferences_tests;
#[cfg(unix)]
mod remote_tests_tests;
#[cfg(target_os = "macos")]
mod import_tests;
#[cfg(target_os = "macos")]
mod recovery_tests;
#[cfg(unix)]
mod read_tests;
#[cfg(unix)]
mod local_access_tests;

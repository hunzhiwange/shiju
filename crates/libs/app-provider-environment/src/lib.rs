//! Host-bound provider environment diagnostics, cleanup and restoration.

pub mod env_diagnose;
pub mod env_clean;
pub mod env_restore;

#[cfg(target_os = "macos")]
mod env_backup;

#[cfg(all(test, target_os = "macos"))]
mod env_diagnose_tests;
#[cfg(all(test, target_os = "macos"))]
mod env_clean_tests;
#[cfg(all(test, target_os = "macos"))]
mod env_restore_tests;

//! Executor-owned configuration staging and descriptor-relative writes.

#[cfg(target_os = "macos")]
pub mod write_guard;
#[cfg(target_os = "macos")]
pub mod stage;
#[cfg(target_os = "macos")]
pub mod concurrency;

#[cfg(all(test, target_os = "macos"))]
mod write_guard_tests;
#[cfg(all(test, target_os = "macos"))]
mod stage_tests;

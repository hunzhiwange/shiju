//! Executor-owned configuration staging and descriptor-relative writes.

/// Fixed errors deliberately exclude credentials, input bytes and host paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// A prior transaction was recovered; reload state before retrying.
    Recovered,
    /// Concurrent edits exhausted replanning or invalidated a guarded snapshot.
    Conflict,
    InvalidResource,
    Io,
}

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

//! Executor-only history maintenance primitives, never client-path APIs.
#[cfg(target_os = "macos")]
pub mod rollout;
#[cfg(target_os = "macos")]
pub mod state_db;

#[cfg(target_os = "macos")]
pub mod provider_refs;

#[cfg(target_os = "macos")]
pub mod unify;

#[cfg(target_os = "macos")]
pub mod restore;

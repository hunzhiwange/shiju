//! Executor-owned project profile services.
pub use gateway_project::{profile_store, profile_operations};
#[cfg(target_os = "macos")]
pub mod profile_skills;

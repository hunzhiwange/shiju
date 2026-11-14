//! Executor-owned project profile persistence.
pub(crate) mod profile_store;

pub(crate) mod profile_operations;
#[cfg(target_os = "macos")]
pub(crate) mod profile_skills;

//! Native prompt targets; persistence and file operations belong to their owners.
pub mod targets;

pub mod template_slug;

pub mod template_metadata;

pub mod shared_import;

#[cfg(target_os = "macos")]
pub mod live_file;

pub mod mcode;

pub mod pi_agents;

#[cfg(target_os = "macos")]
pub mod pi_system_files;

pub mod omp;

pub mod copy;

pub mod import;

pub mod listing;

#[cfg(target_os = "macos")]
pub mod activation;

#[cfg(target_os = "macos")]
pub mod templates;

#[cfg(target_os = "macos")]
pub mod deactivation;

#[cfg(target_os = "macos")]
pub mod template_rename;

pub mod sync;

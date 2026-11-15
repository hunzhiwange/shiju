//! Managed Git worktree lifecycle and maintenance.
#[cfg(not(target_arch = "wasm32"))]
use agent_project as project;
#[cfg(not(target_arch = "wasm32"))]
use agent_storage as storage;
#[cfg(not(target_arch = "wasm32"))]
use agent_paths as global;
use agent_bus as bus;
#[cfg(not(target_arch = "wasm32"))]
use agent_database as global_database;
#[cfg(not(target_arch = "wasm32"))]
use agent_config::runtime as config;
#[cfg(not(target_arch = "wasm32"))]
use shell_command as shell;
#[cfg(not(target_arch = "wasm32"))]
use git_discovery::git_repository;
#[cfg(not(target_arch = "wasm32"))]
pub mod host;
mod worktree;
pub use worktree::{CreateInput, Error, Info, RemoveInput, ResetInput, create, remove, reset, list_directories, event};
#[cfg(not(target_arch = "wasm32"))]
pub use worktree::{registry, environment, recovery, cleanup, management, cleanup_host, create_fork_checkout, remove_fork_checkout, retry_environment};

/// Existing host integration tests require access to lifecycle preflight helpers.
#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
#[doc(hidden)]
pub mod testing {
    pub use crate::worktree::{native, ops, creation_policy};
}

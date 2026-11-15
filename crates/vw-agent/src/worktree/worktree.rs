//! Application composition for the extracted worktree service.
pub use worktree_core::*;
#[cfg(not(target_arch = "wasm32"))]
pub use super::host_adapter::initialize_host;
#[cfg(target_arch = "wasm32")]
fn initialize_host() {}

#[cfg(not(target_arch = "wasm32"))]
pub async fn create(input: Option<CreateInput>) -> Result<Info, Error> {
    initialize_host();
    worktree_core::create(input).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn create_fork_checkout(name: Option<&str>) -> Result<Info, Error> {
    initialize_host();
    worktree_core::create_fork_checkout(name).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn remove_fork_checkout(directory: String) -> Result<(), Error> {
    initialize_host();
    worktree_core::remove_fork_checkout(directory).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn retry_environment(directory: String) -> Result<(), Error> {
    initialize_host();
    worktree_core::retry_environment(directory).await
}
#[cfg(all(test, not(target_arch = "wasm32")))]
use worktree_core::testing::{native, ops, creation_policy};

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "worktree_native_tests.rs"]
mod worktree_native_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "worktree_ops_tests.rs"]
mod worktree_ops_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "cleanup_flow_tests.rs"]
mod cleanup_flow_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "cleanup_tests.rs"]
mod cleanup_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "environment_tests.rs"]
mod environment_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "recovery_restore_tests.rs"]
mod recovery_restore_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "management_e2e_tests.rs"]
mod management_e2e_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "creation_policy_tests.rs"]
mod creation_policy_tests;

#[cfg(not(target_arch = "wasm32"))]
pub async fn remove(input: RemoveInput) -> Result<bool, Error> {
    initialize_host();
    worktree_core::remove(input).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn reset(input: ResetInput) -> Result<bool, Error> {
    initialize_host();
    worktree_core::reset(input).await
}

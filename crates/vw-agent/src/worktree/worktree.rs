//! Git Worktree 管理模块
//!
//! 本模块按职责拆分为类型、事件、命名规则、原生实现细节和公开操作，
//! 保持 `worktree::...` 外部接口不变。

#[path = "worktree_event.rs"]
pub mod event;

#[path = "worktree_naming.rs"]
mod naming;
#[cfg(not(target_arch = "wasm32"))]
#[path = "worktree_native.rs"]
mod native;
#[path = "worktree_ops.rs"]
mod ops;
#[path = "worktree_types.rs"]
mod types;

#[cfg(not(target_arch = "wasm32"))]
pub use ops::retry_environment;
pub use ops::{create, list_directories, remove, reset};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use ops::{create_fork_checkout, remove_fork_checkout};
pub use types::{CreateInput, Error, Info, RemoveInput, ResetInput};

#[cfg(test)]
#[path = "worktree_event_tests.rs"]
mod worktree_event_tests;
#[cfg(test)]
#[path = "worktree_naming_tests.rs"]
mod worktree_naming_tests;
#[cfg(test)]
#[path = "worktree_native_tests.rs"]
mod worktree_native_tests;
#[cfg(test)]
#[path = "worktree_ops_tests.rs"]
mod worktree_ops_tests;
#[cfg(test)]
#[path = "worktree_types_tests.rs"]
mod worktree_types_tests;

#[cfg(not(target_arch = "wasm32"))]
#[path = "registry.rs"]
pub mod registry;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "registry_tests.rs"]
mod registry_tests;

#[cfg(not(target_arch = "wasm32"))]
#[path = "creation_policy.rs"]
mod creation_policy;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "creation_policy_tests.rs"]
mod creation_policy_tests;

#[cfg(not(target_arch = "wasm32"))]
#[path = "environment.rs"]
pub mod environment;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "environment_tests.rs"]
mod environment_tests;

#[cfg(not(target_arch = "wasm32"))]
#[path = "recovery.rs"]
pub mod recovery;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "recovery_tests.rs"]
mod recovery_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "recovery_restore_tests.rs"]
mod recovery_restore_tests;

#[cfg(not(target_arch = "wasm32"))]
#[path = "cleanup.rs"]
pub mod cleanup;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "cleanup_tests.rs"]
mod cleanup_tests;

#[cfg(not(target_arch = "wasm32"))]
#[path = "management.rs"]
pub mod management;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "cleanup_flow_tests.rs"]
mod cleanup_flow_tests;

#[cfg(not(target_arch = "wasm32"))]
#[path = "cleanup_host.rs"]
pub mod cleanup_host;

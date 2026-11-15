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
pub mod native;
#[path = "worktree_ops.rs"]
pub mod ops;
#[path = "worktree_types.rs"]
mod types;

#[cfg(not(target_arch = "wasm32"))]
pub use ops::retry_environment;
pub use ops::{create, list_directories, remove, reset};
#[cfg(not(target_arch = "wasm32"))]
pub use ops::{create_fork_checkout, remove_fork_checkout};
pub use types::{CreateInput, Error, Info, RemoveInput, ResetInput};

#[cfg(not(target_arch = "wasm32"))]
#[path = "registry.rs"]
pub mod registry;

#[cfg(not(target_arch = "wasm32"))]
#[path = "creation_policy.rs"]
pub mod creation_policy;

#[cfg(not(target_arch = "wasm32"))]
#[path = "environment.rs"]
pub mod environment;

#[cfg(not(target_arch = "wasm32"))]
#[path = "recovery.rs"]
pub mod recovery;

#[cfg(not(target_arch = "wasm32"))]
#[path = "cleanup.rs"]
pub mod cleanup;

#[cfg(not(target_arch = "wasm32"))]
#[path = "management.rs"]
pub mod management;

#[cfg(not(target_arch = "wasm32"))]
#[path = "cleanup_host.rs"]
pub mod cleanup_host;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "recovery_tests.rs"]
mod recovery_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "registry_tests.rs"]
mod registry_tests;

#[cfg(test)]
#[path = "worktree_event_tests.rs"]
mod worktree_event_tests;

#[cfg(test)]
#[path = "worktree_naming_tests.rs"]
mod worktree_naming_tests;

#[cfg(test)]
#[path = "worktree_types_tests.rs"]
mod worktree_types_tests;

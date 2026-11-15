//! 工具执行上下文的共享状态与执行契约。

pub mod decision;
pub mod executor;
mod pending_images;

pub use decision::PermissionDecision;
pub use executor::{ExecutedToolCall, ToolCallError, ToolResultHistoryEntry};
pub use pending_images::{MAX_PENDING_IMAGE_RELOADS, PendingImageReloadQueue};

pub mod context;
pub mod host;
pub mod read_state;

#[cfg(not(target_arch = "wasm32"))]
pub mod run_worktree;

pub mod hooks;

pub mod approval;
